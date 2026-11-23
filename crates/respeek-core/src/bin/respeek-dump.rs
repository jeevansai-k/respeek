//! Debug helper: connect to Magpie, take a few samples and print one snapshot as JSON.
//! Usage: respeek-dump [performance|apps|services] [device-id]
use std::sync::mpsc;
use std::time::Duration;

use respeek_core::{Engine, Page};

fn main() {
    env_logger::init();
    let mut args = std::env::args().skip(1);
    let page = match args.next().as_deref() {
        Some("apps") => Page::Apps,
        Some("services") => Page::Services,
        _ => Page::Performance,
    };
    let device = args.next();

    let (tx, rx) = mpsc::channel();
    let engine = Engine::start(move |s| {
        let _ = tx.send(s.seq);
    });
    engine.set_page(page);
    if let Some(d) = device {
        engine.select_device(&d);
    }
    // Wait for three sampler ticks so rates and histories are populated.
    let mut ticks = 0;
    let want: u32 = std::env::var("RESPEEK_DUMP_TICKS").ok().and_then(|v| v.parse().ok()).unwrap_or(3);
    while ticks < want {
        match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(_) => ticks += 1,
            Err(_) => {
                eprintln!("timed out waiting for Magpie");
                std::process::exit(1);
            }
        }
    }
    println!("{}", serde_json::to_string_pretty(&engine.snapshot()).unwrap());
    engine.shutdown();
}
