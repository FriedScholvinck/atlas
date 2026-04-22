use crate::model::{Kind, SoftwareItem, Source, Status};
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Zerobrew exposes a simpler CLI than brew:
///   `zb list`     → "<name> <version>" per line
///   `zb outdated` → "<name> (<current>) < <latest>" per line
/// No formula/cask split — everything is a formula today.
pub fn scan(exe: &Path) -> Result<Vec<SoftwareItem>> {
    let outdated = outdated_map(exe);
    let Ok(output) = Command::new(exe).arg("list").output() else {
        return Ok(vec![]);
    };
    if !output.status.success() {
        return Ok(vec![]);
    }
    let items = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| parse_list_line(line, &outdated))
        .collect();
    Ok(items)
}

fn parse_list_line(line: &str, outdated: &HashMap<String, String>) -> Option<SoftwareItem> {
    let mut parts = line.split_whitespace();
    let name = parts.next()?.to_string();
    let version = parts.next().map(String::from);
    let mut item = SoftwareItem::new(name.clone(), Kind::Formula, Source::Zerobrew);
    item.version = version;
    if let Some(latest) = outdated.get(&name) {
        item.status = Status::Outdated;
        item.latest_version = Some(latest.clone());
    }
    Some(item)
}

fn outdated_map(exe: &Path) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Ok(output) = Command::new(exe).arg("outdated").output() else {
        return out;
    };
    if !output.status.success() {
        return out;
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        // "aom (3.13.1) < 3.13.3"
        let Some(name) = line.split_whitespace().next() else {
            continue;
        };
        let Some((_, right)) = line.split_once('<') else {
            continue;
        };
        let latest = right.trim().to_string();
        if !name.is_empty() && !latest.is_empty() {
            out.insert(name.to_string(), latest);
        }
    }
    out
}
