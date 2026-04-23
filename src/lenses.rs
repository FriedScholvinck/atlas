use crate::model::{Arch, Kind, SoftwareItem};
use chrono::{Duration, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lens {
    All,
    Outdated,
    Duplicates,
    Bloat,
    Stale,
    Rosetta,
    Unsigned,
}

impl Lens {
    pub const ORDER: &'static [Lens] = &[
        Lens::All,
        Lens::Outdated,
        Lens::Duplicates,
        Lens::Bloat,
        Lens::Stale,
        Lens::Rosetta,
        Lens::Unsigned,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Lens::All => "All",
            Lens::Outdated => "Outdated",
            Lens::Duplicates => "Duplicates",
            Lens::Bloat => "Bloat",
            Lens::Stale => "Stale",
            Lens::Rosetta => "Rosetta",
            Lens::Unsigned => "Unsigned",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Lens::All => "everything on disk",
            Lens::Outdated => "upgrade available",
            Lens::Duplicates => "same tool, multiple sources",
            Lens::Bloat => "largest on disk",
            Lens::Stale => "not opened in 90+ days",
            Lens::Rosetta => "x86_64 on Apple Silicon",
            Lens::Unsigned => "missing code signature",
        }
    }
}

pub fn apply(lens: Lens, items: &[SoftwareItem]) -> Vec<&SoftwareItem> {
    match lens {
        Lens::All => items.iter().collect(),
        Lens::Outdated => items.iter().filter(|i| i.is_outdated()).collect(),
        Lens::Duplicates => duplicates(items),
        Lens::Bloat => bloat(items, 50),
        Lens::Stale => stale(items, 90),
        Lens::Rosetta => items
            .iter()
            .filter(|i| i.kind == Kind::App && i.arch == Arch::X86_64)
            .collect(),
        Lens::Unsigned => items.iter().filter(|i| i.signed == Some(false)).collect(),
    }
}

/// Count items sharing a normalized base name across different sources.
fn duplicates(items: &[SoftwareItem]) -> Vec<&SoftwareItem> {
    let mut by_name: HashMap<String, Vec<&SoftwareItem>> = HashMap::new();
    for item in items {
        let key = normalize_name(&item.name);
        by_name.entry(key).or_default().push(item);
    }
    let mut out = vec![];
    for group in by_name.into_values() {
        if group.len() < 2 {
            continue;
        }
        // Dedupe requires different *sources* — same tool from two managers.
        let mut sources: Vec<_> = group.iter().map(|i| i.source).collect();
        sources.sort_by_key(|s| s.label());
        sources.dedup();
        if sources.len() >= 2 {
            out.extend(group);
        }
    }
    out.sort_by_key(|a| normalize_name(&a.name));
    out
}

fn normalize_name(name: &str) -> String {
    // Strip common version suffixes and case differences.
    let base = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '@');
    base.to_ascii_lowercase()
}

fn bloat(items: &[SoftwareItem], top: usize) -> Vec<&SoftwareItem> {
    let mut scored: Vec<&SoftwareItem> = items.iter().filter(|i| i.size_bytes.is_some()).collect();
    scored.sort_by_key(|b| std::cmp::Reverse(b.size_bytes));
    scored.into_iter().take(top).collect()
}

fn stale(items: &[SoftwareItem], days: i64) -> Vec<&SoftwareItem> {
    let cutoff = Utc::now() - Duration::days(days);
    let mut out: Vec<&SoftwareItem> = items
        .iter()
        .filter(|i| i.kind == Kind::App)
        .filter(|i| match i.last_used {
            Some(t) => t < cutoff,
            None => false,
        })
        .collect();
    out.sort_by_key(|i| i.last_used);
    out
}
