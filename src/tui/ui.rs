use crate::lenses::Lens;
use crate::model::{Arch, Kind, SoftwareItem, Source, Status};
use crate::tui::app::{App, ConfirmPrompt, Mode, Pane, SortMode};
use chrono::{Duration as ChronoDuration, Utc};
use humansize::{format_size, BINARY};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

const ACCENT: Color = Color::Cyan;
const MUTED: Color = Color::DarkGray;
const DIM: Color = Color::Gray;
const WARN: Color = Color::Yellow;
const BAD: Color = Color::LightRed;
const GOOD: Color = Color::LightGreen;
const SELECT_BAR: &str = "▎ ";
const UNFOCUS_PAD: &str = "  ";

pub fn draw(f: &mut Frame, app: &mut App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());

    draw_title(f, root[0], app);
    draw_body(f, root[1], app);
    draw_help(f, root[2], app);
    draw_status(f, root[3], app);

    match &app.mode {
        Mode::Help => draw_help_modal(f),
        Mode::Confirm(c) => draw_confirm_modal(f, c),
        _ => {}
    }
}

fn draw_title(f: &mut Frame, area: Rect, app: &App) {
    let outdated = app
        .snapshot
        .items
        .iter()
        .filter(|i| i.is_outdated())
        .count();
    let installers = {
        let a = &app.snapshot.available;
        let mut on = vec![];
        if a.zb {
            on.push("zb");
        }
        if a.brew {
            on.push("brew");
        }
        if a.mas {
            on.push("mas");
        }
        if a.nix {
            on.push("nix");
        }
        if on.is_empty() {
            "none".into()
        } else {
            on.join(" · ")
        }
    };

    let mut spans = vec![
        Span::styled("atlas", Style::default().fg(ACCENT).bold()),
        Span::styled("  ·  ", Style::default().fg(MUTED)),
        Span::styled(format!("{} items", app.snapshot.items.len()), Style::default().fg(DIM)),
        Span::styled("  ·  ", Style::default().fg(MUTED)),
        Span::styled(installers, Style::default().fg(DIM)),
    ];
    if let Some(s) = app.source_filter {
        spans.push(Span::styled("  ·  ", Style::default().fg(MUTED)));
        spans.push(Span::styled(
            format!("filter: {}", s.label()),
            Style::default().fg(ACCENT).italic(),
        ));
    }
    if outdated > 0 {
        spans.push(Span::styled("  ·  ", Style::default().fg(MUTED)));
        spans.push(Span::styled(
            format!("{} outdated", outdated),
            Style::default().fg(WARN),
        ));
    }
    if app.sort != SortMode::None {
        spans.push(Span::styled("  ·  ", Style::default().fg(MUTED)));
        spans.push(Span::styled(
            format!("sort: {}", app.sort.label()),
            Style::default().fg(ACCENT).italic(),
        ));
    }
    f.render_widget(Line::from(spans), area);
}

fn draw_body(f: &mut Frame, area: Rect, app: &mut App) {
    // Responsive: drop details pane when width is tight.
    let show_details = area.width >= 110;
    let constraints: Vec<Constraint> = if show_details {
        vec![
            Constraint::Length(20),
            Constraint::Min(40),
            Constraint::Length(42),
        ]
    } else {
        vec![Constraint::Length(20), Constraint::Min(30)]
    };
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    draw_lenses(f, cols[0], app);
    draw_inventory(f, cols[1], app);
    if show_details {
        draw_details(f, cols[2], app);
    }
}

fn rounded_block(title: &str, focused: bool) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style(focused))
        .title(Span::styled(
            format!(" {} ", title.trim()),
            Style::default().fg(if focused { ACCENT } else { DIM }),
        ))
}

fn draw_lenses(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.pane == Pane::Lenses;
    let items: Vec<ListItem> = Lens::ORDER
        .iter()
        .enumerate()
        .map(|(i, lens)| {
            let count = crate::lenses::apply(*lens, &app.snapshot.items).len();
            let selected = i == app.lens_cursor;
            let prefix = if selected && focused {
                Span::styled(SELECT_BAR, Style::default().fg(ACCENT))
            } else {
                Span::raw(UNFOCUS_PAD)
            };
            let title_style = if selected {
                Style::default().fg(if focused { ACCENT } else { DIM }).bold()
            } else {
                Style::default().fg(DIM)
            };
            let count_style = Style::default().fg(MUTED);
            ListItem::new(Line::from(vec![
                prefix,
                Span::styled(format!("{:<10}", lens.title()), title_style),
                Span::styled(format!("{:>4}", count), count_style),
            ]))
        })
        .collect();

    let list = List::new(items).block(rounded_block("Lenses", focused));
    f.render_widget(list, area);
}

fn draw_inventory(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.pane == Pane::Inventory;
    let title = format!("{} · {}", app.lens.title(), app.lens.hint());
    let block = rounded_block(&title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    let search_line = if app.mode == Mode::Search {
        Line::from(vec![
            Span::styled("/ ", Style::default().fg(ACCENT).bold()),
            Span::styled(&app.query, Style::default().fg(Color::White)),
            Span::styled(
                "▏",
                Style::default().fg(ACCENT).add_modifier(Modifier::SLOW_BLINK),
            ),
        ])
    } else if !app.query.is_empty() {
        Line::from(vec![
            Span::styled("/ ", Style::default().fg(MUTED)),
            Span::styled(&app.query, Style::default().fg(DIM)),
            Span::styled("   esc clears", Style::default().fg(MUTED).italic()),
        ])
    } else {
        Line::from(Span::styled(
            format!(
                "press / or f to search · [ ] cycle installer (now: {})",
                app.source_filter_label()
            ),
            Style::default().fg(MUTED).italic(),
        ))
    };
    f.render_widget(Paragraph::new(search_line), split[0]);

    let visible = app.visible();
    let rows: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(i, item)| render_row(item, i == app.list_cursor, focused))
        .collect();

    let mut state = ListState::default();
    if !visible.is_empty() {
        state.select(Some(app.list_cursor.min(visible.len() - 1)));
    }
    let list = List::new(rows);
    f.render_stateful_widget(list, split[1], &mut state);
}

fn render_row(item: &SoftwareItem, selected: bool, focused: bool) -> ListItem<'static> {
    let prefix: Span = if selected && focused {
        Span::styled(SELECT_BAR, Style::default().fg(ACCENT))
    } else {
        Span::raw(UNFOCUS_PAD)
    };

    let name_style = if selected {
        Style::default()
            .fg(if focused { Color::White } else { DIM })
            .bold()
    } else {
        Style::default().fg(DIM)
    };

    let source_span = Span::styled(
        format!("{:>6}", item.source.label()),
        Style::default().fg(source_color(item.source)),
    );
    let kind_span = Span::styled(
        format!("  {:<7}", item.kind.label()),
        Style::default().fg(MUTED),
    );
    let name_span = Span::styled(pad_display(&item.name, 30), name_style);

    // Strict columns: name 30, version 14, arch 6, size 10 — no matter what.
    let version = item.version.as_deref().unwrap_or("—");
    let size_str = item
        .size_bytes
        .map(|b| format_size(b, BINARY))
        .unwrap_or_default();
    let arch_lbl = match item.arch {
        Arch::Unknown => "",
        a => a.label(),
    };

    let meta = Span::styled(
        format!(
            "  {:<14} {:>5}  {:>10}",
            truncate(version, 14),
            arch_lbl,
            size_str
        ),
        Style::default().fg(MUTED),
    );

    let status_span = match item.status {
        Status::Outdated => Span::styled("  ↑", Style::default().fg(WARN)),
        Status::Running => Span::styled("  ●", Style::default().fg(GOOD)),
        Status::Broken => Span::styled("  ✗", Style::default().fg(BAD)),
        _ => Span::raw("   "),
    };
    let arch_warn: Span = if item.arch == Arch::X86_64 {
        Span::styled("  rosetta", Style::default().fg(BAD).italic())
    } else {
        Span::raw("")
    };

    ListItem::new(Line::from(vec![
        prefix,
        source_span,
        kind_span,
        name_span,
        meta,
        status_span,
        arch_warn,
    ]))
}

fn draw_details(f: &mut Frame, area: Rect, app: &App) {
    let block = rounded_block("Details", false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(item) = app.selected() else {
        f.render_widget(
            Paragraph::new(Span::styled(
                "nothing selected",
                Style::default().fg(MUTED).italic(),
            )),
            inner,
        );
        return;
    };

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            item.name.clone(),
            Style::default().fg(Color::White).bold(),
        )),
        Line::raw(""),
    ];

    if let Some(b) = &item.bundle_id {
        lines.push(kv("bundle", b));
    }
    lines.push(kv("source", item.source.label()));
    lines.push(kv("kind", item.kind.label()));
    if let Some(v) = &item.version {
        lines.push(kv("version", v));
    }
    if let Some(lv) = &item.latest_version {
        lines.push(kv("latest", lv));
    }
    if let Some(p) = &item.install_path {
        lines.push(kv("path", &p.display().to_string()));
    }
    if let Some(s) = item.size_bytes {
        lines.push(kv("size", &format_size(s, BINARY)));
    }
    if item.arch != Arch::Unknown {
        lines.push(kv("arch", item.arch.label()));
    }
    if let Some(t) = item.last_used {
        lines.push(kv("last used", &human_ago(t)));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "insights",
        Style::default().fg(ACCENT),
    )));
    for insight in item_insights(item) {
        lines.push(Line::from(vec![
            Span::styled("·  ", Style::default().fg(MUTED)),
            Span::styled(insight, Style::default().fg(DIM)),
        ]));
    }

    f.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn item_insights(item: &SoftwareItem) -> Vec<String> {
    let mut out = vec![];
    if item.arch == Arch::X86_64 {
        out.push("runs under Rosetta 2 — check for an arm64 build".into());
    }
    if item.is_outdated() {
        match (&item.version, &item.latest_version) {
            (Some(v), Some(lv)) => out.push(format!("upgrade available: {} → {}", v, lv)),
            _ => out.push("upgrade available".into()),
        }
    }
    if let Some(t) = item.last_used {
        let days = (Utc::now() - t).num_days();
        if days >= 90 {
            out.push(format!("not opened in {} days", days));
        }
    }
    if item.signed == Some(false) {
        out.push("unsigned — provenance unclear".into());
    }
    if item.kind == Kind::App && item.size_bytes.unwrap_or(0) > 2_000_000_000 {
        out.push("over 2 GiB on disk".into());
    }
    if out.is_empty() {
        out.push("nothing notable".into());
    }
    out
}

fn kv(key: &str, val: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{:<11}", key), Style::default().fg(MUTED)),
        Span::styled(val.to_string(), Style::default().fg(DIM)),
    ])
}

fn draw_help(f: &mut Frame, area: Rect, app: &App) {
    let hints: &[(&str, &str)] = match app.mode {
        Mode::Search => &[("esc", "done"), ("⌫", "char")],
        Mode::Confirm(_) => &[("y/⏎", "confirm"), ("n/esc", "cancel")],
        Mode::Help => &[("esc", "close")],
        _ => &[
            ("j/k", "nav"),
            ("h/l", "pane"),
            ("/ f", "search"),
            ("[ ]", "installer"),
            ("s", "sort"),
            ("u", "update"),
            ("U", "update all"),
            ("d", "delete"),
            ("r", "rescan"),
            ("e", "export"),
            ("?", "help"),
            ("q", "quit"),
        ],
    };
    let mut spans = vec![];
    for (k, v) in hints {
        spans.push(Span::styled(
            (*k).to_string(),
            Style::default().fg(ACCENT).bold(),
        ));
        spans.push(Span::styled(
            format!(" {}   ", v),
            Style::default().fg(MUTED),
        ));
    }
    f.render_widget(Line::from(spans), area);
}

fn draw_status(f: &mut Frame, area: Rect, app: &App) {
    let text = app.status.clone().unwrap_or_else(|| {
        format!(
            "{} visible · scanned {}",
            app.visible().len(),
            human_ago(app.snapshot.generated_at)
        )
    });
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            text,
            Style::default().fg(MUTED).italic(),
        ))),
        area,
    );
}

fn draw_help_modal(f: &mut Frame) {
    let area = centered_rect(64, 72, f.area());
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(" Help ", Style::default().fg(ACCENT)))
        .border_style(Style::default().fg(ACCENT));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let section = |t: &str| {
        Line::from(Span::styled(
            t.to_string(),
            Style::default().fg(ACCENT).italic(),
        ))
    };
    let row = |k: &str, v: &str| {
        Line::from(vec![
            Span::styled(format!("  {:<10}", k), Style::default().fg(DIM).bold()),
            Span::styled(v.to_string(), Style::default().fg(MUTED)),
        ])
    };

    let lines = vec![
        section("navigation"),
        row("j / k", "down / up"),
        row("g / G", "top / bottom"),
        row("h / l / tab", "switch pane"),
        Line::raw(""),
        section("filter & sort"),
        row("/ or f", "search name / bundle id"),
        row("] [", "cycle installer (all → brew → zb → mas → manual)"),
        row("s", "cycle sort: biggest → longest-ago → recent → most-used → least-used"),
        row("esc", "exit search / clear filter"),
        Line::raw(""),
        section("actions (owner-aware)"),
        row("u", "update selected via its installer"),
        row("U", "update everything (zb + brew + mas)"),
        row("d", "delete selected via its installer (or Trash for .app)"),
        row("r", "rescan machine"),
        row("e", "export manifest + Brewfile + mas list"),
        Line::raw(""),
        section("lenses"),
        row("All", "everything on disk"),
        row("Outdated", "items with a known upgrade"),
        row("Duplicates", "same tool from multiple installers"),
        row("Bloat", "heaviest 50 on disk"),
        row("Stale", ".apps unused for 90+ days"),
        row("Rosetta", "x86_64-only on Apple Silicon"),
        row("Unsigned", "missing code signature"),
        Line::raw(""),
        section("other"),
        row("? / q", "toggle help / quit"),
    ];
    f.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn draw_confirm_modal(f: &mut Frame, c: &ConfirmPrompt) {
    let width = 72.min(f.area().width.saturating_sub(4));
    let height = (6 + c.lines.len() as u16).min(f.area().height.saturating_sub(4));
    let area = centered_fixed_rect(width, height, f.area());
    f.render_widget(Clear, area);
    let tone = if c.destructive { BAD } else { ACCENT };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(Span::styled(
            format!(" {} ", c.title),
            Style::default().fg(tone).bold(),
        ))
        .border_style(Style::default().fg(tone));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = vec![Line::raw("")];
    for cmd in &c.lines {
        lines.push(Line::from(vec![
            Span::styled("  $ ", Style::default().fg(MUTED)),
            Span::styled(cmd.clone(), Style::default().fg(Color::White).bold()),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(
            "  y / ⏎",
            Style::default().fg(GOOD).bold(),
        ),
        Span::styled(" confirm    ", Style::default().fg(MUTED)),
        Span::styled("n / esc", Style::default().fg(DIM).bold()),
        Span::styled(" cancel", Style::default().fg(MUTED)),
    ]));

    f.render_widget(
        Paragraph::new(Text::from(lines))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn centered_rect(pct_x: u16, pct_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - pct_y) / 2),
            Constraint::Percentage(pct_y),
            Constraint::Percentage((100 - pct_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - pct_x) / 2),
            Constraint::Percentage(pct_x),
            Constraint::Percentage((100 - pct_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn centered_fixed_rect(w: u16, h: u16, area: Rect) -> Rect {
    let x_pad = area.width.saturating_sub(w) / 2;
    let y_pad = area.height.saturating_sub(h) / 2;
    Rect {
        x: area.x + x_pad,
        y: area.y + y_pad,
        width: w,
        height: h,
    }
}

fn border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(MUTED)
    }
}

fn source_color(s: Source) -> Color {
    match s {
        Source::Zerobrew => Color::LightMagenta,
        Source::Brew => Color::LightYellow,
        Source::AppStore => ACCENT,
        Source::Nix => Color::LightBlue,
        Source::Pkg => Color::LightRed,
        Source::Manual => DIM,
        _ => MUTED,
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}

/// Truncate to `width` grapheme-ish columns, then right-pad with spaces so the
/// next span always starts at the same x-coordinate across rows.
fn pad_display(s: &str, width: usize) -> String {
    let truncated = truncate(s, width);
    let n = truncated.chars().count();
    let mut out = String::with_capacity(width);
    out.push_str(&truncated);
    for _ in n..width {
        out.push(' ');
    }
    out
}

fn human_ago(t: chrono::DateTime<Utc>) -> String {
    let d = Utc::now() - t;
    if d < ChronoDuration::minutes(1) {
        return "just now".into();
    }
    if d < ChronoDuration::hours(1) {
        return format!("{}m ago", d.num_minutes());
    }
    if d < ChronoDuration::days(1) {
        return format!("{}h ago", d.num_hours());
    }
    if d < ChronoDuration::days(30) {
        return format!("{}d ago", d.num_days());
    }
    t.format("%Y-%m-%d").to_string()
}
