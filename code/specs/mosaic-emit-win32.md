# mosaic-emit-win32 — Pure Win32 + Paint VM backend for Mosaic

**Status:** Specification (draft)
**Layer:** UI / backend emitter (rendering)
**Depends on:**
  UI22 (mosaic-emit-paint — the Paint VM emit path),
  UI23 (mosaic-pipeline),
  UI24 (mosaic-emit-dispatch),
  P2D00 / P2D01 (PaintInstructions, PaintVM),
  P2D06 (Direct2D native backend — `paint-vm-direct2d`),
  `win32-event-loop.md` (decoupled message pump)
**Sibling backend:** `mosaic-emit-xaml.md` (WinUI 3, C# output). The two share
only the UI24 dispatch contract.
**Produces:** Rust source files (`.rs`) implementing a `Component` struct, a
`render(scene, …)` function that emits Paint VM instructions, and the
Win32 input wiring that translates `win32-event-loop::Event`s into UI24
`{Component}Event` variants.

---

## 1. Purpose

Take a Mosaic three-file pipeline triple and generate a Rust crate (or set
of modules) that, when built and linked into a Win32 application, renders
the component using:

- **Win32 alone** for window creation (no XAML, no WinUI, no .NET runtime).
- **Direct2D + DirectWrite** for drawing, via the existing **Paint VM**
  (`paint-vm` + `paint-vm-direct2d`). The Mosaic emitter never calls
  Direct2D directly; it produces Paint VM scenes and lets the runtime
  dispatch table draw them. This is the same separation as
  UI22 (`mosaic-emit-paint`) — UI22 emits Paint VM scenes for use by *any*
  Paint VM backend, including web canvas. UI29 (this spec) is the Win32
  *executable host* that wraps UI22's output in a real `HWND` + message
  loop.
- The standalone **`win32-event-loop`** crate for the message pump. The
  emitted code does not own the message loop; it registers a handler with
  the loop the host already runs.

The user-facing promise: a Mosaic component compiled with `--backend
win32` produces a single Rust crate that depends on `paint-vm-direct2d`
and `win32-event-loop`. The host application drops that crate into its
`Cargo.toml`, creates an `HWND`, hands it to the generated `Grid::mount(…)`
function, and is done.

Compared with `mosaic-emit-xaml`:

|                          | mosaic-emit-xaml          | mosaic-emit-win32              |
|--------------------------|----------------------------|--------------------------------|
| Output language          | C# + XAML                 | Rust                           |
| Rendering                | WinUI control tree        | Direct2D via Paint VM scene    |
| Runtime dependency       | Windows App SDK 1.5+      | `paint-vm-direct2d` crate only |
| Host integration         | XAML `Resources` + DPs    | `HWND` + closure handler       |
| Final binary             | `.exe` + `.dll`s, MSIX    | single `.exe`, no extra DLLs   |
| DPI / theming surface    | XAML-native               | Hand-rolled (see §8)           |

## 2. Output shape

For a component `Grid` declared in `Grid.mil` / `Grid.desktop.mll` /
`Grid.dark.msl`, the backend writes a Rust module:

```
grid/
  mod.rs          — public `Grid` struct + `mount`/`render`/`dispatch` API
  scene.rs        — the moslayout tree → Paint VM scene builder
  events.rs       — the `GridEvent` discriminated enum (UI24)
  style.rs        — mosstyle properties as Rust constants / functions
  hit.rs          — hit-testing (which cell/element is under (x, y)?), used
                    by the input wiring to translate Win32 mouse events
                    into UI24 events
```

The crate is named `mosaic_generated_<component>` by default; the
`--crate-name` flag overrides it. With `--single-file` (default off) all
five modules collapse into one `{Component}.rs` for embedding directly
into an existing crate without a sub-module.

When invoked with `--emit-host` (default off), the backend additionally
writes:

```
src/main.rs       — host that registers the WNDCLASS, calls
                    `Grid::mount(hwnd, dispatch_callback)`, runs the
                    win32-event-loop, and exits
Cargo.toml        — depends on `paint-vm-direct2d`, `win32-event-loop`,
                    and the generated component crate
```

This is enough to `cargo build --release` and get a working `.exe`. The
VisiCalc plumbing (§10) uses `--emit-host`; library consumers turn it off.

## 3. Mapping table — moslayout primitives → Paint VM scene

Every moslayout primitive lowers to a sub-tree of Paint VM instructions
(`PaintRect`, `PaintGlyphRun`, `PaintGroup`, `PaintClip`, …). The Paint VM
docs (P2D00) define the instruction set; this table is the moslayout view.

| moslayout primitive | Paint VM lowering                                                 |
|---------------------|-------------------------------------------------------------------|
| `Box`               | `PaintGroup { bounds = part_bounds, ops = [ PaintRect(background), …children… ] }` |
| `Row` / `Column`    | `PaintGroup` with children laid out via a layout pass (§5).       |
| `Stack`             | `PaintGroup` — children stacked in source order, all at the same origin. |
| `Grid`              | `PaintGroup` containing header `PaintGlyphRun`s, then a nested `PaintGroup` per row, each holding cell `PaintRect` + `PaintGlyphRun`. See §4. |
| `Text`              | `PaintGlyphRun` resolved at render time from the bound `text` slot via DirectWrite. |
| `Image`             | `PaintImage` (file path or in-memory bytes from the slot). |
| `Input`             | `PaintGroup { bounds, ops = [ PaintRect(background, border), PaintGlyphRun(buffer_text) ] }`. Caret + selection are tracked in the component struct and added to the scene as additional `PaintRect`s during render. See §5. |
| `Scroll`            | `PaintClip` to the viewport bounds; child sub-tree is translated by the scroll offset before being added. |
| `Spacer`            | (no-op — affects layout only). |
| `Divider`           | A single `PaintRect` of the divider thickness in the long axis.   |
| `Icon`              | `PaintGlyphRun` against the Segoe Fluent Icons font.              |

The emitter does NOT call into Direct2D directly. It builds a
`paint_instructions::Scene` and lets the `paint-vm-direct2d` runtime walk
it. The emitter is therefore renderer-agnostic — the same generated code
can later run against a Skia or Cairo backend by swapping the runtime
crate.

## 4. Grid primitive

The Grid primitive carries the most complexity. The emitter produces:

```rust
impl Grid {
    pub fn build_scene(&self) -> paint_instructions::Scene {
        let mut scene = Scene::new();
        let sheet_bounds = self.layout.sheet_bounds();

        // Part style: background, font, padding (from mosstyle).
        scene.push(PaintRect {
            bounds: sheet_bounds,
            fill:   style::SHEET_BACKGROUND,
            stroke: None,
        });

        // Headers: one PaintGlyphRun per column.
        for (c, header) in self.column_headers.iter().enumerate() {
            scene.push(PaintGlyphRun {
                bounds:    self.layout.header_bounds(c),
                text:      header.clone(),
                font:      style::HEADER_FONT,
                fill:      style::HEADER_COLOR,
                alignment: Alignment::Center,
            });
        }

        // Cells: PaintRect for background + PaintGlyphRun for text. Selection
        // and editing highlights are inlined into the cell's fill colour at
        // build-scene time (mirroring the React backend's inline style spread).
        for (r, row) in self.viewport_rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                let fill = if (r as i32) == self.selected_row && (c as i32) == self.selected_col {
                    style::SELECTED_BACKGROUND
                } else if (r as i32) == self.edit_row && (c as i32) == self.edit_col {
                    style::EDITING_BACKGROUND
                } else {
                    style::CELL_BACKGROUND
                };
                let bounds = self.layout.cell_bounds(r, c);
                scene.push(PaintRect { bounds, fill, stroke: Some(style::CELL_BORDER) });
                scene.push(PaintGlyphRun {
                    bounds, text: cell.clone(),
                    font: style::CELL_FONT, fill: style::CELL_COLOR,
                    alignment: Alignment::LeftCenter,
                });
            }
        }
        scene
    }
}
```

`self.layout` is a small layout cache produced by `scene.rs`'s layout pass
(see §5). `style::*` constants are emitted into `style.rs` from the
mosstyle `.msl` source.

### 4.1 column-widths

When the moslayout binds `column-widths: slot: column-widths`, the
emitter inserts a `column_widths: Vec<f32>` field on the component struct
and the layout pass consumes it to compute `cell_bounds(r, c)`. When the
prop is absent, the layout pass falls back to equal-width columns sized
to `sheet_bounds.width() / col_count`.

### 4.2 Hit testing → onNavigate

The generated `hit.rs` module produces a function:

```rust
pub fn hit_test(&self, x: i32, y: i32) -> Option<GridHit> {
    // returns Cell { row, col } or HeaderCell { col } or None
}
```

The Win32 input wiring (§7) calls this on every `Event::MouseDown { Left, .. }`
and dispatches the corresponding `GridEvent::Navigate { row, col }` when
the cell hit succeeds. Header cells produce nothing today (UI26 reserves
header click for column sorting, which is out of scope).

## 5. Layout pass

moslayout's vertical / horizontal / grid containers each get a small
layout function generated alongside the scene builder. The functions are
deterministic and only depend on the bound slot values + the sheet
bounds; they compute child bounds in pixels and stash them in a
`LayoutCache`. Re-running the layout when a slot value changes is cheap
and runs on the main thread before `build_scene()`.

The layout vocabulary is intentionally small:

- `Box`: child fills the box minus padding.
- `Row` / `Column`: lay out children along the main axis. Each child
  contributes either a fixed size (from style `width` / `height`) or
  `flex: <weight>` consumed from a future moslayout grammar extension. The
  first cut treats every non-fixed-size child as `flex: 1`.
- `Stack`: every child gets the parent's full bounds.
- `Grid`: see §4.

The layout pass is *not* a full constraint solver — it is a single
top-down walk that runs in `O(n)` over the moslayout tree. This is enough
for VisiCalc (one Grid + one FormulaBar) and explicitly punts on the
hard cases (resizable splits, percentage-of-parent, intrinsic sizing).

## 6. Event dispatch (UI24)

For a component with `n` emits, the emitter writes the discriminated enum
to `events.rs`:

```rust
#[derive(Debug, Clone)]
pub enum GridEvent {
    Navigate   { row: i32, col: i32 },
    EditCommit { value: String },
    // ...
}
```

…and the component struct exposes a single dispatch field:

```rust
pub struct Grid {
    // slot fields...
    dispatch: Box<dyn Fn(GridEvent) + 'static>,
}

impl Grid {
    pub fn mount(
        hwnd: HWND,
        dispatch: impl Fn(GridEvent) + 'static,
    ) -> (Self, win32_event_loop::Registration) {
        let component = Grid { /* slots = defaults */, dispatch: Box::new(dispatch) };
        let registration = win32_event_loop::EventLoop::current().register(
            hwnd,
            // input wiring — see §7
            component.handler(),
        );
        (component, registration)
    }
}
```

The empty-emit case still emits the enum (`pub enum FooEvent {}` —
matching the `export type FooEvent = never` shape from UI24 §3.1) so
downstream `match` arms compile uniformly.

## 7. Input wiring — `win32-event-loop::Event` → UI24

The generated `mod.rs` includes a `handler()` method that constructs a
`win32_event_loop::EventHandler` whose `handle()` body is a `match` over
the loop's typed events:

```rust
impl Grid {
    fn handler(&self) -> impl win32_event_loop::EventHandler {
        let dispatch = self.dispatch.clone(); // Arc'd internally
        win32_event_loop::closure_handler(move |event| {
            match event {
                Event::Paint { hwnd, rect } => {
                    let scene = self.build_scene();
                    paint_vm_direct2d::render(&scene, hwnd, rect);
                    Action::Consumed
                }
                Event::Resize { hwnd: _, client } => {
                    self.set_bounds(client);
                    Action::Consumed
                }
                Event::MouseDown { x, y, button: MouseButton::Left, .. } => {
                    if let Some(GridHit::Cell { row, col }) = self.hit_test(x, y) {
                        dispatch(GridEvent::Navigate { row, col });
                    }
                    Action::Consumed
                }
                Event::Char { codepoint, .. } => {
                    // Forwarded to host as future GridEvent::TypeChar — out of
                    // scope for v1, falls through to Default.
                    Action::Default
                }
                _ => Action::Default,
            }
        })
    }
}
```

Three things to notice:

1. The handler **never calls `GetMessage` or `DispatchMessage`** — that is
   the event loop's job. The handler is a pure function from `Event` to
   `Action`.
2. The handler **never calls Direct2D APIs directly** — it builds a
   `Scene` and lets `paint_vm_direct2d::render` walk it. The Paint VM is
   the only renderer surface the generated code knows about.
3. The `Action::Default` return values matter — they preserve normal
   Win32 behaviour for everything the component didn't model.
   Keyboard accelerators, Alt+F4, system shortcuts, etc. keep working.

### 7.1 Input primitive — caret + selection

The `Input` primitive is the only one with rich keyboard handling. UI25
specifies four emits and §5 of this spec maps them. The Win32 wiring is:

| Event                                | Maps to                                |
|--------------------------------------|----------------------------------------|
| `Char { codepoint, .. }`             | append to buffer; dispatch `Change(value)` |
| `KeyDown { vk: Backspace, .. }`      | pop from buffer; dispatch `Change(value)` |
| `KeyDown { vk: Enter, .. }`          | dispatch `Commit`                      |
| `KeyDown { vk: Escape, .. }`         | dispatch `Cancel`                      |
| `KeyDown { vk: Left/Right/Home/End, .. }` | move caret; trigger repaint        |
| `MouseDown { Left, x, y, .. }`       | caret-by-hit-test; trigger repaint     |

The caret is rendered as a single thin `PaintRect` blinking via a
`SetTimer`-driven `WM_TIMER` invalidate. Selection is a `PaintRect` with
a translucent fill behind the affected glyph run.

## 8. Style application — mosstyle → `style.rs`

The `.msl` source's part blocks are compiled to a `style.rs` of typed
constants:

```rust
// style.rs — auto-generated from Grid.dark.msl
use paint_instructions::{Color, Font, Stroke};

pub const SHEET_BACKGROUND: Color  = Color::from_rgb(0x1e, 0x1e, 0x1e);
pub const SHEET_FONT_FAMILY: &str  = "Consolas";
pub const SHEET_FONT_SIZE:   f32   = 12.0;
pub const CELL_BORDER:       Stroke = Stroke { color: Color::from_rgb(0xe0, 0xe0, 0xe0), width: 1.0 };
pub const SELECTED_BACKGROUND: Color = Color::from_rgb(0x26, 0x4f, 0x78);
pub const EDITING_BACKGROUND:  Color = Color::from_rgb(0x1f, 0x4f, 0x3f);
// ...
```

State blocks (`state hover { … }`) become per-state functions:

```rust
pub fn cell_background(state: CellState) -> Color {
    match state {
        CellState::Hover    => Color::from_rgb(0x2a, 0x2d, 0x2e),
        CellState::Selected => SELECTED_BACKGROUND,
        CellState::Editing  => EDITING_BACKGROUND,
        CellState::Idle     => SHEET_BACKGROUND,
    }
}
```

Sub-parts (UI27 §3 — `sheet/cell`, `sheet/header-cell`) get their own
constant groups (`SHEET_CELL_*`, `SHEET_HEADER_CELL_*`).

The .msl property → Rust constant mapping is the same fixed table as the
XAML backend (§8 of `mosaic-emit-xaml.md`), but with `Color` / `Font` /
`Stroke` types from `paint_instructions` instead of XAML setters.

### 8.1 DPI awareness

Generated layout code treats logical and physical pixels as **the same**
in v1. A follow-up will pipe `Event::Resize`'s DPI through the layout
pass (the loop reports DPI via a future event; for now the host calls
`SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)`
and the layout works in physical pixels).

## 9. Public API

```rust
// code/packages/rust/mosaic-emit-win32/src/lib.rs

pub struct Win32Renderer { /* options */ }

pub struct Win32EmitResult {
    pub modules: Vec<EmittedModule>,        // mod.rs, scene.rs, events.rs, style.rs, hit.rs
    pub host:    Option<HostFiles>,         // Some(...) when emit_host=true
    pub component_name: String,
}

pub struct EmittedModule {
    pub filename: String,                   // "mod.rs", "scene.rs", ...
    pub source:   String,
}

pub struct HostFiles {
    pub main_rs:    String,
    pub cargo_toml: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Win32EmitError {
    #[error("component name mismatch: mil/mll/msl disagree ({0:?})")]
    ComponentNameMismatch(Vec<String>),
    #[error("unsupported primitive: {0}")]
    UnsupportedPrimitive(String),
    #[error("slot type {0:?} has no Win32 mapping")]
    UnmappableSlotType(String),
    #[error("style property {0:?} has no Paint VM mapping")]
    UnmappableStyleProperty(String),
}

pub fn from_pipeline(
    model:  &mosmodel_compiler::MosmodelComponent,
    layout: &moslayout_compiler::LayoutDef,
    style:  &mosstyle_compiler::StyleDef,
    options: &EmitOptions,
) -> Result<Win32EmitResult, Win32EmitError>;
```

Mirrors the React and XAML emitters; one type swap, same call shape.

## 10. VisiCalc plumbing — proof that this all hangs together

```
code/programs/typescript/visicalc/
  windows/win32/
    Cargo.toml                  — generated (when --emit-host)
    src/
      main.rs                   — generated host: registers WNDCLASS, creates
                                   HWND, mounts Grid + FormulaBar, runs the
                                   win32-event-loop
      generated/
        grid/                   — Grid component crate (mod.rs, scene.rs, ...)
        formula_bar/            — FormulaBar component crate
      state.rs                  — hand-written; mirrors src/app/state.ts. The
                                   reducer is a `fn step(state: AppState,
                                   event: AppEvent) -> AppState` shaped
                                   identically to the React reducer.
  windows/build.ps1             — calls `mosaic-compile --backend win32` for
                                   each component, then `cargo build --release`
```

The end-to-end story: `cargo build --release` in
`code/programs/typescript/visicalc/windows/win32/` produces a single ~3-5 MB `.exe` with no
runtime dependencies (it statically links `paint-vm-direct2d`,
`win32-event-loop`, and the generated component crates). Double-clicking
it opens a window that renders the same VisiCalc grid as the React demo,
fed by the same `.mil`/`.mll`/`.msl` sources.

The host file `state.rs` is approximately 150 lines of pure Rust mirroring
`src/app/state.ts` — same fields, same reducer cases, same A1 helpers.
The shared invariant from UI26 (host owns state, components are dumb
renderers) carries through unchanged.

## 11. CLI integration

`mosaic-compile --backend win32` accepts the standard pipeline flags
(`--interface`, `--layout`, `--style`, `-o`) plus:

| Flag                   | Default | Effect                                          |
|------------------------|---------|-------------------------------------------------|
| `--emit-host`          | `false` | Also write `src/main.rs` + `Cargo.toml`.        |
| `--crate-name <name>`  | `mosaic_generated_<component>` | Override the generated crate name.  |
| `--single-file`        | `false` | Concatenate mod.rs/scene.rs/events.rs/style.rs/hit.rs into one file. |
| `--paint-vm-rev <hash>`| pinned  | Pin a specific `paint-vm` git revision in the emitted `Cargo.toml` when `--emit-host` is on. |

The `mosaic-compile` match in `code/packages/rust/mosaic-compile/src/main.rs`
adds a new `"win32"` arm that constructs a
`mosaic_emit_win32::Win32Renderer { … }` and writes the file set.

## 12. Test plan

1. **Unit tests** (cross-platform — emitter is pure code generation):
   - Each row of §3 → assert the emitted `scene.rs` contains the
     expected `Scene::push(...)` call.
   - Each row of §8 → assert the emitted `style.rs` contains the
     expected constant.
   - UI24 dispatch cases (empty, one, three emits).
   - Component-name mismatch / unsupported primitive errors.
2. **Compile-the-output tests** (cross-platform via `cargo check`):
   for each `code/programs/mosaic/visicalc/*` triple, run the emitter, drop the
   output into a temp crate, and assert `cargo check` succeeds. This
   catches emitter bugs that produce un-compilable Rust.
3. **Windows-only smoke tests** (`#[cfg(target_os = "windows")]`):
   on the windows-latest CI matrix:
   - `cargo build --release` the VisiCalc Win32 demo.
   - Launch the resulting `.exe` with a `--smoketest` flag that
     immediately calls `request_quit` after the first paint;
     exit-code 0 means the WNDCLASS registered, the HWND was created,
     `Grid::mount` succeeded, and the first scene rendered without
     panic.

Target coverage: ≥90% for the emitter, exit-code-0 from the smoke test.

## 13. Why a separate event-loop crate

The `win32-event-loop.md` spec explains the rationale at length. The
short version: the existing `window-win32` crate bakes together window
creation, paint dispatch, and message pumping. Mosaic Win32's generated
code wants only the third concern, decoupled enough that:

- One event loop can host multiple Mosaic components in the same window
  (different sub-HWNDs, or one HWND with internal routing).
- A future MFC-style host can mix Mosaic components with hand-written
  Win32 controls.
- Tests can pump synthetic messages without owning any real window.

The Mosaic Win32 emitter therefore *uses* but does not *own* the loop —
the host application constructs the loop, registers the component's
handler, and runs the loop on its own schedule.

## 14. Why Paint VM rather than direct Direct2D calls

Three reasons:

1. **Renderer agnosticism.** The generated code only knows about
   `paint_instructions::Scene`. Swapping `paint-vm-direct2d` for
   `paint-vm-skia` or `paint-vm-cairo` later is one Cargo dependency
   change; no emitter change is required.
2. **Diff-based repaint.** The Paint VM exposes a `patch(old, new, ctx)`
   API (P2D01) that diffs two scenes and replays only the changed
   instructions. The Mosaic emitter does not need to know about this —
   the host calls `vm.patch(prev_scene, new_scene, &mut d2d_ctx)` on
   every Win32 `Event::Paint` and gets retained-mode performance for
   free.
3. **Testability.** Pure-Rust unit tests can build a `Scene` and assert
   on its instruction list without touching Win32 or Direct2D.

## 15. Out of scope (tracked as follow-ups)

- **Compose mode.** Multiple top-level Mosaic components composed into
  one window (today the spec assumes one component per HWND). A follow-up
  defines a `Compose` primitive in moslayout and an `--emit-compose-root`
  flag.
- **Animations.** Same status as the XAML backend — the IR does not yet
  model motion, so neither backend can generate animations.
- **High-DPI per-monitor changes.** v1 reads DPI once at mount; tracking
  DPI changes mid-flight (e.g. window moved between monitors) needs a
  `Event::DpiChanged` variant on the event-loop crate.
- **Theme switching.** Like the XAML backend, multi-theme requires two
  `.msl` sources and a runtime selector; the demo only ships dark today.
- **Accessibility (UIA).** Exposing Mosaic-rendered components to screen
  readers is a separate spec; Direct2D-painted controls are otherwise
  invisible to UIA without explicit work.
- **Keyboard focus traversal.** v1 assumes one focusable element per
  component (the Input). Multi-input focus rings, Tab order, etc. follow
  in a focused-input PR.
