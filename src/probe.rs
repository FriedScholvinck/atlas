use std::env;
use std::path::{Path, PathBuf};

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
    pub npm: Option<PathBuf>,
    pub pipx: Option<PathBuf>,
    pub uv: Option<PathBuf>,
    pub claude_skills: Option<PathBuf>,
    pub codex_skills: Option<PathBuf>,
}

impl Available {
    pub fn detect() -> Self {
        let home = directories::UserDirs::new().map(|d| d.home_dir().to_path_buf());
        Self {
            brew: which("brew"),
            zb: which("zb"),
            mas: which("mas"),
            npm: which("npm"),
            pipx: which("pipx"),
            uv: which("uv"),
            // Skills are owned by host config dirs, so presence/readability replaces which(1).
            claude_skills: home
                .as_ref()
                .and_then(|h| readable_dir(h.join(".claude/skills"))),
            codex_skills: home
                .as_ref()
                .and_then(|h| readable_dir(h.join(".codex/skills"))),
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
        if self.npm.is_some() {
            parts.push("npm");
        }
        if self.pipx.is_some() {
            parts.push("pipx");
        }
        if self.uv.is_some() {
            parts.push("uv");
        }
        if self.claude_skills.is_some() {
            parts.push("claude");
        }
        if self.codex_skills.is_some() {
            parts.push("codex");
        }
        if parts.is_empty() {
            "none".into()
        } else {
            parts.join(" · ")
        }
    }
}

fn readable_dir(path: impl AsRef<Path>) -> Option<PathBuf> {
    let path = path.as_ref();
    if path.is_dir() && std::fs::read_dir(path).is_ok() {
        Some(path.to_path_buf())
    } else {
        None
    }
}
