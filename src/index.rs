use crate::model::{Source, SoftwareItem};
use crate::probe::Available;
use crate::scan;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub generated_at: DateTime<Utc>,
    pub items: Vec<SoftwareItem>,
    pub available: AvailableSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableSummary {
    pub brew: bool,
    pub zb: bool,
    pub mas: bool,
    pub nix: bool,
}

impl From<&Available> for AvailableSummary {
    fn from(a: &Available) -> Self {
        Self {
            brew: a.brew.is_some(),
            zb: a.zb.is_some(),
            mas: a.mas.is_some(),
            nix: a.nix.is_some(),
        }
    }
}

pub fn snapshot_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "atlas", "Atlas")
        .map(|dirs| dirs.data_dir().join("index.json"))
}

pub fn load() -> Option<Snapshot> {
    let path = snapshot_path()?;
    let bytes = fs::read(&path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn save(snap: &Snapshot) -> Result<PathBuf> {
    let path = snapshot_path().context("no project dir")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(snap)?;
    fs::write(&path, bytes)?;
    Ok(path)
}

pub fn scan_all(available: &Available) -> Result<Snapshot> {
    let mut items: Vec<SoftwareItem> = scan::apps::scan()?;

    // Prefer zb over brew: when both available, we still run both but dedupe
    // such that a formula/cask appearing in both ends up attributed to zb.
    if let Some(zb_exe) = &available.zb {
        items.extend(scan::zerobrew::scan(zb_exe)?);
    }
    if let Some(brew_exe) = &available.brew {
        items.extend(scan::brew::scan(brew_exe)?);
    }
    if let Some(mas_exe) = &available.mas {
        items.extend(scan::mas::scan(mas_exe)?);
    }

    let merged = merge(items);

    Ok(Snapshot {
        generated_at: Utc::now(),
        items: merged,
        available: AvailableSummary::from(available),
    })
}

/// Merge duplicates across adapters.
///
/// Keys in order of confidence:
///   1. bundle_id (apps)
///   2. canonical install_path
///   3. (kind, name) where name is a package identifier (formula/cask/etc.)
///
/// When two items merge and one is from zb while the other is brew, zb wins.
fn merge(items: Vec<SoftwareItem>) -> Vec<SoftwareItem> {
    let mut by_bundle: HashMap<String, usize> = HashMap::new();
    let mut by_path: HashMap<PathBuf, usize> = HashMap::new();
    let mut by_name: HashMap<(crate::model::Kind, String), usize> = HashMap::new();
    let mut out: Vec<SoftwareItem> = Vec::with_capacity(items.len());

    for mut item in items {
        // Try to find an existing match.
        let idx = item
            .bundle_id
            .as_ref()
            .and_then(|b| by_bundle.get(b).copied())
            .or_else(|| {
                item.install_path
                    .as_ref()
                    .and_then(|p| by_path.get(p).copied())
            })
            .or_else(|| by_name.get(&(item.kind, item.name.clone())).copied());

        if let Some(i) = idx {
            merge_into(&mut out[i], &mut item);
            // Re-index so later items still find it.
            if let Some(b) = &out[i].bundle_id {
                by_bundle.insert(b.clone(), i);
            }
            if let Some(p) = &out[i].install_path {
                by_path.insert(p.clone(), i);
            }
            by_name.insert((out[i].kind, out[i].name.clone()), i);
        } else {
            let i = out.len();
            if let Some(b) = &item.bundle_id {
                by_bundle.insert(b.clone(), i);
            }
            if let Some(p) = &item.install_path {
                by_path.insert(p.clone(), i);
            }
            by_name.insert((item.kind, item.name.clone()), i);
            out.push(item);
        }
    }

    out
}

fn merge_into(dst: &mut SoftwareItem, src: &mut SoftwareItem) {
    // Source preference: Zerobrew > AppStore > Brew > everything else.
    let new_source = prefer_source(dst.source, src.source);
    dst.source = new_source;

    if dst.install_path.is_none() {
        dst.install_path = src.install_path.take();
    }
    if dst.bundle_id.is_none() {
        dst.bundle_id = src.bundle_id.take();
    }
    if dst.version.is_none() {
        dst.version = src.version.take();
    }
    if dst.latest_version.is_none() {
        dst.latest_version = src.latest_version.take();
    }
    if dst.size_bytes.is_none() {
        dst.size_bytes = src.size_bytes;
    }
    if dst.last_used.is_none() {
        dst.last_used = src.last_used;
    }
    if matches!(dst.arch, crate::model::Arch::Unknown) {
        dst.arch = src.arch;
    }
    if src.is_outdated() {
        dst.status = crate::model::Status::Outdated;
    }
}

fn prefer_source(a: Source, b: Source) -> Source {
    let rank = |s: Source| match s {
        Source::Zerobrew => 0,
        Source::AppStore => 1,
        Source::Brew => 2,
        Source::Nix => 3,
        Source::Pkg => 4,
        Source::Manual => 5,
        _ => 6,
    };
    if rank(a) <= rank(b) { a } else { b }
}
