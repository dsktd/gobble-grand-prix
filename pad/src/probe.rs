//! Native test client: join the game over iroh relays like a phone would, take a seat, and
//! measure input round trips.
//!
//!     pad-probe <game id> <pin> [seconds] [--direct]

use std::time::Instant;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tokio::task::LocalSet::new().run_until(run()).await;
}

async fn run() {
    let args: Vec<String> = std::env::args().collect();
    let (Some(id), Some(pin)) = (args.get(1), args.get(2)) else {
        eprintln!("usage: pad-probe <game id> <pin> [seconds] [--direct]");
        std::process::exit(2);
    };
    let seconds: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(10.0);
    let relay_only = !args.iter().any(|a| a == "--direct");
    let start = Instant::now();
    let link = gobble_pad::connect(id, relay_only, |task| {
        tokio::task::spawn_local(task);
    })
    .await
    .unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });
    println!("connected in {:.0} ms (relay only: {relay_only})", start.elapsed().as_secs_f64() * 1000.0);
    let token = format!("probe{}", std::process::id());
    let _ = link
        .outgoing
        .send(format!(r#"{{"t":"join","pin":"{pin}","token":"{token}","name":"probe"}}"#))
        .await;

    let clock = Instant::now();
    let now = || clock.elapsed().as_secs_f64() * 1000.0;
    let mut me = None;
    let mut seat = None;
    let mut rtts = Vec::new();
    let mut seq = 0u64;
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(16));
    let mut pressed = 0;
    let end = clock.elapsed().as_secs_f64() + seconds;
    while clock.elapsed().as_secs_f64() < end {
        tokio::select! {
            line = link.incoming.recv() => {
                let Ok(line) = line else { break };
                if line.contains(r#""t":"hello""#) {
                    me = field(&line, "id");
                    println!("hello, phone id {}", me.clone().unwrap_or_default());
                } else if line.contains(r#""t":"seat""#) && field(&line, "id") == me {
                    seat = field(&line, "p");
                    println!("seat {}", seat.clone().unwrap_or_default());
                } else if line.contains(r#""t":"ack""#) && field(&line, "id") == me
                    && let Some(ms) = field(&line, "ms").and_then(|v| v.parse::<f64>().ok())
                {
                    rtts.push(now() - ms);
                } else if line.contains(r#""t":"denied""#) {
                    println!("{line}");
                    break;
                }
            }
            _ = tick.tick() => {
                // Title menu to the lobby, then join a seat
                // Title menu, join, start, ready, track: then race
                if me.is_some() && pressed < 5 && seq % 40 == 0 {
                    let _ = link.outgoing.send(r#"{"t":"menu","k":"ok"}"#.to_string()).await;
                    pressed += 1;
                }
                seq += 1;
                let _ = link.outgoing.send(format!(
                    r#"{{"t":"in","s":{seq},"ms":{},"steer":0,"gas":false,"brake":false,"drift":false,"item":false}}"#,
                    now()
                )).await;
            }
        }
    }
    rtts.sort_by(f64::total_cmp);
    println!("seat: {}", seat.unwrap_or_else(|| "none".into()));
    if rtts.is_empty() {
        println!("no input acks (acks only come during a race)");
    } else {
        println!(
            "input round trip ms over {} acks: median {:.1}, p95 {:.1}, max {:.1}",
            rtts.len(),
            rtts[rtts.len() / 2],
            rtts[rtts.len() * 95 / 100],
            rtts[rtts.len() - 1]
        );
    }
}

/// Pull a scalar field out of a flat JSON line without a JSON crate.
fn field(
    line: &str,
    key: &str,
) -> Option<String> {
    let at = line.find(&format!("\"{key}\":"))? + key.len() + 3;
    let rest = line[at..].trim_start_matches('"');
    let end = rest.find(['"', ',', '}']).unwrap_or(rest.len());
    Some(rest[..end].to_string())
}
