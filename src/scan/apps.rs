use crate::model::{Arch, Kind, SoftwareItem, Source};
use anyhow::Result;
use chrono::{DateTime, Utc};
use plist::Value;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

const APP_ROOTS: &[&str] = &["/Applications", "/System/Applications"];

pub fn scan() -> Result<Vec<SoftwareItem>> {
    let mut roots: Vec<PathBuf> = APP_ROOTS.iter().map(PathBuf::from).collect();
    if let Some(home) = directories::UserDirs::new().map(|d| d.home_dir().to_path_buf()) {
        roots.push(home.join("Applications"));
    }

    let bundles: Vec<PathBuf> = roots
        .iter()
        .flat_map(|root| find_app_bundles(root))
        .collect();

    let items: Vec<SoftwareItem> = bundles
        .par_iter()
        .filter_map(|path| parse_bundle(path).ok())
        .collect();

    Ok(items)
}

fn find_app_bundles(root: &Path) -> Vec<PathBuf> {
    if !root.exists() {
        return vec![];
    }
    let mut out = vec![];
    for entry in WalkDir::new(root)
        .max_depth(3)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("app") && path.is_dir() {
            out.push(path.to_path_buf());
        }
    }
    out
}

fn parse_bundle(path: &Path) -> Result<SoftwareItem> {
    let info = path.join("Contents/Info.plist");
    let name_fallback = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let plist = Value::from_file(&info).ok();

    let (name, bundle_id, version, arch_from_plist, executable) = plist
        .as_ref()
        .and_then(|v| v.as_dictionary())
        .map(|dict| {
            let name = dict
                .get("CFBundleDisplayName")
                .or_else(|| dict.get("CFBundleName"))
                .and_then(|v| v.as_string())
                .map(String::from)
                .unwrap_or_else(|| name_fallback.clone());
            let bundle_id = dict
                .get("CFBundleIdentifier")
                .and_then(|v| v.as_string())
                .map(String::from);
            let version = dict
                .get("CFBundleShortVersionString")
                .or_else(|| dict.get("CFBundleVersion"))
                .and_then(|v| v.as_string())
                .map(String::from);
            let arch = dict
                .get("LSArchitecturePriority")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    let labels: Vec<&str> = arr.iter().filter_map(|v| v.as_string()).collect();
                    arch_from_labels(&labels)
                });
            let executable = dict
                .get("CFBundleExecutable")
                .and_then(|v| v.as_string())
                .map(String::from);
            (name, bundle_id, version, arch, executable)
        })
        .unwrap_or((name_fallback, None, None, None, None));

    let arch = match arch_from_plist {
        Some(a) if a != Arch::Unknown => a,
        _ => probe_arch_via_lipo(path, executable.as_deref()),
    };

    let size = dir_size(path);
    let last_used = spotlight_last_used(path);

    let mut item = SoftwareItem::new(name, Kind::App, Source::Manual);
    item.install_path = Some(path.to_path_buf());
    item.bundle_id = bundle_id;
    item.version = version;
    item.arch = arch;
    item.size_bytes = size;
    item.last_used = last_used;
    if path.starts_with("/System/Applications") {
        item.source = Source::Manual;
    }
    Ok(item)
}

fn arch_from_labels(labels: &[&str]) -> Arch {
    let has_arm = labels.iter().any(|s| *s == "arm64");
    let has_x86 = labels.iter().any(|s| *s == "x86_64");
    match (has_arm, has_x86) {
        (true, true) => Arch::Universal,
        (true, false) => Arch::Arm64,
        (false, true) => Arch::X86_64,
        _ => Arch::Unknown,
    }
}

fn probe_arch_via_lipo(bundle: &Path, executable: Option<&str>) -> Arch {
    let Some(exe) = executable else {
        return Arch::Unknown;
    };
    let exe_path = bundle.join("Contents/MacOS").join(exe);
    if !exe_path.exists() {
        return Arch::Unknown;
    }
    let Ok(output) = Command::new("/usr/bin/lipo").arg("-archs").arg(&exe_path).output() else {
        return Arch::Unknown;
    };
    if !output.status.success() {
        return Arch::Unknown;
    }
    let s = String::from_utf8_lossy(&output.stdout);
    let archs: Vec<&str> = s.split_whitespace().collect();
    arch_from_labels(&archs)
}

fn dir_size(path: &Path) -> Option<u64> {
    let mut total: u64 = 0;
    for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
        if let Ok(md) = entry.metadata() {
            if md.is_file() {
                total = total.saturating_add(md.len());
            }
        }
    }
    if total == 0 { None } else { Some(total) }
}

fn spotlight_last_used(path: &Path) -> Option<DateTime<Utc>> {
    let output = Command::new("/usr/bin/mdls")
        .args(["-raw", "-name", "kMDItemLastUsedDate"])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() || s == "(null)" {
        return None;
    }
    // mdls emits e.g. "2026-03-14 21:05:11 +0000"
    DateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S %z")
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}
