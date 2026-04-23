use crate::model::{Kind, SoftwareItem, Source};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn scan(uv_exe: &Path) -> Result<Vec<SoftwareItem>> {
    let mut items = Vec::new();

    // Get uv tool dir
    let dir_out = Command::new(uv_exe)
        .arg("tool")
        .arg("dir")
        .output()
        .context("failed to run uv tool dir")?;

    let dir_path = String::from_utf8(dir_out.stdout)
        .unwrap_or_default()
        .trim()
        .to_string();
    let root_path = PathBuf::from(dir_path);

    let list_out = Command::new(uv_exe)
        .arg("tool")
        .arg("list")
        .output()
        .context("failed to run uv tool list")?;

    let list_str = String::from_utf8(list_out.stdout).unwrap_or_default();

    // Parse output which looks like:
    // ruff v0.1.2
    // - ruff
    // something-else v1.0.0
    // - something
    for line in list_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('-') {
            continue;
        }

        // e.g. "ruff v0.1.2"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if !parts.is_empty() {
            let name = parts[0];
            let mut version = None;
            if parts.len() > 1 {
                version = Some(parts[1].trim_start_matches('v').to_string());
            }

            let mut item = SoftwareItem::new(name, Kind::Cli, Source::Uv);
            item.version = version;
            item.install_path = Some(root_path.join(name));
            items.push(item);
        }
    }

    Ok(items)
}
