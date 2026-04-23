use crate::index::Snapshot;
use crate::model::{Kind, Source};
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Export JSON manifest + Brewfile + (optional) mas list into `./atlas-export/`.
/// Returns paths (relative) that were written.
pub fn export_all(snap: &Snapshot) -> Result<Vec<String>> {
    let dir = PathBuf::from("atlas-export");
    fs::create_dir_all(&dir)?;
    let mut written = vec![];

    let manifest = dir.join("manifest.json");
    fs::write(&manifest, serde_json::to_vec_pretty(snap)?)?;
    written.push(manifest.display().to_string());

    let brewfile = build_brewfile(snap);
    if !brewfile.is_empty() {
        let path = dir.join("Brewfile");
        fs::write(&path, brewfile)?;
        written.push(path.display().to_string());
    }

    let mas_list = build_mas_list(snap);
    if !mas_list.is_empty() {
        let path = dir.join("mas.txt");
        fs::write(&path, mas_list)?;
        written.push(path.display().to_string());
    }

    Ok(written)
}

fn build_brewfile(snap: &Snapshot) -> String {
    let mut out = String::new();
    let mut formulas: Vec<&str> = snap
        .items
        .iter()
        .filter(|i| matches!(i.source, Source::Brew | Source::Zerobrew) && i.kind == Kind::Formula)
        .map(|i| i.name.as_str())
        .collect();
    let mut casks: Vec<&str> = snap
        .items
        .iter()
        .filter(|i| matches!(i.source, Source::Brew | Source::Zerobrew) && i.kind == Kind::Cask)
        .map(|i| i.name.as_str())
        .collect();
    formulas.sort();
    formulas.dedup();
    casks.sort();
    casks.dedup();

    for f in formulas {
        out.push_str(&format!("brew \"{}\"\n", f));
    }
    if !out.is_empty() {
        out.push('\n');
    }
    for c in casks {
        out.push_str(&format!("cask \"{}\"\n", c));
    }
    out
}

fn build_mas_list(snap: &Snapshot) -> String {
    let mut lines: Vec<String> = snap
        .items
        .iter()
        .filter(|i| i.source == Source::AppStore)
        .map(|i| i.name.clone())
        .collect();
    lines.sort();
    lines.dedup();
    lines.join("\n")
}
