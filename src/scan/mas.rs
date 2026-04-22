use crate::model::{Kind, SoftwareItem, Source};
use anyhow::Result;
use std::path::Path;
use std::process::Command;

pub fn scan(exe: &Path) -> Result<Vec<SoftwareItem>> {
    let Ok(output) = Command::new(exe).arg("list").output() else {
        return Ok(vec![]);
    };
    if !output.status.success() {
        return Ok(vec![]);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // mas list format: "<id>  <name>  (version)"
    let items = stdout
        .lines()
        .filter_map(|line| {
            let (_id, rest) = line.split_once(char::is_whitespace)?;
            let rest = rest.trim();
            let (name, version) = match rest.rfind('(') {
                Some(i) => (
                    rest[..i].trim().to_string(),
                    Some(rest[i + 1..].trim_end_matches(')').to_string()),
                ),
                None => (rest.to_string(), None),
            };
            let mut item = SoftwareItem::new(name, Kind::App, Source::AppStore);
            item.version = version;
            Some(item)
        })
        .collect();
    Ok(items)
}
