use crate::model::{Kind, SoftwareItem, Source};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn scan(root: &Path, source: Source) -> Result<Vec<SoftwareItem>> {
    let mut items = Vec::new();

    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let Ok(entry) = entry else {
            eprintln!(
                "warning: failed to read skill entry under {}",
                root.display()
            );
            continue;
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        match parse_skill_dir(&path, source) {
            Ok(Some(item)) => items.push(item),
            Ok(None) => {}
            Err(e) => eprintln!("warning: skipping skill {}: {e}", path.display()),
        }
    }

    Ok(items)
}

fn parse_skill_dir(path: &Path, source: Source) -> Result<Option<SoftwareItem>> {
    let manifest = path.join("SKILL.md");
    if !manifest.is_file() {
        eprintln!(
            "warning: skipping skill {}: missing SKILL.md",
            path.display()
        );
        return Ok(None);
    }

    let markdown = fs::read_to_string(&manifest)
        .with_context(|| format!("failed to read {}", manifest.display()))?;
    let frontmatter = parse_frontmatter(&markdown);

    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .context("skill directory has no valid UTF-8 name")?
        .to_string();

    let mut item = SoftwareItem::new(name.clone(), Kind::Skill, source);
    item.id = format!("{}:{name}", source.label());
    item.install_path = Some(PathBuf::from(path));
    item.size_bytes = dir_size(path);
    item.bundle_id = frontmatter.name;
    item.version = frontmatter.version;

    Ok(Some(item))
}

#[derive(Default)]
struct SkillFrontmatter {
    name: Option<String>,
    version: Option<String>,
}

fn parse_frontmatter(markdown: &str) -> SkillFrontmatter {
    let mut out = SkillFrontmatter::default();
    let mut lines = markdown.lines();
    if lines.next().map(str::trim) != Some("---") {
        return out;
    }

    for line in lines {
        let line = line.trim();
        if line == "---" {
            break;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if value.is_empty() {
            continue;
        }
        match key.trim() {
            "name" => out.name = Some(value.to_string()),
            "version" => out.version = Some(value.to_string()),
            _ => {}
        }
    }

    out
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
    if total == 0 {
        None
    } else {
        Some(total)
    }
}
