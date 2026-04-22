use crate::actions::{self, Action};
use crate::index::Snapshot;
use crate::lenses::Lens;
use crate::model::{SoftwareItem, Source};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use std::io::{self, Write};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Lenses,
    Inventory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    None,
    SizeDesc,
    LastUsedAsc,
    LastUsedDesc,
    UseCountDesc,
    UseCountAsc,
}

impl SortMode {
    pub const ORDER: &'static [SortMode] = &[
        SortMode::None,
        SortMode::SizeDesc,
        SortMode::LastUsedAsc,
        SortMode::LastUsedDesc,
        SortMode::UseCountDesc,
        SortMode::UseCountAsc,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SortMode::None => "—",
            SortMode::SizeDesc => "biggest",
            SortMode::LastUsedAsc => "longest ago",
            SortMode::LastUsedDesc => "recently used",
            SortMode::UseCountDesc => "most used",
            SortMode::UseCountAsc => "least used",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    Help,
    Confirm(ConfirmPrompt),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmPrompt {
    pub title: String,
    pub lines: Vec<String>,
    pub destructive: bool,
    pub action_id: u64,
}

pub struct App {
    pub snapshot: Snapshot,
    pub lens: Lens,
    pub pane: Pane,
    pub mode: Mode,
    pub query: String,
    pub list_cursor: usize,
    pub lens_cursor: usize,
    pub source_filter: Option<Source>,
    pub sort: SortMode,
    pub status: Option<String>,

    pending: Option<(u64, Action)>,
    next_action_id: u64,
}

const SOURCE_CYCLE: &[Option<Source>] = &[
    None,
    Some(Source::Brew),
    Some(Source::Zerobrew),
    Some(Source::AppStore),
    Some(Source::Manual),
];

impl App {
    pub fn new(snapshot: Snapshot) -> Self {
        Self {
            snapshot,
            lens: Lens::All,
            pane: Pane::Inventory,
            mode: Mode::Normal,
            query: String::new(),
            list_cursor: 0,
            lens_cursor: 0,
            source_filter: None,
            sort: SortMode::None,
            status: None,
            pending: None,
            next_action_id: 0,
        }
    }

    pub fn visible(&self) -> Vec<&SoftwareItem> {
        let lensed = crate::lenses::apply(self.lens, &self.snapshot.items);
        let filtered: Vec<&SoftwareItem> = lensed
            .into_iter()
            .filter(|i| match self.source_filter {
                None => true,
                Some(s) => i.source == s,
            })
            .collect();
        let searched: Vec<&SoftwareItem> = if self.query.is_empty() {
            filtered
        } else {
            let q = self.query.to_ascii_lowercase();
            filtered
                .into_iter()
                .filter(|i| {
                    i.name.to_ascii_lowercase().contains(&q)
                        || i.bundle_id
                            .as_deref()
                            .map(|b| b.to_ascii_lowercase().contains(&q))
                            .unwrap_or(false)
                        || i.source.label().contains(&q)
                })
                .collect()
        };
        apply_sort(searched, self.sort)
    }

    pub fn selected(&self) -> Option<&SoftwareItem> {
        self.visible().get(self.list_cursor).copied()
    }

    pub fn source_filter_label(&self) -> &'static str {
        match self.source_filter {
            None => "all",
            Some(Source::Brew) => "brew",
            Some(Source::Zerobrew) => "zb",
            Some(Source::AppStore) => "mas",
            Some(Source::Manual) => "manual",
            Some(_) => "other",
        }
    }

    fn cycle_sort(&mut self) {
        let idx = SortMode::ORDER
            .iter()
            .position(|m| *m == self.sort)
            .unwrap_or(0);
        self.sort = SortMode::ORDER[(idx + 1) % SortMode::ORDER.len()];
        self.list_cursor = 0;
    }

    fn cycle_source_filter(&mut self, forward: bool) {
        let len = SOURCE_CYCLE.len();
        let idx = SOURCE_CYCLE
            .iter()
            .position(|s| *s == self.source_filter)
            .unwrap_or(0);
        let next = if forward { idx + 1 } else { idx + len - 1 };
        self.source_filter = SOURCE_CYCLE[next % len];
        self.list_cursor = 0;
    }

    fn clamp_cursor(&mut self) {
        let len = self.visible().len();
        if len == 0 {
            self.list_cursor = 0;
        } else if self.list_cursor >= len {
            self.list_cursor = len - 1;
        }
    }

    fn queue_action(&mut self, action: Action) {
        let id = self.next_action_id;
        self.next_action_id += 1;
        let title = action.title();
        let mut lines = action.display_cmds();
        if lines.is_empty() {
            lines.push("(no-op)".into());
        }
        let destructive = action.is_destructive();
        self.pending = Some((id, action));
        self.mode = Mode::Confirm(ConfirmPrompt {
            title,
            lines,
            destructive,
            action_id: id,
        });
    }
}

pub fn apply_sort<'a>(items: Vec<&'a SoftwareItem>, mode: SortMode) -> Vec<&'a SoftwareItem> {
    let mut out = items;
    // None sorts go to the bottom for every mode, so the "live" data is always on top.
    match mode {
        SortMode::None => {}
        SortMode::SizeDesc => {
            out.sort_by(|a, b| b.size_bytes.unwrap_or(0).cmp(&a.size_bytes.unwrap_or(0)));
        }
        SortMode::LastUsedAsc => {
            out.sort_by(|a, b| match (a.last_used, b.last_used) {
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
        }
        SortMode::LastUsedDesc => {
            out.sort_by(|a, b| match (a.last_used, b.last_used) {
                (Some(x), Some(y)) => y.cmp(&x),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
        }
        SortMode::UseCountDesc => {
            out.sort_by(|a, b| match (a.use_count, b.use_count) {
                (Some(x), Some(y)) => y.cmp(&x),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
        }
        SortMode::UseCountAsc => {
            out.sort_by(|a, b| match (a.use_count, b.use_count) {
                (Some(x), Some(y)) => x.cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
        }
    }
    out
}

pub fn run(snapshot: Snapshot) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(snapshot);
    let res = event_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn event_loop<B: Backend + io::Write>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| crate::tui::ui::draw(f, app))?;

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match &app.mode {
            Mode::Normal => {
                if handle_normal(terminal, app, key.code, key.modifiers)? {
                    return Ok(());
                }
            }
            Mode::Search => handle_search(app, key.code),
            Mode::Help => {
                if matches!(
                    key.code,
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?')
                ) {
                    app.mode = Mode::Normal;
                }
            }
            Mode::Confirm(_) => {
                let confirmed = matches!(key.code, KeyCode::Char('y') | KeyCode::Enter);
                let cancelled = matches!(
                    key.code,
                    KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q')
                );
                if confirmed {
                    app.mode = Mode::Normal;
                    if let Some((_, action)) = app.pending.take() {
                        run_action(terminal, app, action)?;
                    }
                } else if cancelled {
                    app.mode = Mode::Normal;
                    app.pending = None;
                    app.status = Some("cancelled".into());
                }
            }
        }
    }
}

fn handle_normal<B: Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    code: KeyCode,
    mods: KeyModifiers,
) -> Result<bool> {
    match (code, mods) {
        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(true),
        (KeyCode::Char('?'), _) => app.mode = Mode::Help,
        (KeyCode::Char('/'), _) | (KeyCode::Char('f'), _) => {
            app.mode = Mode::Search;
            app.pane = Pane::Inventory;
        }
        (KeyCode::Char(']'), _) => app.cycle_source_filter(true),
        (KeyCode::Char('['), _) => app.cycle_source_filter(false),
        (KeyCode::Char('s'), _) => {
            app.cycle_sort();
            app.status = Some(format!("sort: {}", app.sort.label()));
        }
        (KeyCode::Tab, _) | (KeyCode::Char('h'), _) | (KeyCode::Char('l'), _) => {
            app.pane = match app.pane {
                Pane::Lenses => Pane::Inventory,
                Pane::Inventory => Pane::Lenses,
            };
        }
        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => match app.pane {
            Pane::Lenses => {
                if app.lens_cursor + 1 < Lens::ORDER.len() {
                    app.lens_cursor += 1;
                    app.lens = Lens::ORDER[app.lens_cursor];
                    app.list_cursor = 0;
                }
            }
            Pane::Inventory => {
                let len = app.visible().len();
                if len > 0 && app.list_cursor + 1 < len {
                    app.list_cursor += 1;
                }
            }
        },
        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => match app.pane {
            Pane::Lenses => {
                if app.lens_cursor > 0 {
                    app.lens_cursor -= 1;
                    app.lens = Lens::ORDER[app.lens_cursor];
                    app.list_cursor = 0;
                }
            }
            Pane::Inventory => {
                if app.list_cursor > 0 {
                    app.list_cursor -= 1;
                }
            }
        },
        (KeyCode::Char('g'), _) => app.list_cursor = 0,
        (KeyCode::Char('G'), _) => {
            let len = app.visible().len();
            app.list_cursor = len.saturating_sub(1);
        }
        (KeyCode::Char('r'), _) => {
            rescan(terminal, app)?;
        }
        (KeyCode::Char('d'), _) => {
            let available = crate::probe::Available::detect();
            if let Some(item) = app.selected().cloned() {
                match actions::delete_for(&item, &available) {
                    Some(a) => app.queue_action(a),
                    None => {
                        app.status =
                            Some(format!("no delete path for source '{}'", item.source.label()));
                    }
                }
            }
        }
        (KeyCode::Char('u'), _) => {
            let available = crate::probe::Available::detect();
            if let Some(item) = app.selected().cloned() {
                match actions::update_for(&item, &available) {
                    Some(a) => app.queue_action(a),
                    None => {
                        app.status =
                            Some(format!("no update path for source '{}'", item.source.label()));
                    }
                }
            }
        }
        (KeyCode::Char('U'), _) => {
            let available = crate::probe::Available::detect();
            match actions::update_all(&available) {
                Some(a) => app.queue_action(a),
                None => app.status = Some("no installers available".into()),
            }
        }
        (KeyCode::Char('e'), _) => match crate::manifest::export_all(&app.snapshot) {
            Ok(paths) => {
                app.status = Some(format!("exported → {}", paths.join(", ")));
            }
            Err(e) => app.status = Some(format!("export error: {e}")),
        },
        (KeyCode::Esc, _) => {
            app.query.clear();
            app.clamp_cursor();
        }
        _ => {}
    }
    Ok(false)
}

fn handle_search(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Enter => {
            app.mode = Mode::Normal;
            app.clamp_cursor();
        }
        KeyCode::Backspace => {
            app.query.pop();
            app.list_cursor = 0;
        }
        KeyCode::Char(c) => {
            app.query.push(c);
            app.list_cursor = 0;
        }
        _ => {}
    }
}

fn rescan<B: Backend + io::Write>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    app.status = Some("rescanning…".into());
    terminal.draw(|f| crate::tui::ui::draw(f, app))?;
    let available = crate::probe::Available::detect();
    match crate::index::scan_all(&available) {
        Ok(snap) => {
            let _ = crate::index::save(&snap);
            app.snapshot = snap;
            app.clamp_cursor();
            app.status = Some(format!("{} items", app.snapshot.items.len()));
        }
        Err(e) => app.status = Some(format!("scan error: {e}")),
    }
    Ok(())
}

/// Drop out of the alt-screen, let the command stream to the user's terminal,
/// wait for a keypress, then restore the TUI and rescan.
fn run_action<B: Backend + io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    action: Action,
) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!("\n\x1b[36m▎\x1b[0m {}\n", action.title());
    io::stdout().flush().ok();

    let outcome = action.run();

    match &outcome {
        Ok(()) => println!("\n\x1b[32m✓\x1b[0m done"),
        Err(e) => println!("\n\x1b[31m✗\x1b[0m failed: {e}"),
    }
    println!("\n[press enter to return to Atlas]");
    let _ = io::stdin().read_line(&mut String::new());

    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;

    rescan(terminal, app)?;
    app.status = Some(match outcome {
        Ok(()) => format!("{}  ·  ok", action.title()),
        Err(e) => format!("{}  ·  {}", action.title(), e),
    });
    Ok(())
}
