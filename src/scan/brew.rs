use crate::model::{Kind, SoftwareItem, Source, Status};
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

pub fn scan(exe: &Path) -> Result<Vec<SoftwareItem>> {
    scan_as(exe, Source::Brew)
}

pub fn scan_as(exe: &Path, source: Source) -> Result<Vec<SoftwareItem>> {
    let outdated = outdated_set(exe);
    let mut items = vec![];
    items.extend(list_kind(
        exe,
        "--formula",
        Kind::Formula,
        source,
        &outdated,
    ));
    items.extend(list_kind(exe, "--cask", Kind::Cask, source, &outdated));
    Ok(items)
}

fn list_kind(
    exe: &Path,
    flag: &str,
    kind: Kind,
    source: Source,
    outdated: &HashSet<String>,
) -> Vec<SoftwareItem> {
    let Ok(output) = Command::new(exe)
        .args(["list", flag, "--versions"])
        .output()
    else {
        return vec![];
    };
    if !output.status.success() {
        return vec![];
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?.to_string();
            let version = parts.next().map(String::from);
            let mut item = SoftwareItem::new(name.clone(), kind, source);
            item.version = version;
            if outdated.contains(&name) {
                item.status = Status::Outdated;
            }
            Some(item)
        })
        .collect()
}

fn outdated_set(exe: &Path) -> HashSet<String> {
    let mut out = HashSet::new();
    let Ok(output) = Command::new(exe).args(["outdated", "--quiet"]).output() else {
        return out;
    };
    if !output.status.success() {
        return out;
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let name = line.split_whitespace().next().unwrap_or("").to_string();
        if !name.is_empty() {
            out.insert(name);
        }
    }
    out
}
