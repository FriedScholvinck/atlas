use crate::index::{self, Snapshot};
use crate::lenses::{self, Lens};
use crate::model::{SoftwareItem, Source};
use crate::probe::Available;
use crate::tui::app::{apply_sort, SortMode};
use anyhow::{bail, Result};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct ListOpts {
    pub lens: Option<Lens>,
    pub source: Option<Source>,
    pub filter: Option<String>,
    pub sort: Option<SortMode>,
    pub limit: Option<usize>,
    pub json: bool,
    pub rescan: bool,
}

pub fn list(opts: ListOpts) -> Result<()> {
    let snap = snapshot(opts.rescan)?;
    let mut items = select(&snap, opts.lens, opts.source, opts.filter.as_deref());
    if let Some(mode) = opts.sort {
        items = apply_sort(items, mode);
    }
    if let Some(n) = opts.limit {
        items.truncate(n);
    }
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else {
        for i in items {
            let size = i
                .size_bytes
                .map(|b| humansize::format_size(b, humansize::BINARY))
                .unwrap_or_default();
            let last = i
                .last_used
                .map(|t| t.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            println!(
                "{:<6} {:<8} {:<40} {:<14} {:>10}  {}",
                i.source.label(),
                i.kind.label(),
                truncate(&i.name, 40),
                i.version.as_deref().unwrap_or("-"),
                size,
                last,
            );
        }
    }
    Ok(())
}

pub fn info(query: &str, json: bool) -> Result<()> {
    let snap = snapshot(false)?;
    let hit = find(&snap.items, query);
    match hit {
        Some(i) if json => println!("{}", serde_json::to_string_pretty(i)?),
        Some(i) => print_human(i),
        None => bail!("no match for {query:?}"),
    }
    Ok(())
}

pub fn doctor(json: bool) -> Result<()> {
    let snap = snapshot(false)?;
    let report = Report::from(&snap);
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{report}");
    }
    Ok(())
}

fn snapshot(rescan: bool) -> Result<Snapshot> {
    if rescan {
        let available = Available::detect();
        let snap = index::scan_all(&available)?;
        let _ = index::save(&snap);
        return Ok(snap);
    }
    match index::load() {
        Some(s) => Ok(s),
        None => {
            let available = Available::detect();
            let snap = index::scan_all(&available)?;
            let _ = index::save(&snap);
            Ok(snap)
        }
    }
}

fn select<'a>(
    snap: &'a Snapshot,
    lens: Option<Lens>,
    source: Option<Source>,
    filter: Option<&str>,
) -> Vec<&'a SoftwareItem> {
    let base: Vec<&SoftwareItem> = match lens {
        Some(l) => lenses::apply(l, &snap.items),
        None => snap.items.iter().collect(),
    };
    let q = filter.map(|s| s.to_ascii_lowercase());
    base.into_iter()
        .filter(|i| source.is_none_or(|s| i.source == s))
        .filter(|i| match &q {
            None => true,
            Some(q) => {
                i.name.to_ascii_lowercase().contains(q)
                    || i.bundle_id
                        .as_deref()
                        .is_some_and(|b| b.to_ascii_lowercase().contains(q))
            }
        })
        .collect()
}

fn find<'a>(items: &'a [SoftwareItem], query: &str) -> Option<&'a SoftwareItem> {
    let q = query.to_ascii_lowercase();
    items
        .iter()
        .find(|i| i.id.to_ascii_lowercase() == q)
        .or_else(|| items.iter().find(|i| i.bundle_id.as_deref() == Some(query)))
        .or_else(|| items.iter().find(|i| i.name.to_ascii_lowercase() == q))
        .or_else(|| {
            items
                .iter()
                .find(|i| i.name.to_ascii_lowercase().contains(&q))
        })
}

fn print_human(i: &SoftwareItem) {
    println!("name      {}", i.name);
    println!("id        {}", i.id);
    println!("source    {}", i.source.label());
    println!("kind      {}", i.kind.label());
    if let Some(b) = &i.bundle_id {
        println!("bundle    {b}");
    }
    if let Some(p) = &i.install_path {
        println!("path      {}", p.display());
    }
    println!("version   {}", i.version.as_deref().unwrap_or("-"));
    if let Some(v) = &i.latest_version {
        println!("latest    {v}");
    }
    println!("arch      {}", i.arch.label());
    if let Some(sz) = i.size_bytes {
        println!(
            "size      {}",
            humansize::format_size(sz, humansize::BINARY)
        );
    }
    if let Some(t) = i.last_used {
        println!("last_used {}", t.to_rfc3339());
    }
    if let Some(s) = i.signed {
        println!("signed    {}", if s { "yes" } else { "no" });
    }
    println!("status    {:?}", i.status);
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[derive(Serialize)]
struct Report {
    generated_at: String,
    total: usize,
    by_source: BTreeMap<String, usize>,
    by_kind: BTreeMap<String, usize>,
    outdated: usize,
    duplicates: usize,
    rosetta: usize,
    stale: usize,
    total_size_bytes: u64,
    installers_available: Vec<String>,
}

impl Report {
    fn from(snap: &Snapshot) -> Self {
        let mut by_source: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
        let mut total_size = 0u64;
        for i in &snap.items {
            *by_source.entry(i.source.label().into()).or_default() += 1;
            *by_kind.entry(i.kind.label().into()).or_default() += 1;
            total_size += i.size_bytes.unwrap_or(0);
        }
        let mut installers = vec![];
        if snap.available.brew {
            installers.push("brew".into());
        }
        if snap.available.zb {
            installers.push("zb".into());
        }
        if snap.available.mas {
            installers.push("mas".into());
        }
        if snap.available.npm {
            installers.push("npm".into());
        }
        if snap.available.pipx {
            installers.push("pipx".into());
        }
        if snap.available.uv {
            installers.push("uv".into());
        }
        Report {
            generated_at: snap.generated_at.to_rfc3339(),
            total: snap.items.len(),
            by_source,
            by_kind,
            outdated: lenses::apply(Lens::Outdated, &snap.items).len(),
            duplicates: lenses::apply(Lens::Duplicates, &snap.items).len(),
            rosetta: lenses::apply(Lens::Rosetta, &snap.items).len(),
            stale: lenses::apply(Lens::Stale, &snap.items).len(),
            total_size_bytes: total_size,
            installers_available: installers,
        }
    }
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "generated  {}", self.generated_at)?;
        writeln!(f, "installers {}", self.installers_available.join(", "))?;
        writeln!(f, "total      {}", self.total)?;
        writeln!(
            f,
            "size       {}",
            humansize::format_size(self.total_size_bytes, humansize::BINARY)
        )?;
        writeln!(f, "outdated   {}", self.outdated)?;
        writeln!(f, "duplicates {}", self.duplicates)?;
        writeln!(f, "rosetta    {}", self.rosetta)?;
        writeln!(f, "stale      {}", self.stale)?;
        writeln!(f, "by source  {:?}", self.by_source)?;
        writeln!(f, "by kind    {:?}", self.by_kind)?;
        Ok(())
    }
}

pub fn parse_lens(s: &str) -> Result<Lens> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "all" => Lens::All,
        "outdated" => Lens::Outdated,
        "duplicates" | "dupes" => Lens::Duplicates,
        "bloat" => Lens::Bloat,
        "stale" => Lens::Stale,
        "rosetta" => Lens::Rosetta,
        "unsigned" => Lens::Unsigned,
        other => bail!("unknown lens: {other}"),
    })
}

pub fn parse_sort(s: &str) -> Result<SortMode> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "size" | "biggest" | "size-desc" => SortMode::SizeDesc,
        "old" | "oldest" | "stale" | "last-used-asc" => SortMode::LastUsedAsc,
        "recent" | "last-used" | "last-used-desc" => SortMode::LastUsedDesc,
        "frequent" | "most-used" | "use-desc" => SortMode::UseCountDesc,
        "rare" | "least-used" | "use-asc" => SortMode::UseCountAsc,
        "none" => SortMode::None,
        other => bail!("unknown sort: {other} (try: size, old, recent, frequent, rare)"),
    })
}

pub fn parse_source(s: &str) -> Result<Source> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "mas" | "appstore" => Source::AppStore,
        "brew" => Source::Brew,
        "zb" | "zerobrew" => Source::Zerobrew,
        "npm" => Source::Npm,
        "cargo" => Source::Cargo,
        "pipx" => Source::Pipx,
        "uv" => Source::Uv,
        "manual" | "app" => Source::Manual,
        other => bail!("unknown source: {other}"),
    })
}
