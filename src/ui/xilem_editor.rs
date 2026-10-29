use std::sync::{Arc, Mutex};

use anyhow::Result;
use xilem::core::one_of::Either;
use xilem::masonry::layout::Length;
use xilem::masonry::parley::FontWeight;
use xilem::masonry::peniko::Color;
use xilem::style::Style as _;
use xilem::view::{FlexExt, flex_col, flex_row, label, portal, sized_box, text_button, text_input};
use xilem::winit::dpi::LogicalSize;
use xilem::{AppState, EventLoop, WidgetView, WindowId, WindowOptions, WindowView, Xilem, window};

use crate::store::{SecretKind, SecretValue};

use super::SecretEditor;

// -- Colors (dark theme) --

const BG_WINDOW: Color = Color::from_rgb8(0x1a, 0x1a, 0x2e);
const BG_HEADER: Color = Color::from_rgb8(0x22, 0x22, 0x3a);
const BG_ROW: Color = Color::from_rgb8(0x2a, 0x2a, 0x42);
const BG_INPUT: Color = Color::from_rgb8(0x1e, 0x1e, 0x36);
const TEXT_PRIMARY: Color = Color::from_rgb8(0xe0, 0xe0, 0xe8);
const TEXT_MUTED: Color = Color::from_rgb8(0x80, 0x80, 0x98);
const BORDER: Color = Color::from_rgb8(0x44, 0x44, 0x5a);
const ACCENT: Color = Color::from_rgb8(0x3b, 0x82, 0xf6);

/// Shared slot for the editor result. Written by the Save callback, read after run_in returns.
type ResultSlot = Arc<Mutex<Option<String>>>;

// ── App state wrapper ──
// Uses Xilem::new (not new_simple) so we can return an empty window list to exit.

struct EditorApp {
    window_id: WindowId,
    done: bool,
    warn_unsaved_row: bool,
    result: ResultSlot,
    inner: EditorInner,
    initial_height: f64,
}

enum EditorInner {
    PlainText { content: String, masked: bool },
    Json(JsonInner),
}

struct JsonInner {
    rows: Vec<KvRow>,
    new_key: String,
    new_value: String,
}

struct KvRow {
    key: String,
    value: String,
    masked: bool,
}

impl AppState for EditorApp {
    fn keep_running(&self) -> bool {
        !self.done
    }
}

impl EditorApp {
    fn has_pending_row(&self) -> bool {
        matches!(&self.inner, EditorInner::Json(j) if !j.new_key.is_empty())
    }

    fn add_pending_row(&mut self) {
        if let EditorInner::Json(j) = &mut self.inner
            && !j.new_key.is_empty() {
                j.rows.push(KvRow {
                    key: std::mem::take(&mut j.new_key),
                    value: std::mem::take(&mut j.new_value),
                    masked: true,
                });
            }
    }

    fn force_save(&mut self) {
        let value = match &self.inner {
            EditorInner::PlainText { content, .. } => content.clone(),
            EditorInner::Json(j) => {
                let mut map = serde_json::Map::new();
                for row in &j.rows {
                    if !row.key.is_empty() {
                        map.insert(
                            row.key.clone(),
                            serde_json::Value::String(row.value.clone()),
                        );
                    }
                }
                serde_json::to_string(&map).unwrap()
            }
        };
        *self.result.lock().unwrap() = Some(value);
        self.warn_unsaved_row = false;
        self.done = true;
    }

    fn save(&mut self) {
        if self.has_pending_row() {
            self.warn_unsaved_row = true;
            return;
        }
        self.force_save();
    }

    fn cancel(&mut self) {
        self.done = true;
    }
}

// ── Logic function (returns 0 or 1 windows) ──

fn app_logic(state: &mut EditorApp) -> Vec<WindowView<EditorApp>> {
    if state.done {
        return vec![];
    }

    let body: Box<xilem::AnyWidgetView<EditorApp>> = match &mut state.inner {
        EditorInner::PlainText { content, masked } => {
            plain_text_body(content.clone(), *masked).boxed()
        }
        EditorInner::Json(j) => json_body(j),
    };

    let header = header_bar();

    let warning: Option<Box<xilem::AnyWidgetView<EditorApp>>> = if state.warn_unsaved_row {
        Some(unsaved_row_warning().boxed())
    } else {
        None
    };

    let root = flex_col((header, warning, body.flex(1.0))).background_color(BG_WINDOW);

    let height = state.initial_height;
    let win = window(state.window_id, "credctl — Edit Secret", root)
        .with_options(move |_| {
            WindowOptions::new("credctl — Edit Secret")
                .with_initial_inner_size(LogicalSize::new(800.0, height))
                .with_min_inner_size(LogicalSize::new(500.0, 300.0))
        })
        .with_base_color(BG_WINDOW);

    vec![win]
}

// ── Header ──

fn header_bar() -> impl WidgetView<EditorApp> {
    let title = label("Editing")
        .text_size(14.0)
        .weight(FontWeight::BOLD)
        .color(ACCENT);

    let cancel_btn = text_button("Cancel", |s: &mut EditorApp| {
        s.cancel();
    })
    .border(BORDER, 1.0)
    .corner_radius(4.0);

    let save_btn = text_button("Save", |s: &mut EditorApp| {
        s.save();
    })
    .background_color(ACCENT)
    .corner_radius(4.0);

    flex_row((title.flex(1.0), cancel_btn, save_btn))
        .gap(Length::px(8.))
        .padding(12.0)
        .background_color(BG_HEADER)
}

// ── Unsaved row warning ──

const WARN_BG: Color = Color::from_rgb8(0x4a, 0x3a, 0x1a);
const WARN_TEXT: Color = Color::from_rgb8(0xff, 0xcc, 0x00);

fn unsaved_row_warning() -> impl WidgetView<EditorApp> {
    let msg = label("You have an unadded row in the new variable fields.")
        .text_size(13.0)
        .color(WARN_TEXT);

    let add_btn = text_button("Add & Save", |s: &mut EditorApp| {
        s.add_pending_row();
        s.force_save();
    })
    .background_color(ACCENT)
    .corner_radius(4.0);

    let discard_btn = text_button("Discard & Save", |s: &mut EditorApp| {
        if let EditorInner::Json(j) = &mut s.inner {
            j.new_key.clear();
            j.new_value.clear();
        }
        s.force_save();
    })
    .border(BORDER, 1.0)
    .corner_radius(4.0);

    let dismiss_btn = text_button("Go Back", |s: &mut EditorApp| {
        s.warn_unsaved_row = false;
    })
    .border(BORDER, 1.0)
    .corner_radius(4.0);

    flex_row((msg.flex(1.0), add_btn, discard_btn, dismiss_btn))
        .gap(Length::px(8.))
        .padding(12.0)
        .background_color(WARN_BG)
}

// ── Plain text body ──

fn masked_label(value: &str) -> &str {
    if value.is_empty() {
        "(empty)"
    } else {
        "••••••••"
    }
}

fn plain_text_body(content: String, masked: bool) -> impl WidgetView<EditorApp> {
    let value_display = if masked {
        Either::A(
            label(masked_label(&content))
                .text_size(14.0)
                .color(TEXT_MUTED)
                .border(BORDER, 1.0)
                .corner_radius(4.0)
                .background_color(BG_INPUT)
                .padding(8.0),
        )
    } else {
        Either::B(
            text_input(content, |s: &mut EditorApp, t| {
                if let EditorInner::PlainText { content, .. } = &mut s.inner {
                    *content = t;
                }
            })
            .placeholder("Enter secret value…")
            .text_color(TEXT_PRIMARY)
            .text_size(14.0)
            .border(BORDER, 1.0)
            .corner_radius(4.0)
            .background_color(BG_INPUT),
        )
    };

    let editor = portal(value_display)
        .constrain_horizontal(true)
        .must_fill(true);

    let toggle_icon = if masked { "👁" } else { "👁\u{200d}🗨" };
    let toggle_btn = text_button(toggle_icon, |s: &mut EditorApp| {
        if let EditorInner::PlainText { masked, .. } = &mut s.inner {
            *masked = !*masked;
        }
    })
    .border(BORDER, 1.0)
    .corner_radius(4.0);

    let header = flex_row((
        label("Value").text_size(12.0).color(TEXT_MUTED).flex(1.0),
        toggle_btn,
    ))
    .gap(Length::px(8.));

    flex_col((header, editor)).gap(Length::px(6.)).padding(16.0)
}

// ── JSON body ──

fn json_body(j: &mut JsonInner) -> Box<xilem::AnyWidgetView<EditorApp>> {
    let col_headers = flex_row((
        sized_box(label("Name").text_size(12.0).color(TEXT_MUTED)).fixed_width(Length::px(250.0)),
        label("Value").text_size(12.0).color(TEXT_MUTED),
    ))
    .gap(Length::px(12.))
    .padding(12.0);

    let rows: Vec<Box<xilem::AnyWidgetView<EditorApp>>> = j
        .rows
        .iter()
        .enumerate()
        .map(|(i, row)| kv_row(i, row.key.clone(), row.value.clone(), row.masked))
        .collect();

    let scrollable = portal(flex_col(rows).gap(Length::px(1.0)))
        .constrain_horizontal(true)
        .must_fill(true);

    let new_key = j.new_key.clone();
    let new_value = j.new_value.clone();
    let new_row = new_kv_row(new_key, new_value);

    flex_col((col_headers, scrollable.flex(1.0), new_row)).boxed()
}

fn kv_row(
    idx: usize,
    key: String,
    value: String,
    masked: bool,
) -> Box<xilem::AnyWidgetView<EditorApp>> {
    let key_input = sized_box(
        text_input(key, move |s: &mut EditorApp, t| {
            if let EditorInner::Json(j) = &mut s.inner {
                j.rows[idx].key = t;
            }
        })
        .text_color(TEXT_PRIMARY)
        .text_size(14.0)
        .border(BORDER, 1.0)
        .corner_radius(4.0)
        .background_color(BG_INPUT),
    )
    .fixed_width(Length::px(250.0));

    let value_display = if masked {
        Either::A(
            label(masked_label(&value))
                .text_size(14.0)
                .color(TEXT_MUTED)
                .border(BORDER, 1.0)
                .corner_radius(4.0)
                .background_color(BG_INPUT)
                .padding(8.0),
        )
    } else {
        Either::B(
            text_input(value, move |s: &mut EditorApp, t| {
                if let EditorInner::Json(j) = &mut s.inner {
                    j.rows[idx].value = t;
                }
            })
            .text_color(TEXT_PRIMARY)
            .text_size(14.0)
            .border(BORDER, 1.0)
            .corner_radius(4.0)
            .background_color(BG_INPUT),
        )
    };

    let toggle_icon = if masked { "👁" } else { "👁\u{200d}🗨" };
    let mask_btn = text_button(toggle_icon, move |s: &mut EditorApp| {
        if let EditorInner::Json(j) = &mut s.inner {
            j.rows[idx].masked = !j.rows[idx].masked;
        }
    })
    .border(BORDER, 1.0)
    .corner_radius(4.0);

    let remove_btn = text_button("❌", move |s: &mut EditorApp| {
        if let EditorInner::Json(j) = &mut s.inner {
            j.rows.remove(idx);
        }
    })
    .border(BORDER, 1.0)
    .corner_radius(4.0);

    flex_row((key_input, value_display.flex(1.0), mask_btn, remove_btn))
        .gap(Length::px(12.))
        .padding(12.0)
        .background_color(BG_ROW)
        .boxed()
}

fn new_kv_row(new_key: String, new_value: String) -> impl WidgetView<EditorApp> {
    let key_input = sized_box(
        text_input(new_key, |s: &mut EditorApp, t| {
            if let EditorInner::Json(j) = &mut s.inner {
                j.new_key = t;
            }
        })
        .placeholder("VARIABLE_NAME")
        .text_color(TEXT_PRIMARY)
        .text_size(14.0)
        .border(BORDER, 1.0)
        .corner_radius(4.0)
        .background_color(BG_INPUT),
    )
    .fixed_width(Length::px(250.0));

    let value_input = text_input(new_value, |s: &mut EditorApp, t| {
        if let EditorInner::Json(j) = &mut s.inner {
            j.new_value = t;
        }
    })
    .placeholder("VARIABLE_VALUE")
    .text_color(TEXT_PRIMARY)
    .text_size(14.0)
    .border(BORDER, 1.0)
    .corner_radius(4.0)
    .background_color(BG_INPUT);

    let add_btn = text_button("➕", |s: &mut EditorApp| {
        if let EditorInner::Json(j) = &mut s.inner
            && !j.new_key.is_empty() {
                j.rows.push(KvRow {
                    key: std::mem::take(&mut j.new_key),
                    value: std::mem::take(&mut j.new_value),
                    masked: true,
                });
            }
    })
    .border(BORDER, 1.0)
    .corner_radius(4.0);

    flex_row((key_input, value_input.flex(1.0), add_btn))
        .gap(Length::px(12.))
        .padding(12.0)
}

// ── Public API ──

pub struct XilemEditor;

impl SecretEditor for XilemEditor {
    fn edit(&self, kind: &SecretKind, current: Option<SecretValue>) -> Result<SecretValue> {
        // Xilem creates its own tokio runtime. If we're called from within an
        // existing runtime (e.g. #[tokio::main]), we need block_in_place to
        // allow the nested runtime to drop cleanly on exit.
        if tokio::runtime::Handle::try_current().is_ok() {
            return tokio::task::block_in_place(|| self.run_editor(kind, current));
        }
        self.run_editor(kind, current)
    }
}

impl XilemEditor {
    fn run_editor(&self, kind: &SecretKind, current: Option<SecretValue>) -> Result<SecretValue> {
        let result: ResultSlot = Arc::new(Mutex::new(None));

        let inner = match kind {
            SecretKind::PlainText => EditorInner::PlainText {
                content: current.map(|v| v.expose().to_owned()).unwrap_or_default(),
                masked: true,
            },
            SecretKind::Structured { keys } => {
                let existing: serde_json::Map<String, serde_json::Value> = current
                    .and_then(|v| serde_json::from_str(v.expose()).ok())
                    .unwrap_or_default();

                let mut rows: Vec<KvRow> = Vec::new();
                for k in keys {
                    let value = existing
                        .get(k)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_owned();
                    rows.push(KvRow {
                        key: k.clone(),
                        value,
                        masked: true,
                    });
                }
                for (k, v) in &existing {
                    if !keys.contains(k) {
                        rows.push(KvRow {
                            key: k.clone(),
                            value: v.as_str().unwrap_or("").to_owned(),
                            masked: true,
                        });
                    }
                }

                EditorInner::Json(JsonInner {
                    rows,
                    new_key: String::new(),
                    new_value: String::new(),
                })
            }
        };

        let initial_height = match &inner {
            EditorInner::PlainText { .. } => 300.0,
            EditorInner::Json(j) => (200.0 + j.rows.len() as f64 * 52.0).clamp(300.0, 800.0),
        };

        let state = EditorApp {
            window_id: WindowId::next(),
            done: false,
            warn_unsaved_row: false,
            result: result.clone(),
            inner,
            initial_height,
        };

        let _ = Xilem::new(state, |s: &mut EditorApp| app_logic(s).into_iter())
            .run_in(EventLoop::with_user_event());

        // Extract result — Arc should be the sole owner now
        let value = Arc::try_unwrap(result)
            .expect("result arc should have single owner")
            .into_inner()
            .unwrap();

        match value {
            Some(s) => Ok(SecretValue::new(s)),
            None => Err(super::Cancelled)?,
        }
    }
}
