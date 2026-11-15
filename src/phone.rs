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
use axum::extract::ConnectInfo;
use axum::extract::Query as UrlQuery;
use axum::extract::State as AxumState;
use axum::extract::ws::Message as WsMessage;
use axum::extract::ws::WebSocket;
use axum::extract::ws::WebSocketUpgrade;
use axum::http::StatusCode;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::routing::get;

use crate::phone_guard::MAX_MESSAGE;
use crate::phone_guard::PinGuard;
use crate::phone_guard::Rate;
use crate::phone_guard::Seats as TokenSeats;
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

macro_rules! glyph {
    ($file:literal) => {
        (
            concat!("/glyphs/", $file),
            "image/png",
            include_bytes!(concat!("../assets/prompts/steamdeck/", $file)),
        )
    };
}

pub const PORT: u16 = 7777;
/// Ports tried after `PORT` when it is taken, before asking the OS for any free one.
const FALLBACKS: u16 = 10;
/// How long a silent phone keeps its kart before the CPU takes over.
const TAKEOVER: Duration = Duration::from_secs(3);
/// How long a disconnected phone keeps its seat.
const REJOIN: Duration = Duration::from_secs(30);
/// iroh application protocol for phone controllers.
const ALPN: &[u8] = b"gobble/controller/1";
/// The phone page on GitHub Pages; the game id and PIN ride in the fragment so no server logs them.
const PAGES: &str = "https://dsktd.github.io/gobble-grand-prix/pad/";
const PAGE: &str = include_str!("../assets/phone/index.html");
/// Files the page loads next to itself: its font and the Kenney button glyphs.
const FILES: &[(&str, &str, &[u8])] = &[
    (
        "/fonts/Fredoka-500.ttf",
        "font/ttf",
        include_bytes!("../assets/fonts/Fredoka-500.ttf"),
    ),
    (
        "/fonts/Fredoka-600.ttf",
        "font/ttf",
        include_bytes!("../assets/fonts/Fredoka-600.ttf"),
    ),
    (
        "/fonts/Fredoka-700.ttf",
        "font/ttf",
        include_bytes!("../assets/fonts/Fredoka-700.ttf"),
    ),
    glyph!("steamdeck_button_a.png"),
    glyph!("steamdeck_button_b.png"),
    glyph!("steamdeck_button_l2.png"),
    glyph!("steamdeck_button_r1.png"),
    glyph!("steamdeck_button_r2.png"),
    glyph!("steamdeck_button_options.png"),
    glyph!("steamdeck_dpad.png"),
];

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Phones>();
    app.add_systems(Startup, start);
    app.add_systems(PreUpdate, receive.after(menu_input));
    app.add_systems(Update, (buzz, takeover).run_if(in_state(GameState::Race)));
    app.add_systems(Update, (tell_seats, drop_seats, anywhere));
}

/// What the title screen shows so a phone can join.
#[derive(Resource, Clone, Debug)]
pub struct JoinInfo {
    /// Same Wi-Fi only.
    pub url: String,
    /// Through iroh relays, from any network; the QR shows this one when it is up.
    pub anywhere: Option<String>,
    pub pin: String,
    pub qr: Handle<Image>,
}

/// Waits for the iroh endpoint id from the server thread.
#[derive(Resource)]
struct Anywhere {
    rx: std::sync::Mutex<mpsc::Receiver<String>>,
    pin: String,
    /// The game's own HTTPS page, for phones on the same Wi-Fi when iroh fails.
    lan: SocketAddr,
}

/// Once iroh is up, point the QR code at the anywhere link.
fn anywhere(
    mut commands: Commands,
    waiting: Option<Res<Anywhere>>,
    join: Option<ResMut<JoinInfo>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(waiting), Some(mut join)) = (waiting, join) else {
        return;
    };
    let Ok(id) = waiting.rx.lock().unwrap().try_recv() else {
        return;
    };
    let link = format!("{PAGES}#g={id}&pin={}&lan={}", waiting.pin, waiting.lan);
    info!("Phone join anywhere {link}");
    if let Some(image) = qr_image(&link) {
        join.qr = images.add(image);
    }
    join.anywhere = Some(link);
    commands.remove_resource::<Anywhere>();
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
    /// A racer the phone tapped on its list this frame.
    pub pick: Option<usize>,
    /// When the socket closed; the seat is kept for [`REJOIN`].
    pub gone: Option<Instant>,
    /// The name the phone page sent, already sanitised.
    pub name: Option<String>,
    /// Whether the phone page knows its seat, and which seat that was.
    told: bool,
    told_seat: Option<usize>,
    told_name: Option<String>,
    /// Whether the phone page knows the race is on (drive controls) or not (menu keys).
    told_race: Option<bool>,
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

    /// The name a phone chose, if any.
    pub fn name(
        &self,
        id: u32,
    ) -> Option<String> {
        self.pads
            .iter()
            .find(|(p, _)| *p == id)
            .and_then(|(_, pad)| pad.name.clone())
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

    /// The racer one phone tapped this frame.
    pub fn pick(
        &self,
        id: u32,
    ) -> Option<usize> {
        self.pads
            .iter()
            .find(|(p, _)| *p == id)
            .and_then(|(_, pad)| pad.pick)
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
    Joined(u32, Option<String>),
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
    seats: std::sync::Arc<std::sync::Mutex<TokenSeats>>,
    /// Wrong-PIN lockouts.
    guard: std::sync::Arc<std::sync::Mutex<PinGuard>>,
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
    let listener = match bind_lan(ip) {
        Ok(listener) => Some(listener),
        Err(err) => {
            error!("Phone controller: no LAN port on {ip}: {err}");
            None
        }
    };
    let port = listener
        .as_ref()
        .and_then(|l| l.local_addr().ok())
        .map_or(PORT, |a| a.port());
    let pin = format!("{:06}", rand::random_range(0..1_000_000));
    let url = join_url(ip, port, &pin);
    info!("Phone join {url}");
    let (tx, rx) = mpsc::channel();
    let (out, _) = broadcast::channel(256);
    let shared = Shared {
        pin: pin.clone(),
        tx,
        out: out.clone(),
        next: std::sync::Arc::default(),
        seats: std::sync::Arc::default(),
        guard: std::sync::Arc::new(std::sync::Mutex::new(PinGuard::new(Instant::now()))),
    };
    let addr = SocketAddr::new(ip, port);
    let (id_tx, id_rx) = mpsc::channel();
    commands.insert_resource(Anywhere {
        rx: std::sync::Mutex::new(id_rx),
        pin: pin.clone(),
        lan: addr,
    });
    let (stop, mut stopped) = tokio::sync::oneshot::channel::<()>();
    let thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            // Anywhere: iroh through n0's relays, found by endpoint id. It lives until the
            // game exits, whatever happens to the LAN server below.
            let endpoint = match iroh::Endpoint::builder(iroh::endpoint::presets::N0)
                .relay_mode(relays())
                .alpns(vec![ALPN.to_vec()])
                .bind()
                .await
            {
                Ok(endpoint) => {
                    let _ = id_tx.send(endpoint.id().to_string());
                    info!("Phone controller over iroh as {}", endpoint.id());
                    tokio::spawn(iroh_accept(endpoint.clone(), shared.clone()));
                    Some(endpoint)
                }
                Err(err) => {
                    warn!("No iroh, phones need the LAN: {err}");
                    None
                }
            };
            tokio::select! {
                () = lan_server(listener, cert, key, addr, shared) => {
                    warn!("Phone controller: LAN server is off, iroh stays up");
                    let _ = (&mut stopped).await;
                }
                _ = &mut stopped => {}
            }
            if let Some(endpoint) = endpoint {
                endpoint.close().await;
            }
        });
    });

    commands.insert_resource(Shutdown(Some((stop, thread))));
    phones.rx = Some(std::sync::Mutex::new(rx));
    phones.tx = Some(out);
    if let Some(image) = qr_image(&url) {
        commands.insert_resource(JoinInfo {
            anywhere: None,
            url,
            pin,
            qr: images.add(image),
        });
    }
}

/// Stops the phone thread and closes the iroh endpoint gracefully when the game exits.
#[derive(Resource)]
struct Shutdown(
    Option<(
        tokio::sync::oneshot::Sender<()>,
        std::thread::JoinHandle<()>,
    )>,
);

impl Drop for Shutdown {
    fn drop(&mut self) {
        if let Some((stop, thread)) = self.0.take() {
            let _ = stop.send(());
            let _ = thread.join();
        }
    }
}

fn join_url(
    ip: IpAddr,
    port: u16,
    pin: &str,
) -> String {
    format!("https://{ip}:{port}/?pin={pin}")
}

/// Claims the LAN port: `PORT`, the next few, then any free one. Another running instance
/// keeps its own port and its phones.
fn bind_lan(ip: IpAddr) -> std::io::Result<std::net::TcpListener> {
    let ports = (PORT..=PORT + FALLBACKS).chain([0]);
    let mut last = None;
    for port in ports {
        match std::net::TcpListener::bind(SocketAddr::new(ip, port)) {
            Ok(listener) => {
                listener.set_nonblocking(true)?;
                if port != PORT {
                    warn!("Port {PORT} is taken (another instance running?), using {port}");
                }
                return Ok(listener);
            }
            Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => last = Some(err),
            Err(err) => return Err(err),
        }
    }
    Err(last.unwrap())
}

/// The HTTPS page and websocket; returns when it fails or has no port, never the endpoint's concern.
async fn lan_server(
    listener: Option<std::net::TcpListener>,
    cert: Vec<u8>,
    key: Vec<u8>,
    addr: SocketAddr,
    shared: Shared,
) {
    let Some(listener) = listener else { return };
    let config = match RustlsConfig::from_pem(cert, key).await {
        Ok(config) => config,
        Err(err) => {
            error!("Phone controller TLS: {err}");
            return;
        }
    };
    let mut app = Router::new().route("/", get(page));
    for &(path, kind, bytes) in FILES {
        app = app.route(
            path,
            get(move || async move { ([(axum::http::header::CONTENT_TYPE, kind)], bytes) }),
        );
    }
    let app = app.route("/ws", get(socket)).with_state(shared);
    info!("Phone controller on https://{addr}");
    match axum_server::from_tcp_rustls(listener, config) {
        Ok(server) => {
            if let Err(err) = server
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await
            {
                error!("Phone controller stopped: {err}");
            }
        }
        Err(err) => error!("Phone controller stopped: {err}"),
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
    /// The player's chosen name.
    #[serde(default)]
    name: String,
}

async fn socket(
    upgrade: WebSocketUpgrade,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    UrlQuery(join): UrlQuery<Join>,
    AxumState(shared): AxumState<Shared>,
) -> impl IntoResponse {
    let id = match admit(&shared, Some(peer.ip()), &join.pin, &join.token) {
        Ok(id) => id,
        Err(Denied::Pin) => return (StatusCode::FORBIDDEN, "Wrong PIN").into_response(),
        Err(Denied::Locked) => {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                "Too many wrong PINs, wait a bit",
            )
                .into_response();
        }
        Err(Denied::Full) => return (StatusCode::SERVICE_UNAVAILABLE, "Room full").into_response(),
    };
    let name = Some(join.name);
    upgrade
        .max_message_size(MAX_MESSAGE)
        .max_frame_size(MAX_MESSAGE)
        .on_upgrade(move |ws| session(ws, shared, id, name))
}

async fn session(
    ws: WebSocket,
    shared: Shared,
    id: u32,
    name: Option<String>,
) {
    let _ = shared.tx.send(Inbound::Joined(id, name));
    let (mut sink, mut stream) = ws.split();
    let mut out = shared.out.subscribe();
    let mut rate = Rate::new(Instant::now());
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
                if !rate.allow(Instant::now()) {
                    break;
                }
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
                let mine = for_phone(&text, id);
                if mine && sink.send(WsMessage::Text(text.into())).await.is_err() {
                    break;
                }
            }
        }
    }
    shared.seats.lock().unwrap().release(id);
    let _ = shared.tx.send(Inbound::Left(id));
}

/// Messages carry an id when meant for one phone only.
fn for_phone(
    text: &str,
    id: u32,
) -> bool {
    !text.contains(r#""id":"#)
        || text.contains(&format!(r#""id":{id},"#))
        || text.contains(&format!(r#""id":{id}}}"#))
}

/// Phones kept in the seat table: twice the karts, so reloads find their seat.
const SEAT_CAP: usize = 12;

#[derive(Debug)]
enum Denied {
    Pin,
    Locked,
    Full,
}

/// Checks the PIN (counting failures) and gives the phone its seat: a token only counts after a right PIN.
fn admit(
    shared: &Shared,
    addr: Option<IpAddr>,
    pin: &str,
    token: &str,
) -> Result<u32, Denied> {
    let now = Instant::now();
    let mut guard = shared.guard.lock().expect("guard lock");
    if !guard.allowed(addr, now) {
        return Err(Denied::Locked);
    }
    if pin != shared.pin {
        guard.failed(addr, now);
        return Err(Denied::Pin);
    }
    guard.succeeded(addr);
    drop(guard);
    claim(shared, token).ok_or(Denied::Full)
}

/// The connection id for a phone token, the same one again after a reconnect.
fn claim(
    shared: &Shared,
    token: &str,
) -> Option<u32> {
    let fresh = || {
        shared
            .next
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1
    };
    shared
        .seats
        .lock()
        .expect("seats lock")
        .claim(token, SEAT_CAP, fresh)
}

/// n0's relays without the trailing dot iroh writes in their hostnames: `WebKit` (every iOS
/// browser) rejects the TLS certificate of `use1-1.relay.n0.iroh.link.`, and phones dial the
/// home relay this endpoint publishes. Same map as the pad's (`pad/src/lib.rs`).
fn relays() -> iroh::RelayMode {
    use iroh::defaults::prod;
    let urls = [
        prod::NA_EAST_RELAY_HOSTNAME,
        prod::NA_WEST_RELAY_HOSTNAME,
        prod::EU_RELAY_HOSTNAME,
        prod::AP_RELAY_HOSTNAME,
    ]
    .map(|host| {
        let url: iroh::RelayUrl = format!("https://{}", host.trim_end_matches('.'))
            .parse()
            .expect("relay url");
        iroh::RelayConfig::from(url)
    });
    iroh::RelayMode::Custom(iroh::RelayMap::from_iter(urls))
}

/// First line on an iroh stream.
#[derive(Deserialize)]
struct Hello {
    pin: String,
    #[serde(default)]
    token: String,
    #[serde(default)]
    name: String,
}

/// Accept phones over iroh: relayed or direct QUIC, newline-delimited JSON on one stream.
async fn iroh_accept(
    endpoint: iroh::Endpoint,
    shared: Shared,
) {
    while let Some(incoming) = endpoint.accept().await {
        let shared = shared.clone();
        tokio::spawn(async move {
            let Ok(connection) = incoming.await else {
                return;
            };
            let Ok((send, recv)) = connection.accept_bi().await else {
                return;
            };
            iroh_session(send, recv, shared).await;
            drop(connection);
        });
    }
}

async fn iroh_session(
    mut send: iroh::endpoint::SendStream,
    mut recv: iroh::endpoint::RecvStream,
    shared: Shared,
) {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; 4096];
    let mut id = None;
    let mut rate = Rate::new(Instant::now());
    let mut out = shared.out.subscribe();
    'session: loop {
        tokio::select! {
            read = recv.read(&mut chunk) => {
                let Ok(Some(n)) = read else {
                    break;
                };
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() > 65_536 {
                    break;
                }
                while let Some(end) = buf.iter().position(|b| *b == b'\n') {
                    let line: Vec<u8> = buf.drain(..=end).collect();
                    let text = String::from_utf8_lossy(&line[..line.len() - 1]).into_owned();
                    match id {
                        // The first line must carry the PIN
                        None => {
                            let Ok(hello) = serde_json::from_str::<Hello>(&text) else {
                                return;
                            };
                            let Ok(phone) = admit(&shared, None, &hello.pin, &hello.token) else {
                                let _ = send.write_all(b"{\"t\":\"denied\"}\n").await;
                                let _ = send.finish();
                                return;
                            };
                            id = Some(phone);
                            let _ = shared.tx.send(Inbound::Joined(phone, Some(hello.name)));
                            let _ = send
                                .write_all(format!("{{\"t\":\"hello\",\"id\":{phone}}}\n").as_bytes())
                                .await;
                        }
                        Some(phone) => {
                            if !rate.allow(Instant::now()) {
                                break 'session;
                            }
                            let Ok(wire) = serde_json::from_str::<Wire>(&text) else {
                                continue;
                            };
                            if shared.tx.send(Inbound::Wire(phone, wire)).is_err() {
                                break 'session;
                            }
                        }
                    }
                }
            }
            outgoing = out.recv(), if id.is_some() => {
                let Ok(text) = outgoing else {
                    continue;
                };
                let phone = id.unwrap_or_default();
                if for_phone(&text, phone) && send.write_all(format!("{text}\n").as_bytes()).await.is_err() {
                    break;
                }
            }
        }
    }
    if let Some(phone) = id {
        shared.seats.lock().unwrap().release(phone);
        let _ = shared.tx.send(Inbound::Left(phone));
    }
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
        pad.pick = None;
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
            Inbound::Joined(id, name) => {
                info!("Phone {id} connected");
                let name = name.as_deref().and_then(sanitize);
                match phones.pads.iter_mut().find(|(p, _)| *p == id) {
                    Some((_, pad)) => {
                        pad.gone = None;
                        pad.told = false;
                        pad.told_race = None;
                        if name.is_some() {
                            pad.name = name;
                        }
                    }
                    None => phones.pads.push((
                        id,
                        Pad {
                            name,
                            ..Pad::default()
                        },
                    )),
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
                    k => {
                        if let Some(n) = k.strip_prefix("pick").and_then(|n| n.parse().ok())
                            && let Some((_, pad)) = phones.pads.iter_mut().find(|(p, _)| *p == id)
                        {
                            pad.pick = Some(n);
                        }
                    }
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

/// Tell each phone which seat it holds, so its page can show "P2", and whether the race is on.
fn tell_seats(
    mut phones: ResMut<Phones>,
    seats: Res<Seats>,
    names: Res<SeatNames>,
    state: Res<State<GameState>>,
) {
    let race = *state.get() == GameState::Race;
    let mut messages = Vec::new();
    for (id, pad) in &mut phones.pads {
        if pad.told_race != Some(race) {
            pad.told_race = Some(race);
            messages.push(format!(r#"{{"t":"screen","id":{id},"race":{race}}}"#));
        }
        let slot = seats.0.iter().position(|d| *d == Device::Phone(*id));
        // Only on change, or to a phone that just (re)connected
        let name = slot.map(|s| names.get(s));
        if pad.told && pad.told_seat == slot && pad.told_name == name {
            continue;
        }
        pad.told = true;
        pad.told_seat = slot;
        pad.told_name = name;
        let message = seats
            .0
            .iter()
            .position(|d| *d == Device::Phone(*id))
            .map_or_else(
                || format!(r#"{{"t":"seat","id":{id},"p":null}}"#),
                |slot| {
                    let [r, g, b, _] = seat_color(slot).to_srgba().to_u8_array();
                    format!(
                        r##"{{"t":"seat","id":{id},"p":{},"color":"#{r:02x}{g:02x}{b:02x}","name":{}}}"##,
                        slot + 1,
                        serde_json::to_string(&names.get(slot)).unwrap_or_default()
                    )
                },
            );
        messages.push(message);
    }
    for message in messages {
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

#[cfg(test)]
mod port_tests {
    use super::*;

    #[test]
    fn falls_back_when_the_port_is_taken() {
        let ip: IpAddr = [127, 0, 0, 1].into();
        // Hold the default port ourselves (or find it already held).
        let _held = std::net::TcpListener::bind((ip, PORT));
        let got = bind_lan(ip).unwrap();
        let port = got.local_addr().unwrap().port();
        assert_ne!(port, PORT);
        assert!(join_url(ip, port, "1234").contains(&format!(":{port}/")));
    }

    #[test]
    fn lan_failure_leaves_the_runtime_alive() {
        // With no LAN port the server returns at once; the caller keeps iroh up afterwards.
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let (tx, _rx) = mpsc::channel();
        let shared = Shared {
            pin: String::new(),
            tx,
            out: broadcast::channel(1).0,
            next: std::sync::Arc::default(),
            seats: std::sync::Arc::default(),
            guard: std::sync::Arc::new(std::sync::Mutex::new(PinGuard::new(Instant::now()))),
        };
        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        runtime.block_on(lan_server(None, vec![], vec![], addr, shared));
    }

    fn shared(pin: &str) -> Shared {
        let (tx, _rx) = mpsc::channel();
        Shared {
            pin: pin.into(),
            tx,
            out: broadcast::channel(1).0,
            next: std::sync::Arc::default(),
            seats: std::sync::Arc::default(),
            guard: std::sync::Arc::new(std::sync::Mutex::new(PinGuard::new(Instant::now()))),
        }
    }

    #[test]
    fn five_wrong_pins_lock_out_even_the_right_one() {
        let shared = shared("123456");
        let ip: Option<IpAddr> = Some([10, 0, 0, 9].into());
        for _ in 0..5 {
            assert!(matches!(
                admit(&shared, ip, "000000", "t"),
                Err(Denied::Pin)
            ));
        }
        assert!(matches!(
            admit(&shared, ip, "123456", "t"),
            Err(Denied::Locked)
        ));
        let other: Option<IpAddr> = Some([10, 0, 0, 10].into());
        assert!(admit(&shared, other, "123456", "t").is_ok());
    }

    #[test]
    fn tokens_are_capped_and_never_stored_for_wrong_pins() {
        let shared = shared("123456");
        for n in 0..SEAT_CAP * 3 {
            let who: Option<IpAddr> = Some(IpAddr::from([10, 0, 1, u8::try_from(n).unwrap()]));
            let id = admit(&shared, who, "123456", &format!("tok{n}")).unwrap();
            shared.seats.lock().unwrap().release(id);
        }
        assert!(shared.seats.lock().unwrap().len() <= SEAT_CAP);
        let before = shared.seats.lock().unwrap().len();
        assert!(admit(&shared, None, "bad", "junk").is_err());
        assert_eq!(shared.seats.lock().unwrap().len(), before);
    }

    /// A raw websocket client: handshake, then one masked text frame over the limit.
    #[test]
    fn an_oversized_websocket_message_closes_the_connection_without_a_panic() {
        use tokio::io::AsyncReadExt;
        use tokio::io::AsyncWriteExt;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let app = Router::new().route("/ws", get(socket)).with_state(shared("123456"));
            tokio::spawn(async move {
                let _ = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await;
            });
            let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let request = format!(
                "GET /ws?pin=123456&token=x HTTP/1.1\r\nHost: {addr}\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n"
            );
            stream.write_all(request.as_bytes()).await.unwrap();
            let mut head = [0u8; 512];
            let n = stream.read(&mut head).await.unwrap();
            assert!(String::from_utf8_lossy(&head[..n]).contains("101"));
            let len = crate::phone_guard::MAX_MESSAGE + 1;
            let mut frame = vec![0x81, 127];
            frame.extend_from_slice(&(len as u64).to_be_bytes());
            frame.extend_from_slice(&[0; 4]);
            frame.extend(std::iter::repeat_n(b'a', len));
            let _ = stream.write_all(&frame).await;
            let mut sink = Vec::new();
            let closed = tokio::time::timeout(std::time::Duration::from_secs(5), stream.read_to_end(&mut sink)).await;
            assert!(closed.is_ok());
        });
    }
}
