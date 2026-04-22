mod actions;
mod index;
mod lenses;
mod manifest;
mod model;
mod probe;
mod scan;
mod tui;

use anyhow::Result;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mode = args.first().map(|s| s.as_str()).unwrap_or("tui");

    match mode {
        "tui" => run_tui(),
        "scan" => run_scan(),
        "export" => run_export(),
        "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            eprintln!("unknown command: {other}");
            print_help();
            std::process::exit(2);
        }
    }
}

fn run_tui() -> Result<()> {
    let available = probe::Available::detect();
    let snapshot = match index::load() {
        Some(snap) => snap,
        None => {
            eprintln!("no cached index — scanning (installers: {})…", available.summary());
            let snap = index::scan_all(&available)?;
            let _ = index::save(&snap);
            snap
        }
    };
    tui::run(snapshot)
}

fn run_scan() -> Result<()> {
    let available = probe::Available::detect();
    println!("scanning (installers: {})…", available.summary());
    let t = std::time::Instant::now();
    let snap = index::scan_all(&available)?;
    let path = index::save(&snap)?;
    println!(
        "{} items in {:.2}s → {}",
        snap.items.len(),
        t.elapsed().as_secs_f32(),
        path.display()
    );
    Ok(())
}

fn run_export() -> Result<()> {
    let snap = index::load()
        .ok_or_else(|| anyhow::anyhow!("no snapshot yet — run `atlas scan` first"))?;
    let paths = manifest::export_all(&snap)?;
    for p in paths {
        println!("wrote {p}");
    }
    Ok(())
}

fn print_help() {
    println!(
        "atlas — local software graph for your Mac\n\n\
         usage:\n  \
           atlas tui       launch the interactive TUI (default)\n  \
           atlas scan      run a full rescan and update the on-disk index\n  \
           atlas export    emit manifest.json + Brewfile + mas list to ./atlas-export/\n"
    );
}
