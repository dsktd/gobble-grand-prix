//! Phone controller: an HTTPS + WebSocket server on the LAN, a QR code to join, and a bridge
//! that turns phone messages into kart and menu input.
//!
//! The server runs on its own thread with a small tokio runtime. Phones talk JSON over a
//! WebSocket; the game answers with acks (for latency) and buzzes (for haptics).

use std::net::IpAddr;
use std::net::SocketAddr;
use std::sync::mpsc;
use std::time::Duration;
use std::time::Instant;

use axum::Router;
use axum::extract::Query as UrlQuery;
use axum::extract::State as AxumState;
use axum::extract::ws::Message as WsMessage;
use axum::extract::ws::WebSocket;
use axum::extract::ws::WebSocketUpgrade;
use axum::http::StatusCode;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::routing::get;
use axum_server::tls_rustls::RustlsConfig;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::TextureDimension;
use bevy::render::render_resource::TextureFormat;
use futures_util::SinkExt;
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::broadcast;

use crate::prelude::*;

pub(super) mod prelude {
    pub use super::JoinInfo;
    pub use super::Phones;
}

pub const PORT: u16 = 7777;
/// How long a silent phone keeps its kart before the CPU takes over.
const TAKEOVER: Duration = Duration::from_secs(3);
/// How long a disconnected phone keeps its seat.
const REJOIN: Duration = Duration::from_secs(30);
const PAGE: &str = include_str!("../assets/phone/index.html");

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Phones>();
    app.add_systems(Startup, start);
    app.add_systems(PreUpdate, receive.after(menu_input));
    app.add_systems(Update, (buzz, takeover).run_if(in_state(GameState::Race)));
    app.add_systems(Update, (tell_seats, drop_seats));
}

/// What the title screen shows so a phone can join.
#[derive(Resource, Clone, Debug)]
pub struct JoinInfo {
    pub url: String,
    pub pin: String,
    pub qr: Handle<Image>,
}

/// Latest state of a phone's controls.
#[derive(Clone, Debug, Default)]
pub struct Pad {
    pub steer: f32,
    pub gas: bool,
    pub brake: bool,
    pub drift: bool,
    /// Item taps not yet used by the kart.
    pub item: bool,
    pub seq: u64,
    acked: u64,
    /// Phone clock of the last input, echoed back for latency.
    pub stamp: f64,
    pub seen: Option<Instant>,
    /// Menu buttons pressed this frame, for seat joining and per-seat choices.
    pub menu: MenuInput,
    /// When the socket closed; the seat is kept for [`REJOIN`].
    pub gone: Option<Instant>,
}

/// Connected phones and the channel back to them.
#[derive(Resource, Default)]
pub struct Phones {
    pub pads: Vec<(u32, Pad)>,
    /// The last input for P1 came from a phone rather than a pad or keyboard.
    pub driving: bool,
    rx: Option<std::sync::Mutex<mpsc::Receiver<Inbound>>>,
    tx: Option<broadcast::Sender<String>>,
}
impl Phones {
    pub fn send(
        &self,
        message: String,
    ) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(message);
        }
    }

    /// Menu presses from one phone this frame.
    pub fn menu(
        &self,
        id: u32,
    ) -> MenuInput {
        self.pads
            .iter()
            .find(|(p, _)| *p == id)
            .map_or_else(MenuInput::default, |(_, pad)| pad.menu)
    }

    /// Fold the chosen live phones into one kart input, consuming item taps.
    pub fn take_input(
        &mut self,
        pick: impl Fn(u32) -> bool,
    ) -> Option<KartInput> {
        let mut merged: Option<KartInput> = None;
        for (id, pad) in &mut self.pads {
            if !pick(*id) || pad.seen.is_none_or(|seen| seen.elapsed() > TAKEOVER) {
                continue;
            }
            let input = KartInput {
                throttle: i32::from(pad.gas) as f32 - i32::from(pad.brake) as f32,
                steer: pad.steer,
                drift: pad.drift,
                item: std::mem::take(&mut pad.item),
            };
            if let (Some(tx), true) = (&self.tx, pad.seq != pad.acked) {
                pad.acked = pad.seq;
                let _ = tx.send(format!(
                    r#"{{"t":"ack","id":{id},"s":{},"ms":{}}}"#,
                    pad.seq, pad.stamp
                ));
            }
            merged = Some(match merged {
                Some(m) => KartInput {
                    throttle: if m.throttle.abs() >= input.throttle.abs() {
                        m.throttle
                    } else {
                        input.throttle
                    },
                    steer: if m.steer.abs() >= input.steer.abs() {
                        m.steer
                    } else {
                        input.steer
                    },
                    drift: m.drift || input.drift,
                    item: m.item || input.item,
                },
                None => input,
            });
        }
        merged
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "t")]
enum Wire {
    #[serde(rename = "in")]
    Input {
        s: u64,
        ms: f64,
        steer: f32,
        gas: bool,
        brake: bool,
        drift: bool,
        item: bool,
    },
    #[serde(rename = "menu")]
    Menu { k: String },
}

enum Inbound {
    Joined(u32),
    Left(u32),
    Wire(u32, Wire),
}

#[derive(Clone)]
struct Shared {
    pin: String,
    tx: mpsc::Sender<Inbound>,
    out: broadcast::Sender<String>,
    next: std::sync::Arc<std::sync::atomic::AtomicU32>,
    /// Phone token to connection id, so a reload gets its old seat back.
    ids: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, u32>>>,
}

/// The address phones reach us on: the interface of the default route. Nothing is sent.
fn lan_ip() -> Option<IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    if ip.is_loopback() || ip.is_unspecified() {
        None
    } else {
        Some(ip)
    }
}

/// A self-signed certificate for this IP, made once and kept beside the save data.
fn certificate(ip: IpAddr) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let dir = dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("gobble-grand-prix");
    let cert_path = dir.join(format!("cert-{ip}.pem"));
    let key_path = dir.join(format!("key-{ip}.pem"));
    if let (Ok(cert), Ok(key)) = (std::fs::read(&cert_path), std::fs::read(&key_path)) {
        return Ok((cert, key));
    }
    std::fs::create_dir_all(&dir)?;
    let made =
        rcgen::generate_simple_self_signed(vec![ip.to_string(), "gobble.local".to_string()])?;
    let cert = made.cert.pem().into_bytes();
    let key = made.signing_key.serialize_pem().into_bytes();
    std::fs::write(&cert_path, &cert)?;
    std::fs::write(&key_path, &key)?;
    Ok((cert, key))
}

fn qr_image(url: &str) -> Option<Image> {
    let code = qrcode::QrCode::new(url).ok()?;
    let width = code.width();
    let quiet = 2;
    let side = width + quiet * 2;
    let mut data = vec![255u8; side * side * 4];
    for (i, color) in code.to_colors().iter().enumerate() {
        if *color == qrcode::Color::Dark {
            let (x, y) = (i % width + quiet, i / width + quiet);
            let at = (y * side + x) * 4;
            data[at..at + 3].copy_from_slice(&[0, 0, 0]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: side as u32,
            height: side as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = bevy::image::ImageSampler::nearest();
    Some(image)
}

fn start(
    mut commands: Commands,
    mut phones: ResMut<Phones>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(ip) = lan_ip() else {
        warn!("No LAN address, phone controller off.");
        return;
    };
    let (cert, key) = match certificate(ip) {
        Ok(pair) => pair,
        Err(err) => {
            error!("Phone controller certificate: {err}");
            return;
        }
    };
    let pin = format!("{:04}", rand::random_range(0..10_000));
    let url = format!("https://{ip}:{PORT}/?pin={pin}");
    info!("Phone join {url}");
    let (tx, rx) = mpsc::channel();
    let (out, _) = broadcast::channel(256);
    let shared = Shared {
        pin: pin.clone(),
        tx,
        out: out.clone(),
        next: std::sync::Arc::default(),
        ids: std::sync::Arc::default(),
    };
    let addr = SocketAddr::new(ip, PORT);
    let _handle = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let config = match RustlsConfig::from_pem(cert, key).await {
                Ok(config) => config,
                Err(err) => {
                    error!("Phone controller TLS: {err}");
                    return;
                }
            };
            let app = Router::new()
                .route("/", get(page))
                .route("/ws", get(socket))
                .with_state(shared);
            info!("Phone controller on https://{addr}");
            if let Err(err) = axum_server::bind_rustls(addr, config)
                .serve(app.into_make_service())
                .await
            {
                error!("Phone controller stopped: {err}");
            }
        });
    });

    phones.rx = Some(std::sync::Mutex::new(rx));
    phones.tx = Some(out);
    if let Some(image) = qr_image(&url) {
        commands.insert_resource(JoinInfo {
            url,
            pin,
            qr: images.add(image),
        });
    }
}

async fn page() -> Html<&'static str> {
    Html(PAGE)
}

#[derive(Deserialize)]
struct Join {
    pin: String,
    /// Random id the page keeps in localStorage.
    #[serde(default)]
    token: String,
}

async fn socket(
    upgrade: WebSocketUpgrade,
    UrlQuery(join): UrlQuery<Join>,
    AxumState(shared): AxumState<Shared>,
) -> impl IntoResponse {
    if join.pin != shared.pin {
        return (StatusCode::FORBIDDEN, "Wrong PIN").into_response();
    }
    let id = {
        let mut ids = shared.ids.lock().unwrap();
        let fresh = || {
            shared
                .next
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                + 1
        };
        if join.token.is_empty() {
            fresh()
        } else {
            *ids.entry(join.token.chars().take(64).collect())
                .or_insert_with(fresh)
        }
    };
    upgrade.on_upgrade(move |ws| session(ws, shared, id))
}

async fn session(
    ws: WebSocket,
    shared: Shared,
    id: u32,
) {
    let _ = shared.tx.send(Inbound::Joined(id));
    let (mut sink, mut stream) = ws.split();
    let mut out = shared.out.subscribe();
    let _ = sink
        .send(WsMessage::Text(
            format!(r#"{{"t":"hello","id":{id}}}"#).into(),
        ))
        .await;
    loop {
        tokio::select! {
            incoming = stream.next() => {
                let Some(Ok(message)) = incoming else {
                    break;
                };
                let WsMessage::Text(text) = message else {
                    continue;
                };
                let Ok(wire) = serde_json::from_str::<Wire>(&text) else {
                    continue;
                };
                if shared.tx.send(Inbound::Wire(id, wire)).is_err() {
                    break;
                }
            }
            outgoing = out.recv() => {
                let Ok(text) = outgoing else {
                    continue;
                };
                // Messages carry an id when meant for one phone
                let mine = !text.contains(r#""id":"#) || text.contains(&format!(r#""id":{id},"#));
                if mine && sink.send(WsMessage::Text(text.into())).await.is_err() {
                    break;
                }
            }
        }
    }
    let _ = shared.tx.send(Inbound::Left(id));
}

fn receive(
    mut phones: ResMut<Phones>,
    mut menu: ResMut<MenuInput>,
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
) {
    // Any key or button hands P1 back to the local devices
    let local = keys.get_just_pressed().next().is_some()
        || gamepads
            .iter()
            .any(|g| g.get_just_pressed().next().is_some());
    if local {
        phones.driving = false;
    }
    let Some(rx) = phones.rx.as_ref() else {
        return;
    };
    let inbound: Vec<Inbound> = rx.lock().unwrap().try_iter().collect();
    for (_, pad) in &mut phones.pads {
        pad.menu = MenuInput::default();
    }
    // Forget phones that stayed away too long
    if phones
        .pads
        .iter()
        .any(|(_, pad)| pad.gone.is_some_and(|t| t.elapsed() > REJOIN))
    {
        phones
            .pads
            .retain(|(_, pad)| pad.gone.is_none_or(|t| t.elapsed() <= REJOIN));
    }
    for message in inbound {
        match message {
            Inbound::Joined(id) => {
                info!("Phone {id} connected");
                match phones.pads.iter_mut().find(|(p, _)| *p == id) {
                    Some((_, pad)) => pad.gone = None,
                    None => phones.pads.push((id, Pad::default())),
                }
            }
            Inbound::Left(id) => {
                info!("Phone {id} disconnected");
                if let Some((_, pad)) = phones.pads.iter_mut().find(|(p, _)| *p == id) {
                    pad.gone = Some(Instant::now());
                }
            }
            Inbound::Wire(
                id,
                Wire::Input {
                    s,
                    ms,
                    steer,
                    gas,
                    brake,
                    drift,
                    item,
                },
            ) => {
                phones.driving = true;
                let Some((_, pad)) = phones.pads.iter_mut().find(|(p, _)| *p == id) else {
                    continue;
                };
                pad.steer = steer.clamp(-1.0, 1.0);
                pad.gas = gas;
                pad.brake = brake;
                pad.drift = drift;
                pad.item |= item;
                pad.seq = s;
                pad.stamp = ms;
                pad.seen = Some(Instant::now());
            }
            Inbound::Wire(id, Wire::Menu { k }) => {
                let mut press = MenuInput::default();
                match k.as_str() {
                    "up" => press.up = true,
                    "down" => press.down = true,
                    "left" => press.left = true,
                    "right" => press.right = true,
                    "ok" => press.confirm = true,
                    "back" => press.back = true,
                    "pause" => press.pause = true,
                    _ => {}
                }
                // Into the shared menu input, and the phone's own for its seat
                menu.up |= press.up;
                menu.down |= press.down;
                menu.left |= press.left;
                menu.right |= press.right;
                menu.confirm |= press.confirm;
                menu.back |= press.back;
                menu.pause |= press.pause;
                if let Some((_, pad)) = phones.pads.iter_mut().find(|(p, _)| *p == id) {
                    pad.menu.up |= press.up;
                    pad.menu.down |= press.down;
                    pad.menu.left |= press.left;
                    pad.menu.right |= press.right;
                    pad.menu.confirm |= press.confirm;
                    pad.menu.back |= press.back;
                    pad.menu.pause |= press.pause;
                }
            }
        }
    }
}

/// Tell each phone which seat it holds, so its page can show "P2".
fn tell_seats(
    phones: Res<Phones>,
    seats: Res<Seats>,
) {
    if !seats.is_changed() && !phones.is_changed() {
        return;
    }
    for (id, _) in &phones.pads {
        let message = seats
            .0
            .iter()
            .position(|d| *d == Device::Phone(*id))
            .map_or_else(
                || format!(r#"{{"t":"seat","id":{id},"p":null}}"#),
                |slot| {
                    let [r, g, b, _] = seat_color(slot).to_srgba().to_u8_array();
                    format!(
                        r##"{{"t":"seat","id":{id},"p":{},"color":"#{r:02x}{g:02x}{b:02x}"}}"##,
                        slot + 1
                    )
                },
            );
        phones.send(message);
    }
}

/// A phone that disconnects in the lobby gives its seat back.
fn drop_seats(
    phones: Res<Phones>,
    state: Res<State<GameState>>,
    mut seats: ResMut<Seats>,
) {
    if *state.get() == GameState::Race || !phones.is_changed() {
        return;
    }
    let gone =
        |d: &Device| matches!(d, Device::Phone(id) if !phones.pads.iter().any(|(p, _)| p == id));
    if seats.0.iter().any(gone) {
        seats.0.retain(|d| !gone(d));
    }
}

/// Buzz phones when P1 is hit or boosts.
fn buzz(
    phones: Res<Phones>,
    mut sfx: MessageReader<Sfx>,
) {
    for sound in sfx.read() {
        let ms = match sound {
            Sfx::Hit => 120,
            Sfx::Boost | Sfx::Turbo => 40,
            Sfx::Fanfare => 300,
            _ => continue,
        };
        phones.send(format!(r#"{{"t":"buzz","ms":{ms}}}"#));
    }
}

/// Marks a kart the CPU drives because its phone went quiet.
#[derive(Component, Copy, Clone, Debug)]
struct Takeover;

fn takeover(
    mut commands: Commands,
    phones: Res<Phones>,
    seats: Res<Seats>,
    karts: Query<(Entity, &Human, Has<Takeover>, Has<Finished>)>,
) {
    let live = |pick: &dyn Fn(u32) -> bool| {
        phones
            .pads
            .iter()
            .any(|(id, pad)| pick(*id) && pad.seen.is_some_and(|seen| seen.elapsed() < TAKEOVER))
    };
    for (id, human, taken, finished) in karts {
        if finished {
            continue;
        }
        // A phone seat, or P1 while an unseated phone was driving it
        let lost = match seats.0.get(human.0) {
            Some(Device::Phone(phone)) => !live(&|p| p == *phone),
            _ => false,
        };
        if lost && !taken {
            info!("Phone quiet, CPU takes P{}", human.0 + 1);
            commands
                .entity(id)
                .insert((Takeover, Driver::Cpu, Cpu::new(0.95)));
        } else if !lost && taken {
            commands
                .entity(id)
                .remove::<(Takeover, Cpu)>()
                .insert(Driver::Local(human.0));
        }
    }
}
