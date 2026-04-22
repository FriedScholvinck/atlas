use std::env;
use std::path::PathBuf;

pub fn which(cmd: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[derive(Debug, Clone, Default)]
pub struct Available {
    pub brew: Option<PathBuf>,
    pub zb: Option<PathBuf>,
    pub mas: Option<PathBuf>,
    pub nix: Option<PathBuf>,
}

impl Available {
    pub fn detect() -> Self {
        Self {
            brew: which("brew"),
            zb: which("zb"),
            mas: which("mas"),
            nix: which("nix"),
        }
    }

    pub fn summary(&self) -> String {
        let mut parts = vec![];
        if self.zb.is_some() {
            parts.push("zb");
        }
        if self.brew.is_some() {
            parts.push("brew");
        }
        if self.mas.is_some() {
            parts.push("mas");
        }
        if self.nix.is_some() {
            parts.push("nix");
        }
        if parts.is_empty() {
            "none".into()
        } else {
            parts.join(" · ")
        }
    }
}
