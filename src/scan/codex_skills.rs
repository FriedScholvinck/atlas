use crate::model::{SoftwareItem, Source};
use anyhow::Result;
use std::path::Path;

pub fn scan(root: &Path) -> Result<Vec<SoftwareItem>> {
    super::agent_skills::scan(root, Source::Codex)
}
