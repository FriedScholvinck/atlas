use crate::model::{Kind, SoftwareItem, Source};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn scan(pipx_exe: &Path) -> Result<Vec<SoftwareItem>> {
    let mut items = Vec::new();

    let out = Command::new(pipx_exe)
        .arg("list")
        .arg("--json")
        .output()
        .context("failed to run pipx list --json")?;

    let json_str = String::from_utf8(out.stdout).unwrap_or_default();
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).context("failed to parse pipx list json")?;

    if let Some(venvs) = parsed.get("venvs").and_then(|v| v.as_object()) {
        for (name, info) in venvs {
            let version = info
                .get("metadata")
                .and_then(|m| m.get("main_package"))
                .and_then(|mp| mp.get("package_version"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut item = SoftwareItem::new(name.clone(), Kind::Cli, Source::Pipx);
            item.version = version;

            // Try to find the app path.
            if let Some(paths) = info
                .get("metadata")
                .and_then(|m| m.get("main_package"))
                .and_then(|mp| mp.get("app_paths"))
                .and_then(|ap| ap.as_array())
            {
                if let Some(first_path) = paths.first() {
                    if let Some(p) = first_path.get("__Path__").and_then(|p| p.as_str()) {
                        item.install_path = Some(PathBuf::from(p));
                    }
                }
            }

            items.push(item);
        }
    }

    Ok(items)
}
