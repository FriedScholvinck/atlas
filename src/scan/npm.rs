use crate::model::{Kind, SoftwareItem, Source};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn scan(npm_exe: &Path) -> Result<Vec<SoftwareItem>> {
    let mut items = Vec::new();

    // Get the global npm root directory.
    let root_out = Command::new(npm_exe)
        .arg("root")
        .arg("-g")
        .output()
        .context("failed to run npm root -g")?;

    let root_path = String::from_utf8(root_out.stdout)
        .unwrap_or_default()
        .trim()
        .to_string();
    let root_path = PathBuf::from(root_path);

    // Get the globally installed packages in JSON format.
    let list_out = Command::new(npm_exe)
        .arg("list")
        .arg("-g")
        .arg("--depth=0")
        .arg("--json")
        .output()
        .context("failed to run npm list -g")?;

    // npm list might exit with non-zero status if there are extraneous packages,
    // but it still outputs valid JSON. So we don't strictly check list_out.status.
    let list_str = String::from_utf8(list_out.stdout).unwrap_or_default();

    // Parse the JSON output.
    let parsed: serde_json::Value =
        serde_json::from_str(&list_str).context("failed to parse npm list json")?;

    if let Some(deps) = parsed.get("dependencies").and_then(|d| d.as_object()) {
        for (name, info) in deps {
            let version = info
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut item = SoftwareItem::new(name.clone(), Kind::Cli, Source::Npm);
            item.version = version;
            item.install_path = Some(root_path.join(name));

            items.push(item);
        }
    }

    Ok(items)
}
