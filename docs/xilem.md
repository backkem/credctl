# Xilem Integration Notes

Working notes from integrating xilem (commit d29be95) as the editor UI for credctl.

## Local checkout

Xilem is pinned via git dep (`github.com/backkem/xilem`, branch `keep-running-after-action`).

## Cargo.toml

```toml
xilem = { git = "https://github.com/backkem/xilem", branch = "keep-running-after-action", package = "xilem" }
```

## Key types and imports

```rust
use xilem::{Xilem, EventLoop, WidgetView, WindowOptions, WindowView, WindowId, AppState};
use xilem::view::{
    FlexExt, flex_col, flex_row, label, portal, sized_box,
    text_button, text_input, checkbox, switch, slider,
    button, prose, grid, zstack, image, canvas,
    progress_bar, spinner, variable_label,
};
use xilem::style::Style as _;
use xilem::masonry::peniko::Color;        // Color = AlphaColor<Srgb>
use xilem::masonry::layout::Length;
use xilem::masonry::parley::FontWeight;
use xilem::winit::dpi::LogicalSize;
use xilem::AnyWidgetView;                 // type alias for dyn AnyView (NOT a trait)
use xilem::window;                        // fn window(id, title, root_view) -> WindowView
```

## Color

`Color` is `peniko::Color` which is `color::AlphaColor<color::Srgb>`.

```rust
const MY_COLOR: Color = Color::from_rgb8(0x1a, 0x1a, 0x2e);  // const-friendly
```

## Method chaining order (CRITICAL)

Widget-specific methods return `Self`. Style trait methods return `Prop<...>` wrapper.
Once you call a Style method, you can only call more Style methods — NOT widget methods.

**Always call widget-specific methods FIRST, then Style methods LAST.**

### text_input — widget methods (return Self):
- `.placeholder("...")`, `.text_color(color)`, `.caret_color(color)`, `.selection_color(color)`
- `.text_size(f32)`, `.weight(FontWeight)`, `.font(FontStack)`
- `.text_alignment(TextAlign)`, `.on_enter(cb)`, `.insert_newline(InsertNewline)`
- `.disabled(bool)`, `.clip(bool)`

### text_input — style methods (return Prop, call LAST):
- `.placeholder_color(color)` — this is a Style method, NOT a widget method!
- `.border(color, width)`, `.corner_radius(f64)`, `.background_color(color)`
- `.color(color)`, `.padding(f64)`, `.box_shadow(...)`, etc.

### label — widget methods (return Self):
- `.text_size(f32)`, `.weight(FontWeight)`, `.text_alignment(TextAlign)`, `.font(FontStack)`

### label — style methods (return Prop):
- `.color(color)` — sets text color via Style

### Correct example:
```rust
text_input(content, callback)
    .placeholder("hint")          // widget method
    .text_color(TEXT_PRIMARY)     // widget method
    .text_size(14.0)              // widget method
    .border(BORDER, 1.0)         // Style method — from here on, only Style
    .corner_radius(4.0)          // Style method
    .background_color(BG_INPUT)  // Style method
```

### WRONG — will not compile:
```rust
text_input(content, callback)
    .placeholder_color(MUTED)     // Style method (returns Prop)
    .text_color(PRIMARY)          // FAIL: widget method not found on Prop
```

## sized_box

```rust
sized_box(child_view)
    .fixed_width(Length::px(250.0))
    .fixed_height(Length::px(50.0))
```

No `must_fill_major_axis` — use `portal(...).must_fill(true)` for fill behavior.
`Length::px(f64)` — NOT `Length::new()`.

## Layout

- `flex_col((...))` / `flex_row((...))` — tuples or Vec
- `.flex(1.0)` on a child (via `FlexExt`) to make it grow
- `.gap(Length::px(8.))` for spacing
- `.padding(12.0)` takes `f64`, NOT `Length`
- `portal(child)` for scrollable containers
  - `.constrain_horizontal(true)` — fit to parent width
  - `.must_fill(true)` — fill available space

## Type-erased views (AnyWidgetView)

`AnyWidgetView<State>` is a **type alias** (`dyn AnyView<...>`), not a trait.
Use `Box<AnyWidgetView<State>>` (NOT `Box<dyn AnyWidgetView<...>>`).

```rust
// Creating boxed views:
let view: Box<xilem::AnyWidgetView<MyState>> = my_view.boxed();

// Vec of heterogeneous views:
let rows: Vec<Box<xilem::AnyWidgetView<MyState>>> = items
    .iter()
    .map(|item| make_row(item).boxed())
    .collect();

// flex_col accepts Vec<Box<AnyWidgetView<S>>>
flex_col(rows)
```

## Two app patterns

### new_simple — single window, auto-exit on close

```rust
let app = Xilem::new_simple(state, logic_fn, window_options);
app.run_in(EventLoop::with_user_event())?;
```

- Wraps state in `ExitOnClose { state, running: bool }`
- Automatically exits event loop when window is closed (X button)
- `logic_fn` receives `&mut State` (unwrapped)
- State is consumed — cannot read back after run_in

### new — dynamic windows, custom exit control (what we use)

```rust
impl AppState for MyState {
    fn keep_running(&self) -> bool { !self.done }
}

let app = Xilem::new(state, |s: &mut MyState| app_logic(s).into_iter());
app.run_in(EventLoop::with_user_event())?;
```

- You control window lifecycle by returning different windows from logic fn
- Return empty vec → all windows close
- `keep_running()` checked after close requests AND after actions (with our patch)
- Use `WindowId::next()` to create stable window IDs (store in state)

## The keep_running patch (in our local xilem)

**Problem:** `AppState::keep_running()` was only checked after `on_close_requested` (X button).
Button clicks that set `done = true` would close windows (via empty return from logic fn)
but the event loop would hang because `event_loop.exit()` was never called.

**Fix:** In `xilem/src/driver.rs`, `handle_message_result`, after `run_logic`:
```rust
MessageResult::Action(()) => {
    self.run_logic(masonry_ctx);
    // Added: check if the app wants to exit after this state mutation.
    if !self.state.keep_running() {
        masonry_ctx.exit();
    }
}
```

This resolves the existing TODO in `app.rs`:
```rust
// Was: "Is currently only checked after a close request."
// Now: "Checked after every state mutation (actions, close requests, etc.)."
```

**Upstreamable** — the maintainers already had a TODO for this exact change.

## Tokio runtime nesting

Xilem creates its own internal tokio runtime (`Xilem::new` → `TokioRuntime::new()`).
If the caller is already inside a `#[tokio::main]` runtime, dropping xilem's runtime panics:
`"Cannot drop a runtime in a context where blocking is not allowed."`

**Fix in XilemEditor** (not in the trait — this is xilem-specific):
```rust
fn edit(&self, kind, current) -> Result<SecretValue> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return tokio::task::block_in_place(|| self.run_editor(kind, current));
    }
    self.run_editor(kind, current)
}
```

## Returning values from xilem

`run_in()` consumes `self` (including state) and returns `Result<(), EventLoopError>`.
No way to get state back.

**Solution:** `Arc<Mutex<Option<String>>>` shared between state and caller.
- Save callback writes to Arc, sets `done = true`
- Cancel just sets `done = true`
- After `run_in` returns, caller reads from Arc

```rust
type ResultSlot = Arc<Mutex<Option<String>>>;
```

## WindowOptions

```rust
WindowOptions::new("title")
    .with_initial_inner_size(LogicalSize::new(800.0, 500.0))
    .with_min_inner_size(LogicalSize::new(500.0, 300.0))
    .with_resizable(true)
    .with_decorations(true)
```

When using `Xilem::new`, set options via `window(...).with_options(|_| opts)`.

No built-in dark mode. No `with_initial_position` for centering (winit doesn't center by default;
OS typically centers new windows on Windows).

`.with_base_color(Color)` on `WindowView` sets the window background fill color.

## Conditional rendering with Either / OneOf

For views that switch between two variants (e.g. masked vs unmasked), use `Either<A, B>`:

```rust
use xilem::core::one_of::Either;

let view = if condition {
    Either::A(label("masked"))
} else {
    Either::B(text_input(value, callback))
};
```

- Import from `xilem::core::one_of::Either` (not `xilem::view`)
- Both arms must target the same `State` type
- For more than two variants, use `OneOf3`..`OneOf9` from the same module
- Preferred over `Box<AnyWidgetView>` when variant count is small and static — avoids allocation and dynamic dispatch

## Emoji support

Emoji (e.g. ❌, ➕, 👁) render correctly via system font fallback. Unicode math symbols
(e.g. `\u{2295}` ⊕, `\u{2296}` ⊖) do NOT render — they aren't in typical system fonts.
Use emoji for icon buttons.

## Dynamic window sizing

`WindowOptions::with_initial_inner_size` sets the window size at creation time only.
There is no API to resize after the window is shown (xilem limitation).

Compute the desired height before creating `EditorApp` and store it in state:
```rust
let height = match &inner {
    EditorInner::PlainText { .. } => 300.0,
    EditorInner::Json(j) => (200.0 + j.rows.len() as f64 * 52.0).clamp(300.0, 800.0),
};
```

This works because the initial field count is known upfront. Rows added at runtime
won't change the window size, but the portal/scrollable container handles overflow.

## Debugging

Masonry logs to a temp file: `C:\Users\...\AppData\Local\Temp\masonry-*.log`
Set `RUST_LOG=debug` for more output.
