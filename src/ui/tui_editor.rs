use std::io;
use std::io::Stdout;

use anyhow::Result;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use crossterm::execute;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use serde_json::Map;
use serde_json::Value;

use crate::store::SecretKind;
use crate::store::SecretValue;
use crate::ui::Cancelled;
use crate::ui::SecretEditor;

pub struct TuiEditor;

impl SecretEditor for TuiEditor {
    fn edit(&self, kind: &SecretKind, current: Option<SecretValue>) -> Result<SecretValue> {
        let app = App::new(kind, current);
        match run_app(app)? {
            Outcome::Saved(s) => Ok(SecretValue::new(s)),
            Outcome::Cancelled => Err(Cancelled.into()),
        }
    }
}

enum Outcome {
    Saved(String),
    Cancelled,
}

// Editor modes mirror SecretKind: plain string or key-value rows.
enum Mode {
    PlainText { content: String, masked: bool },
    Structured(Structured),
}

struct Structured {
    rows: Vec<Row>,
    selected: usize,
    editing: EditField,
    masked_all: bool,
}

struct Row {
    key: String,
    value: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditField {
    None,
    Key,
    Value,
}

struct App {
    mode: Mode,
}

impl App {
    fn new(kind: &SecretKind, current: Option<SecretValue>) -> Self {
        let mode = match kind {
            SecretKind::PlainText => {
                let content = current
                    .as_ref()
                    .map(|v| v.expose().to_owned())
                    .unwrap_or_default();
                Mode::PlainText {
                    content,
                    masked: true,
                }
            }
            SecretKind::Structured { keys } => {
                let existing: Map<String, Value> = current
                    .as_ref()
                    .and_then(|v| serde_json::from_str(v.expose()).ok())
                    .unwrap_or_default();

                let mut rows: Vec<Row> = Vec::new();
                // Schema keys first (preserve order), then any extras from the stored value.
                for k in keys {
                    let value = existing
                        .get(k)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_owned();
                    rows.push(Row {
                        key: k.clone(),
                        value,
                    });
                }
                for (k, v) in &existing {
                    if !keys.contains(k) {
                        rows.push(Row {
                            key: k.clone(),
                            value: v.as_str().unwrap_or("").to_owned(),
                        });
                    }
                }
                if rows.is_empty() {
                    rows.push(Row {
                        key: String::new(),
                        value: String::new(),
                    });
                }
                Mode::Structured(Structured {
                    rows,
                    selected: 0,
                    editing: EditField::None,
                    masked_all: true,
                })
            }
        };
        Self { mode }
    }
}

fn run_app(mut app: App) -> Result<Outcome> {
    let mut terminal = setup_terminal()?;
    let outcome = event_loop(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    outcome
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<Outcome> {
    loop {
        terminal.draw(|f| draw(f, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        // Global cancel.
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(Outcome::Cancelled);
        }

        if let Some(out) = handle_key(&mut app.mode, key) {
            return Ok(out);
        }
    }
}

fn handle_key(mode: &mut Mode, key: KeyEvent) -> Option<Outcome> {
    match mode {
        Mode::PlainText { content, masked } => handle_plain_key(content, masked, key),
        Mode::Structured(s) => handle_structured_key(s, key),
    }
}

fn handle_plain_key(content: &mut String, masked: &mut bool, key: KeyEvent) -> Option<Outcome> {
    match key.code {
        KeyCode::Esc => return Some(Outcome::Cancelled),
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Some(Outcome::Saved(content.clone()));
        }
        KeyCode::Tab => {
            *masked = !*masked;
        }
        KeyCode::Backspace => {
            content.pop();
        }
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            content.push(c);
        }
        _ => {}
    }
    None
}

fn handle_structured_key(s: &mut Structured, key: KeyEvent) -> Option<Outcome> {
    // While editing a field, all printable keys go to the buffer.
    if s.editing != EditField::None {
        let row = &mut s.rows[s.selected];
        let buf = match s.editing {
            EditField::Key => &mut row.key,
            EditField::Value => &mut row.value,
            EditField::None => unreachable!(),
        };
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Tab => s.editing = EditField::None,
            KeyCode::Backspace => {
                buf.pop();
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                buf.push(c);
            }
            _ => {}
        }
        return None;
    }

    match key.code {
        KeyCode::Esc => return Some(Outcome::Cancelled),
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Some(Outcome::Saved(serialize_rows(&s.rows)));
        }
        KeyCode::Char('m') => {
            s.masked_all = !s.masked_all;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if s.selected > 0 {
                s.selected -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if s.selected + 1 < s.rows.len() {
                s.selected += 1;
            }
        }
        KeyCode::Enter | KeyCode::Char('e') => s.editing = EditField::Value,
        KeyCode::Char('r') => s.editing = EditField::Key,
        KeyCode::Char('a') => {
            s.rows.push(Row {
                key: String::new(),
                value: String::new(),
            });
            s.selected = s.rows.len() - 1;
            s.editing = EditField::Key;
        }
        KeyCode::Char('d') => {
            s.rows.remove(s.selected);
            if s.rows.is_empty() {
                s.rows.push(Row {
                    key: String::new(),
                    value: String::new(),
                });
            }
            if s.selected >= s.rows.len() {
                s.selected = s.rows.len() - 1;
            }
        }
        _ => {}
    }
    None
}

fn serialize_rows(rows: &[Row]) -> String {
    let mut map = Map::new();
    for r in rows {
        if !r.key.is_empty() {
            map.insert(r.key.clone(), Value::String(r.value.clone()));
        }
    }
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}

// -- Rendering --

const ACCENT: Color = Color::Rgb(0x3b, 0x82, 0xf6);
const MUTED: Color = Color::Rgb(0x80, 0x80, 0x98);

fn draw(f: &mut Frame<'_>, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);

    draw_header(f, chunks[0]);
    match &app.mode {
        Mode::PlainText { content, masked } => draw_plain(f, chunks[1], content, *masked),
        Mode::Structured(s) => draw_structured(f, chunks[1], s),
    }
    draw_footer(f, chunks[2], &app.mode);
}

fn draw_header(f: &mut Frame<'_>, area: Rect) {
    let title = Paragraph::new(Line::from(Span::styled(
        " credctl - Edit Secret ",
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    )))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn draw_plain(f: &mut Frame<'_>, area: Rect, content: &str, masked: bool) {
    let display = if masked {
        if content.is_empty() {
            "(empty)".to_string()
        } else {
            "*".repeat(content.chars().count().min(64))
        }
    } else {
        content.to_string()
    };
    let title = if masked {
        " Value (masked) "
    } else {
        " Value "
    };
    let block = Block::default().borders(Borders::ALL).title(title);
    let style = if masked {
        Style::default().fg(MUTED)
    } else {
        Style::default()
    };
    f.render_widget(Paragraph::new(display).style(style).block(block), area);
}

fn draw_structured(f: &mut Frame<'_>, area: Rect, s: &Structured) {
    let block = Block::default().borders(Borders::ALL).title(" Variables ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::with_capacity(s.rows.len() + 1);
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {:<30} ", "KEY"),
            Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "VALUE",
            Style::default().fg(MUTED).add_modifier(Modifier::BOLD),
        ),
    ]));

    for (i, r) in s.rows.iter().enumerate() {
        let is_sel = i == s.selected;
        let marker = if is_sel { "> " } else { "  " };
        let key_str = if r.key.is_empty() {
            "(empty)".to_string()
        } else {
            r.key.clone()
        };
        let value_str = if s.masked_all && !r.value.is_empty() {
            "*".repeat(r.value.chars().count().min(24))
        } else if r.value.is_empty() {
            "(empty)".to_string()
        } else {
            r.value.clone()
        };

        let key_style = field_style(is_sel, s.editing == EditField::Key, false);
        let value_style = field_style(is_sel, s.editing == EditField::Value, s.masked_all);

        lines.push(Line::from(vec![
            Span::raw(marker),
            Span::styled(format!("{:<30} ", key_str), key_style),
            Span::styled(value_str, value_style),
        ]));
    }

    f.render_widget(Paragraph::new(lines), inner);
}

fn field_style(is_selected: bool, is_editing: bool, masked: bool) -> Style {
    if is_selected && is_editing {
        Style::default().fg(Color::Black).bg(ACCENT)
    } else if is_selected {
        Style::default().add_modifier(Modifier::BOLD)
    } else if masked {
        Style::default().fg(MUTED)
    } else {
        Style::default()
    }
}

fn draw_footer(f: &mut Frame<'_>, area: Rect, mode: &Mode) {
    let hint = match mode {
        Mode::PlainText { masked, .. } => {
            let m = if *masked { "show" } else { "hide" };
            format!(" type to edit | Tab {} | Ctrl+S save | Esc cancel", m)
        }
        Mode::Structured(s) if s.editing != EditField::None => {
            " typing... | Enter/Esc done ".to_string()
        }
        Mode::Structured(_) => " j/k move | e edit value | r rename key | a add | d delete | \
             m mask | Ctrl+S save | Esc cancel"
            .to_string(),
    };
    let p = Paragraph::new(Span::styled(hint, Style::default().fg(MUTED)))
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(p, area);
}
