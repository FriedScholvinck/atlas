use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    App,
    Cli,
    Cask,
    Formula,
    Service,
    Runtime,
    Manager,
    Unknown,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::App => "app",
            Kind::Cli => "cli",
            Kind::Cask => "cask",
            Kind::Formula => "formula",
            Kind::Service => "svc",
            Kind::Runtime => "rt",
            Kind::Manager => "mgr",
            Kind::Unknown => "?",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    AppStore,
    Brew,
    Zerobrew,
    Manual,
    Npm,
    Cargo,
    Pipx,
    Uv,
    Gem,
    Go,
    Unknown,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::AppStore => "mas",
            Source::Brew => "brew",
            Source::Zerobrew => "zb",
            Source::Manual => "manual",
            Source::Npm => "npm",
            Source::Cargo => "cargo",
            Source::Pipx => "pipx",
            Source::Uv => "uv",
            Source::Gem => "gem",
            Source::Go => "go",
            Source::Unknown => "?",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Arch {
    Arm64,
    X86_64,
    Universal,
    #[default]
    Unknown,
}

impl Arch {
    pub fn label(self) -> &'static str {
        match self {
            Arch::Arm64 => "arm64",
            Arch::X86_64 => "x86_64",
            Arch::Universal => "univ",
            Arch::Unknown => "?",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Installed,
    Outdated,
    Running,
    Broken,
    Orphaned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareItem {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    pub source: Source,
    pub install_path: Option<PathBuf>,
    pub version: Option<String>,
    pub latest_version: Option<String>,
    pub status: Status,
    pub bundle_id: Option<String>,
    pub arch: Arch,
    pub size_bytes: Option<u64>,
    pub last_used: Option<DateTime<Utc>>,
    pub use_count: Option<u32>,
    pub signed: Option<bool>,
}

impl SoftwareItem {
    pub fn new(name: impl Into<String>, kind: Kind, source: Source) -> Self {
        let name = name.into();
        let id = format!("{}:{}", source.label(), name);
        Self {
            id,
            name,
            kind,
            source,
            install_path: None,
            version: None,
            latest_version: None,
            status: Status::Installed,
            bundle_id: None,
            arch: Arch::Unknown,
            size_bytes: None,
            last_used: None,
            use_count: None,
            signed: None,
        }
    }

    pub fn is_outdated(&self) -> bool {
        matches!(self.status, Status::Outdated)
            || matches!((&self.version, &self.latest_version), (Some(v), Some(l)) if v != l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_software_item_new() {
        let item = SoftwareItem::new("my-tool", Kind::Cli, Source::Npm);
        assert_eq!(item.name, "my-tool");
        assert_eq!(item.id, "npm:my-tool");
        assert_eq!(item.kind, Kind::Cli);
        assert_eq!(item.source, Source::Npm);
        assert_eq!(item.status, Status::Installed);
        assert!(!item.is_outdated());
    }

    #[test]
    fn test_software_item_outdated() {
        let mut item = SoftwareItem::new("old-tool", Kind::App, Source::Brew);
        item.status = Status::Outdated;
        assert!(item.is_outdated());
    }
}
