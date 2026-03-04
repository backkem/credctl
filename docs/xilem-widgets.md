# Xilem Widget Reference (rev d29be95)

Quick reference for available widgets and styling in our pinned xilem version.

## Layout Containers

- `flex_col()` / `flex_row()` — flexible row/column layouts
- `grid()` / `grid_item()` / `grid_pos()` — grid layout
- `zstack()` / `zstack_item()` — layered views
- `portal()` — scrollable container
- `sized_box()` — force specific width/height
- `split()` — resizable split pane

## Text

- `label()` — static text (`.text_size()`, `.weight()`, `.text_alignment()`, `.font()`)
- `prose()` — selectable, wrappable text (`.text_color()`)
- `variable_label()` — animated font weight transitions

## Input Widgets

- `text_input(contents, on_changed)` — text field
  - `.placeholder()`, `.placeholder_color()`, `.text_color()`, `.caret_color()`
  - `.selection_color()`, `.text_size()`, `.text_alignment()`, `.weight()`, `.font()`
  - `.on_enter(cb)`, `.insert_newline()`, `.disabled()`, `.clip()`
- `button(child_view, cb)` / `text_button(label, cb)` — buttons
- `checkbox(label, checked, cb)` — boolean toggle
- `switch(bool, cb)` — on/off toggle
- `slider(min, max, value, cb)` — range selector

## Display

- `image()`, `progress_bar()`, `spinner()`, `canvas()` (custom vello drawing)
- `badged()` / `badged_optional()` — overlay badge on content

## Style Trait (method chaining on any widget)

**Sizing:** `.dims()`, `.width()`, `.height()`, `.padding()`, `.gap()`
**Colors:** `.color()`, `.background_color()`, `.active_background_color()`, `.disabled_color()`
**Borders:** `.border(color, width)`, `.border_color()`, `.border_width()`, `.corner_radius()`
**Hover/Focus:** `.hovered_border_color()`, `.focused_border_color()`
**Shadow:** `.box_shadow(BoxShadow)`
**Text:** `.line_break_mode()`

## WindowOptions

- `.with_initial_inner_size(S)`, `.with_min_inner_size(S)`, `.with_max_inner_size(S)`
- `.with_initial_position(P)` — for centering
- `.with_resizable(bool)`, `.with_decorations(bool)`, `.with_transparent(bool)`
- `.with_window_level(WindowLevel)` — Normal, AlwaysOnTop
- `.with_title_background_color(Color)` — Windows-specific title bar color
- `.with_title_text_color(Color)` — Windows-specific title bar text

No built-in dark mode toggle — must style manually. Window follows OS theme.

## FlexExt

- `.flex(weight)` — set flex grow on child
- `.must_fill(true)` — fill available space

## Relevant Types

- `Color` — `Color::new(r, g, b, a)` or `Color::from_rgba8(r, g, b, a)`
- `Length::px(f64)` — pixel lengths
- `InsertNewline::OnEnter` / `InsertNewline::Never` — multiline control
- `FontWeight` — `BOLD`, `NORMAL`, etc.
- `TextAlign` — `Start`, `Middle`, `End`, `Justified`
