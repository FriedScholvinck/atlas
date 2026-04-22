mod actions;
mod cli;
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
        "scan" => run_scan(&args[1..]),
        "export" => run_export(),
        "list" => run_list(&args[1..]),
        "info" => run_info(&args[1..]),
        "doctor" => run_doctor(&args[1..]),
        "--help" | "-h" | "help" => {
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

fn run_scan(args: &[String]) -> Result<()> {
    let json = args.iter().any(|a| a == "--json");
    let available = probe::Available::detect();
    if !json {
        println!("scanning (installers: {})…", available.summary());
    }
    let t = std::time::Instant::now();
    let snap = index::scan_all(&available)?;
    let path = index::save(&snap)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "items": snap.items.len(),
                "elapsed_ms": t.elapsed().as_millis(),
                "snapshot_path": path.display().to_string(),
            }))?
        );
    } else {
        println!(
            "{} items in {:.2}s → {}",
            snap.items.len(),
            t.elapsed().as_secs_f32(),
            path.display()
        );
    }
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

fn run_list(args: &[String]) -> Result<()> {
    let mut opts = cli::ListOpts::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => opts.json = true,
            "--rescan" => opts.rescan = true,
            "--lens" => {
                i += 1;
                opts.lens = Some(cli::parse_lens(args.get(i).map(String::as_str).unwrap_or(""))?);
            }
            "--source" => {
                i += 1;
                opts.source =
                    Some(cli::parse_source(args.get(i).map(String::as_str).unwrap_or(""))?);
            }
            "--filter" => {
                i += 1;
                opts.filter = args.get(i).cloned();
            }
            other => anyhow::bail!("unknown flag: {other}"),
        }
        i += 1;
    }
    cli::list(opts)
}

fn run_info(args: &[String]) -> Result<()> {
    let mut json = false;
    let mut query: Option<&str> = None;
    for a in args {
        if a == "--json" {
            json = true;
        } else if query.is_none() {
            query = Some(a);
        }
    }
    let query = query.ok_or_else(|| anyhow::anyhow!("usage: atlas info <name|bundle-id> [--json]"))?;
    cli::info(query, json)
}

fn run_doctor(args: &[String]) -> Result<()> {
    let json = args.iter().any(|a| a == "--json");
    cli::doctor(json)
}

fn print_help() {
    println!(
        "atlas — local software graph for your Mac\n\n\
         usage:\n  \
           atlas tui                         launch the interactive TUI (default)\n  \
           atlas scan [--json]               rescan and update the on-disk index\n  \
           atlas export                      emit manifest.json + Brewfile + mas list\n\n\
         agent / script surface (machine-readable):\n  \
           atlas list [--json] [--lens <l>] [--source <s>] [--filter <q>] [--rescan]\n  \
           atlas info <name|bundle-id> [--json]\n  \
           atlas doctor [--json]            summary counts & storage\n\n\
         lenses:  all, outdated, duplicates, bloat, stale, rosetta, unsigned\n\
         sources: brew, zb, mas, nix, pkg, manual\n"
    );
}
