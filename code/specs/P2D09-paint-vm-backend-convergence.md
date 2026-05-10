# P2D09 - Paint VM Backend Convergence and Expansion

## Overview

This spec defines the convergence plan for every Rust Paint VM backend. The goal
is simple: a producer emits one `PaintScene`, and the pipeline can choose any
available backend that satisfies the scene's requirements.

The backend should be an implementation detail. A QR renderer, Mermaid renderer,
HTML layout pipeline, chart renderer, or document renderer should not care
whether the final pixels came from Direct2D, GDI, Cairo, Skia, Vulkan, OpenGL,
Mesa, OpenCL, WGPU, Metal, or a future backend.

```text
Producer
  -> layout
  -> PaintScene
  -> backend selector
  -> selected Paint VM backend
  -> PixelContainer / window surface / encoded output
```

This spec extends P2D06, which introduced the first native backend plan for
Direct2D, GDI, and Cairo. P2D09 is the larger contract: backend parity,
capability reporting, backend selection, and the remaining Rust crates.

---

## Goals

1. Define a shared Rust backend interface for all pixel-producing Paint VMs.
2. Define a capability model so the pipeline can choose a backend safely.
3. Specify the remaining backend crates: Cairo, Skia, Vulkan, OpenGL, WGPU,
   Mesa, OpenCL, and CoreGraphics.
4. Define a compatibility test suite that every backend must pass before it is
   considered interchangeable.
5. Keep paint dumb: text shaping, layout, DOM semantics, diagram semantics, and
   font resolution stay above PaintScene.

---

## Non-Goals

- This spec does not define the layout engine.
- This spec does not define font shaping. `PaintGlyphRun` remains the primary
  portable text primitive.
- This spec does not require every backend to implement every feature on day
  one. It does require backends to report unsupported capabilities honestly.
- This spec does not require every backend to be the fastest possible version.
  Correctness comes first, then acceleration.

---

## Backend Families

The Paint VM family should eventually include these Rust crates:

| Crate | Backend | Primary platforms | Role |
|-------|---------|-------------------|------|
| `paint-metal` | Metal | macOS, iOS | Apple GPU backend |
| `paint-vm-direct2d` | Direct2D + DirectWrite | Windows | Modern Windows renderer |
| `paint-vm-gdi` | GDI + Win32 | Windows | Conservative Windows fallback |
| `paint-vm-cairo` | Cairo + Pango | Linux, BSD, cross-platform | Mature CPU vector backend |
| `paint-vm-skia` | Skia | Windows, macOS, Linux | High-quality portable raster/vector backend |
| `paint-vm-vulkan` | Vulkan | Windows, Linux, Android, BSD | Raw explicit GPU backend |
| `paint-vm-opengl` | OpenGL | Broad legacy desktop support | Legacy GPU fallback |
| `paint-vm-wgpu` | WGPU | Vulkan, Metal, DX12, WebGPU | Portable modern GPU backend |
| `paint-vm-opencl` | OpenCL compute | Windows, Linux, macOS, vendor runtimes | Compute raster experiments |
| `paint-vm-mesa` | Mesa profiles | Linux, BSD, Windows via Mesa3D | Software/driver profile routing for OpenGL/Vulkan |
| `paint-vm-coregraphics` | CoreGraphics + CoreText | macOS, iOS | Apple CPU/native fallback |
| `paint-vm-runtime` | Backend registry + selector | Cross-platform | Capability analysis and automatic backend choice |

Existing TypeScript SVG and Canvas backends remain useful, but this spec is about
Rust-native backends that can participate in the native pipeline.

Mesa is modeled explicitly even though it is not a single drawing API. It gives
the runtime a vocabulary for predictable software and driver profiles such as
llvmpipe and lavapipe behind OpenGL/Vulkan paths. OpenCL is modeled as a compute
raster backend rather than a vector API; it is useful for filters, software
tessellation, and vendor-runtime experiments.

---

## Portability Contract

The portability contract is stricter than "all crates expose a render function."
It defines where semantic work is allowed to happen and where backend-specific
plumbing begins.

Pipeline layers:

| Layer | Responsibility | Must not do |
|-------|----------------|-------------|
| Producers | Barcode, Mermaid, chart, HTML, document, or UI semantics | Choose a graphics backend |
| Layout | Resolve boxes, inline flow, glyph positions, transforms, and resource references | Emit backend API calls |
| PaintScene | Backend-neutral paint record | Depend on Skia, Cairo, Vulkan, OpenGL, OpenCL, or OS handles |
| Runtime selector | Analyze scene requirements and pick a compatible backend | Silently ignore unsupported features |
| Backend crate | Lower `PaintScene` to a concrete renderer | Reinterpret layout or diagram semantics |

Every backend must accept the same `PaintScene` contract:

| Requirement | Contract |
|-------------|----------|
| Coordinate space | Top-left origin, x right, y down, f64 scene coordinates. Backends with bottom-left or NDC coordinates must adapt internally. |
| Color | Parse the shared paint color syntax and produce premultiplied/straight output according to `PixelContainer` rules. |
| Transforms | Apply Paint VM transforms in scene order, not in backend-specific order. |
| Clips | Rect clips are required before path clips; nested clips intersect. |
| Gradients | `PaintGradient` definitions are collected by id and referenced through `url(#id)`. Missing ids are render errors in strict mode. |
| Text | `PaintGlyphRun` is the exact portable primitive; `PaintText` is a convenience primitive and may be degraded. |
| Images | `ImageSrc::Pixels` is the baseline; decoded URI/file image support is optional and must be reported. |
| Output | Offscreen `PixelContainer` export is mandatory for backend parity tests, even if a backend also supports live windows. |

Backends can be idiomatic internally. A vector backend may set state on a
context; a GPU backend may emit meshes and shaders; a compute backend may raster
tiles. The externally visible result is still governed by `PaintScene`,
capability reporting, and the shared compatibility suite.

---

## Backend Architecture Layers

Paint VM backends fall into three implementation families:

| Family | Backends | Preferred implementation shape |
|--------|----------|--------------------------------|
| Native vector/canvas | Direct2D, GDI, Cairo, Skia, CoreGraphics | Map Paint instructions directly to the native drawing context, using save/restore or layers for isolation. |
| Graphics GPU | Metal, WGPU, Vulkan, OpenGL, Mesa profiles | Consume `paint-vm-gpu-core` render plans, upload meshes/textures, execute shader pipelines, then read back pixels. |
| Compute/software raster | OpenCL, future CPU tile rasterizers | Consume `paint-vm-gpu-core` render plans, rasterize triangles/tiles in kernels, then read back or share the output buffer. |

The split is intentional. Skia and Cairo should not be forced through GPU mesh
lowering when their native vector APIs already solve antialiasing, stroking,
patterns, images, and text integration. Vulkan, OpenGL, WGPU, Mesa, and OpenCL
should not each invent their own path tessellator or gradient sampler.

Shared crates:

| Crate | Role |
|-------|------|
| `paint-instructions` | Stable scene data model and instruction definitions. |
| `paint-vm-runtime` | Backend descriptor, capability model, scene analyzer, selector, and common render trait. |
| `paint-vm-gpu-core` | Backend-neutral GPU/compute render plan: meshes, texture uploads, gradient textures, clip commands, text/glyph preservation. |
| Backend crate | API-specific execution, context/device creation, readback, native text hooks, and capability truth. |

No backend-specific crate should be required by producers. Applications may link
one explicit backend for experiments, but normal product code should go through
`paint-vm-runtime`.

---

## Porting Checklist

A new Paint VM backend is considered properly ported only when it provides the
following pieces:

| Deliverable | Requirement |
|-------------|-------------|
| Crate scaffold | `Cargo.toml`, `README.md`, `CHANGELOG.md`, public `VERSION`, and backend id. |
| Descriptor | `PaintBackendDescriptor` with honest family, acceleration, tier, platform support, priority, and capabilities. |
| Renderer | `PaintRenderer` implementation returning `Result<PixelContainer, PaintRenderError>`. |
| Feature analysis | Unsupported scene features return `PaintRenderError`, not missing pixels. |
| Offscreen target | Headless pixel rendering path suitable for CI. |
| Fixture coverage | Shared compatibility scenes plus backend-specific smoke tests. |
| Docs | Support matrix, platform dependencies, and known degraded paths. |
| CI | Build on at least one supported runner; pixel tests run where runtime/device dependencies exist. |

Tier gates:

| Tier | Minimum requirement |
|------|---------------------|
| Tier 0 scaffold | Descriptor exists, crate builds, runtime does not auto-select it for real rendering. |
| Tier 1 smoke | Offscreen pixels for rect, line, ellipse/path subset, rectangular clip, group transform/opacity, image pixels, gradients if advertised. |
| Tier 2 native scenes | Barcode/QR, Mermaid smoke diagrams, and simple HTML/layout smoke scenes render without missing major primitives. |
| Tier 3 full parity | Shared fixture suite passes within documented tolerances, including text/glyphs, arcs, gradients, layers, images, and blend/filter behavior if advertised. |

---

## Common Rust Interface

Every Rust backend should expose a small common surface:

```rust
pub trait PaintRenderer {
    fn descriptor(&self) -> PaintBackendDescriptor;
    fn render(&self, scene: &PaintScene) -> Result<PixelContainer, PaintRenderError>;
}
```

The shared contract lives in `paint-vm-runtime`. Backend crates implement this
trait and return a `PaintBackendDescriptor` containing the backend id, family,
platform support, acceleration type, maturity tier, and capability set. This
lets applications build a registry without hard-coding backend-specific logic at
the call site.

Backends that can render into a live native surface may expose an additional
surface API, but pixel export must be the common denominator. Pixel export gives
us deterministic tests, image codecs, CI snapshots, and backend comparison.

```rust
pub trait PaintSurfaceRenderer<TSurface> {
    fn render_into(
        &mut self,
        scene: &PaintScene,
        surface: &mut TSurface,
    ) -> Result<(), PaintRenderError>;
}
```

The `render(scene) -> PixelContainer` function remains the ergonomic crate-level
entry point:

```rust
pub fn render(scene: &PaintScene) -> PixelContainer;
```

The trait exists so the backend selector can use every backend uniformly.

---

## Capability Model

Backends must not silently pretend to support features they drop. Each backend
reports a capability set.

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaintBackendCapabilities {
    pub rect: SupportLevel,
    pub line: SupportLevel,
    pub ellipse: SupportLevel,
    pub path: SupportLevel,
    pub path_arc_to: SupportLevel,
    pub glyph_run: SupportLevel,
    pub text: SupportLevel,
    pub image: SupportLevel,
    pub clip: SupportLevel,
    pub group: SupportLevel,
    pub group_transform: SupportLevel,
    pub group_opacity: SupportLevel,
    pub layer: SupportLevel,
    pub layer_opacity: SupportLevel,
    pub layer_filters: SupportLevel,
    pub layer_blend_modes: SupportLevel,
    pub linear_gradient: SupportLevel,
    pub radial_gradient: SupportLevel,
    pub antialiasing: SupportLevel,
    pub offscreen_pixels: SupportLevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupportLevel {
    Unsupported,
    Degraded,
    Supported,
}
```

Definitions:

| Level | Meaning |
|-------|---------|
| `Unsupported` | The backend must reject the scene when strict rendering is requested. |
| `Degraded` | The backend produces approximate output and may be selected only when the caller allows degradation. |
| `Supported` | The backend should render the feature faithfully within its documented tolerances, whether natively or through correct emulation. |

Development builds should prefer loud failures for `Unsupported`. Production
callers may opt into degraded rendering, but that choice belongs to the caller
or pipeline, not to the backend silently.

---

## Scene Requirement Analysis

The selector needs to know what a scene requires before choosing a backend.

```rust
pub struct PaintSceneRequirements {
    pub uses_rect: bool,
    pub uses_line: bool,
    pub uses_ellipse: bool,
    pub uses_path: bool,
    pub uses_path_arc_to: bool,
    pub uses_glyph_run: bool,
    pub uses_text: bool,
    pub uses_image: bool,
    pub uses_clip: bool,
    pub uses_group: bool,
    pub uses_group_transform: bool,
    pub uses_group_opacity: bool,
    pub uses_layer: bool,
    pub uses_layer_opacity: bool,
    pub uses_layer_filters: bool,
    pub uses_layer_blend_modes: bool,
    pub uses_linear_gradient: bool,
    pub uses_radial_gradient: bool,
}

pub fn analyze_scene(scene: &PaintScene) -> PaintSceneRequirements;
```

The analyzer walks the full instruction tree, including nested groups, clips,
and layers. The selector compares requirements against backend capabilities.

---

## Backend Selection

The selector should live in a separate crate:

| Crate | Purpose |
|-------|---------|
| `paint-vm-runtime` | Detect available backends, analyze scenes, select and invoke a backend |

Public API:

```rust
pub enum PaintBackendPreference {
    Auto,
    Named(String),
    PreferGpu,
    PreferCpu,
    PreferDeterministic,
    PreferNativePlatform,
}

pub struct PaintRenderOptions {
    pub preference: PaintBackendPreference,
    pub allow_degraded: bool,
    pub require_antialiasing: bool,
    pub require_exact_text: bool,
}

pub fn render_auto(
    scene: &PaintScene,
    options: PaintRenderOptions,
) -> Result<PixelContainer, PaintRenderError>;
```

Selection algorithm:

1. Honor an explicit backend name from `PaintRenderOptions` or
   `PAINT_VM_BACKEND`.
2. Detect which backend crates are available on the current platform.
3. Analyze the scene requirements.
4. Filter out backends that cannot satisfy required capabilities.
5. Rank remaining backends by platform preference and caller preference.
6. Render with the selected backend.
7. If rendering fails due to backend initialization or device loss, retry with
   the next compatible backend.

Default priority by platform:

| Platform | Default priority |
|----------|------------------|
| Windows | Direct2D, Skia, WGPU, Vulkan, OpenGL, GDI, Cairo |
| macOS | Metal, Skia, WGPU, CoreGraphics, Cairo, OpenGL |
| Linux desktop | Skia, WGPU, Vulkan, Cairo, OpenGL |
| Linux headless | Cairo, Skia CPU, WGPU software if available |
| CI fallback | Cairo or software-capable Skia |

The priority list is not a correctness contract. Capabilities win over priority.
For example, if Direct2D does not support a required instruction but Skia does,
the selector chooses Skia.

---

## Text Contract

`PaintGlyphRun` is the portable text primitive. It represents text after shaping
and layout. Backends should converge on `PaintGlyphRun` first.

`PaintText` is still useful for runtimes that natively shape strings at paint
time, or for convenience renderers. A backend may support `PaintText`, but the
layout pipeline should prefer `PaintGlyphRun` whenever it owns shaping.

Required behavior:

| Instruction | Backend obligation |
|-------------|--------------------|
| `PaintGlyphRun` | Render pre-positioned glyph IDs for supported `font_ref` schemes. |
| `PaintText` | If supported, shape/draw using the backend's native text stack. If unsupported in strict mode, reject the scene. |

Convergence work still needed:

- GDI and Direct2D should share font-ref parsing rules where possible.
- Cairo should support `pango:` glyph bindings and optionally `PaintText` via
  Pango layout.
- Skia should support glyph runs through SkFont/SkTextBlob and string text
  through SkShaper or SkParagraph when available.

---

## Backend Specs

### `paint-vm-cairo`

Purpose: mature CPU vector renderer for Linux/BSD and headless environments.

Recommended dependencies:

- `cairo-rs`
- `pangocairo` or `pango` for text

Handler mapping:

| Paint instruction | Cairo mapping |
|-------------------|---------------|
| Rect | `rectangle`, `fill_preserve`, `stroke` |
| Line | `move_to`, `line_to`, `stroke` |
| Ellipse | `save`, `translate`, `scale`, `arc`, `restore` |
| Path | `move_to`, `line_to`, `curve_to`, `arc` or arc-to-bezier conversion |
| GlyphRun | `pango_cairo_show_glyph_string` or cairo glyph APIs |
| Text | Pango layout |
| Clip | `save`, path/rect clip, children, `restore` |
| Group | `save`, `transform`, opacity group if needed, children, `restore` |
| Layer | `push_group`, children, filters if supported, `pop_group_to_source`, `paint_with_alpha` |
| Image | image surface from `PixelContainer`, source surface, paint |
| Gradient | cairo linear/radial patterns |

First milestone:

- Keep the native Cairo image-surface path as the Linux/BSD baseline.
- Add Pango/PangoCairo glyph and text support so text can move from degraded to
  supported.
- Lower SVG `PathCommand::ArcTo` to Cairo curves or shared arc-to-cubic helpers.
- Keep linear/radial Cairo pattern support in the strict feature set.
- Mermaid smoke render on Linux or WSL.

### `paint-vm-skia`

Purpose: high-quality portable 2D renderer with CPU and GPU paths.

Recommended dependency:

- `skia-safe`

Handler mapping:

| Paint instruction | Skia mapping |
|-------------------|--------------|
| Rect | `Canvas::draw_rect`, `draw_round_rect` |
| Line | `draw_line` with `Paint` stroke settings |
| Ellipse | `draw_oval` |
| Path | `skia_safe::Path` |
| GlyphRun | `TextBlob`, `Font`, positioned glyph APIs |
| Text | `TextBlob`, SkShaper, or SkParagraph |
| Clip | `save`, `clip_rect`, children, `restore` |
| Group | `save`, concat transform, optional saveLayer for opacity |
| Layer | `save_layer`, filters, blend mode, restore |
| Image | `Image::from_raster_data`, draw image rect |
| Gradient | `Shader::linear_gradient`, `Shader::radial_gradient` |

First milestone:

- Keep CPU raster surface as the Tier 1 default so CI and headless machines work
  without GPU initialization.
- Add robust `ArcTo` lowering and stricter glyph/font-ref semantics.
- Add SkShaper/SkParagraph integration before marking text/glyphs exact.
- Add optional GPU surfaces only after CPU parity is stable.
- Golden comparison against Direct2D/GDI on Windows and Cairo on Linux.

### `paint-vm-vulkan`

Purpose: explicit modern GPU backend and a proving ground for the shared GPU
renderer core.

Recommended dependencies:

- `ash` for Vulkan bindings
- `shaderc` or precompiled SPIR-V for shaders

This backend should not manually reinvent all 2D geometry alone. It should use a
shared GPU core:

| Crate | Purpose |
|-------|---------|
| `paint-vm-gpu-core` | Tessellate PaintScene primitives into meshes, glyph atlases, texture uploads, and render passes |

Vulkan mapping:

| Paint instruction | Vulkan strategy |
|-------------------|-----------------|
| Rect, ellipse, path | Tessellate into triangles |
| Line | Stroke tessellation into quads/joins/caps |
| GlyphRun | Glyph atlas texture + instanced quads |
| Clip | Scissor for rect clips, stencil for path clips later |
| Group | Uniform matrix stack |
| Layer | Offscreen framebuffer/image, then composite pass |
| Image | Sampled texture |
| Gradient | Fragment shader or generated gradient texture |

First milestone:

- Device/queue selection without requiring a visible window or swapchain.
- Offscreen RGBA8 color attachment plus staging-buffer readback to
  `PixelContainer`.
- Vertex/index buffers generated from `paint-vm-gpu-core` meshes.
- Descriptor sets for uniforms, images, gradient textures, and future glyph
  atlases.
- Scissor clips for rectangular clips; stencil clips can follow.
- Defer filters and advanced blend modes until render-pass architecture is
  stable.

Vulkan-specific acceptance checks:

| Area | Requirement |
|------|-------------|
| Loader | Runtime load failure returns `BackendUnavailable`, not a panic. |
| Synchronization | Command submission, barriers, and readback are explicit and validated. |
| Format | Output maps to `PixelContainer` RGBA bytes with documented premultiply handling. |
| Headless | CI can run with a software Vulkan path when lavapipe is present, otherwise tests skip cleanly. |

### `paint-vm-opengl`

Purpose: legacy GPU fallback for platforms where OpenGL is still the easiest
available accelerated path.

Recommended dependency:

- `glow` for portable OpenGL function loading

OpenGL should share `paint-vm-gpu-core` with Vulkan where possible.

First milestone:

- Framebuffer object backed render target.
- Triangle renderer for rects, paths, and images.
- Readback via `glReadPixels` to `PixelContainer`.
- Context creation through platform APIs, EGL, or OSMesa depending on target.
- Shader program parity with the shared GPU plan's vertex layout and uniforms.

Important constraints:

- OpenGL is deprecated on macOS. It should not outrank Metal, Skia, or WGPU on
  Apple platforms.
- Driver behavior varies widely. Compatibility tests need tolerances for
  antialiasing and blending differences.
- Global state must be isolated. The backend should bind and restore the state
  it owns so embedding applications do not inherit stale GL state.

### `paint-vm-wgpu`

Purpose: portable GPU backend over Vulkan, Metal, DX12, and browser WebGPU.

Recommended dependency:

- `wgpu`

WGPU can share much of the same architecture as Vulkan and OpenGL through
`paint-vm-gpu-core`, but with a safer and more portable API.

First milestone:

- Offscreen texture render.
- Rect, path tessellation, image texture, group transform, clip scissor.
- Readback to `PixelContainer`.

### `paint-vm-opencl`

Purpose: compute-raster backend for platforms where OpenCL is the common device
API, and a research path for filters, tile rasterization, and CPU/GPU shared
software rendering.

Recommended dependencies:

- An OpenCL loader crate or local FFI bindings.
- Shared kernel source compiled at runtime or precompiled where supported.

OpenCL is not a vector drawing API. It should consume `paint-vm-gpu-core` plans
and run kernels over tiles or spans.

OpenCL mapping:

| Paint instruction | OpenCL strategy |
|-------------------|-----------------|
| Rect, ellipse, path | Rasterize triangle meshes or analytic coverage in kernels |
| Line | Use stroke meshes from `paint-vm-gpu-core` |
| GlyphRun | Sample glyph atlas buffers once the text path exists |
| Clip | Per-tile scissor/coverage mask buffers |
| Group | Pre-fold transforms where possible; otherwise pass matrices to kernels |
| Layer | Separate RGBA buffers plus composite kernels |
| Image | Read-only image/buffer sampling |
| Gradient | Generated gradient textures or analytic gradient kernels |

First milestone:

- Device/context/queue selection with CPU device fallback when available.
- RGBA8 output buffer plus readback to `PixelContainer`.
- Solid mesh raster kernel for rects, paths, and line strokes.
- Buffer upload for images and gradient textures.
- Runtime descriptor remains Tier 0 until kernels produce pixels.

### `paint-vm-mesa`

Purpose: explicit profile router for Mesa software and driver-backed execution,
not a separate drawing API.

Mesa should select an execution profile and then delegate:

| Mesa profile | Preferred Paint VM path |
|--------------|-------------------------|
| `llvmpipe` / `softpipe` OpenGL | `paint-vm-opengl` through EGL/OSMesa |
| `lavapipe` Vulkan | `paint-vm-vulkan` through the Vulkan loader |
| Hardware Mesa driver | Vulkan first, then OpenGL, depending on capability and availability |

First milestone:

- Detect Mesa profiles on Linux/WSL without requiring a visible display.
- Expose profile metadata to `paint-vm-runtime`.
- Route to an OpenGL or Vulkan backend without duplicating render logic.
- Keep deterministic software rendering selectable for CI and machines without
  vendor GPU drivers.

Mesa-specific constraints:

- Mesa capabilities are profile-dependent. The `paint-vm-mesa` descriptor must
  not claim support until it knows which profile will execute the scene.
- Mesa is valuable for CI because llvmpipe/lavapipe can provide deterministic
  software GPU coverage.

### `paint-vm-coregraphics`

Purpose: Apple CPU/native fallback alongside Metal.

Recommended dependencies:

- CoreGraphics and CoreText bindings, preferably through existing local ObjC
  bridge patterns if available.

This backend is valuable because CoreGraphics is the native 2D model on Apple
platforms and maps well to PDF-style vector rendering.

First milestone:

- Rect, line, ellipse, path, image, clip, group transform.
- CoreText glyph run rendering.
- Pixel export through a bitmap CGContext.

---

## Compatibility Test Suite

Every backend should run the same scene suite:

| Fixture | Required instructions |
|---------|-----------------------|
| `basic_rects` | rect fill, stroke, rounded corners |
| `lines_and_dashes` | line stroke width, dash pattern |
| `ellipses` | ellipse fill and stroke |
| `paths` | move, line, quad, cubic, close, fill rules |
| `arcs` | SVG arc semantics through `PathCommand::ArcTo` |
| `clips` | nested rectangular clips |
| `groups` | nested transforms and opacity |
| `layers` | isolated opacity and normal blend |
| `images` | RGBA pixels, file URI decode, opacity |
| `gradients` | linear and radial gradients |
| `glyph_runs` | pre-shaped ASCII text, baseline, color |
| `paint_text` | string text where backend declares support |
| `barcode_qr` | high-contrast pixel-aligned rects |
| `mermaid_flowchart` | real diagram path/text/shape mix |
| `html_smoke` | blocks, borders, inline text, image |

Comparison modes:

| Mode | Use |
|------|-----|
| Exact pixels | Barcode, QR, simple rect scenes |
| Tolerant pixels | Antialiased vector scenes |
| Structural checks | Text present, bounds correct, no missing major primitives |
| Golden PNG | Human-reviewable regression fixtures |

Backends graduate through three tiers:

| Tier | Meaning |
|------|---------|
| Tier 0 | Crate builds and exports a blank scene. |
| Tier 1 | Rect, line, clip, group, image, and glyph smoke tests pass. |
| Tier 2 | Mermaid and HTML smoke scenes render with no missing major primitives. |
| Tier 3 | Full fixture suite passes within backend tolerances. |

Only Tier 2 and above should be candidates for automatic pipeline selection.

---

## Current Gaps to Close

Known gaps after the first native/runtime convergence tranche:

| Area | Status |
|------|--------|
| GDI runtime adapter | Implemented; `paint-vm-gdi` now exposes a `PaintRenderer` adapter, descriptor, capabilities, and runtime registry smoke test. |
| GDI `PathCommand::ArcTo` | Implemented by converting SVG arcs to cubic Beziers and rendering them through the existing GDI path pipeline. |
| GDI gradients | Not implemented. |
| GDI layer filters and blend modes | Layer opacity/transform works; filters/blend modes are not implemented. |
| Direct2D `PaintText` | Implemented with DirectWrite text layout, baseline positioning, alignment, and `directwrite:`/`canvas:` font-ref parsing. |
| Direct2D gradients | Convergence target for native Direct2D gradient brushes; descriptor must remain honest until merged. |
| Direct2D layer filters and blend modes | Layer opacity works; full effects/blend modes are not implemented. |
| Metal text | Still partial. Glyph/text convergence remains a larger font-system project. |
| Cairo | Tier 1 native path implemented on Linux/BSD via `cairo-rs`; gradients are supported; text/glyphs are degraded until Pango/HarfBuzz shaping is connected; SVG `ArcTo` lowering remains open. |
| Skia | Tier 1 CPU raster path exists with primitive/image/gradient coverage; text/glyphs are degraded; `ArcTo`, filters, and blend modes remain open. |
| WGPU | Concrete Tier 1 GPU consumer exists for shared GPU plans and gradients; availability depends on adapter/device setup. |
| Vulkan/OpenGL/OpenCL/Mesa | Tier 0/Tier 1 plan adapters exist; real API execution paths still need context/device setup, draw or kernel dispatch, and readback. |
| CoreGraphics | Still missing as a Rust crate. |
| Shared fixture suite | Needs cross-backend golden/tolerant comparison harness and Mermaid/HTML smoke fixtures. |

Recommended immediate order:

1. Land Direct2D gradients and keep the descriptor aligned with actual support.
2. Add GDI gradients or a correct software fallback for gradient-filled shapes.
3. Cairo Pango/HarfBuzz text and SVG `ArcTo` lowering.
4. Skia `ArcTo`, exact glyph/font semantics, then Mermaid/HTML Tier 2 scenes.
5. Harden `paint-vm-gpu-core` tessellation: robust fills, joins/caps, dashes,
   and glyph-atlas planning.
6. Vulkan execution path: instance/device, render pass, pipeline, texture upload,
   readback.
7. OpenGL execution path and Mesa llvmpipe/lavapipe routing.
8. OpenCL compute raster kernels for solid meshes, images, gradients, and later
   filters/software paths.
9. CoreGraphics/CoreText backend for Apple CPU/native fallback.
10. Filters and advanced blend modes across GPU-capable backends.

---

## Pipeline Integration

The pipeline should choose a backend through `paint-vm-runtime`, not by linking
directly to a backend crate from producer code.

Example:

```rust
let scene = layout_to_paint::render(&layout);
let pixels = paint_vm_runtime::render_auto(
    &scene,
    PaintRenderOptions {
        preference: PaintBackendPreference::Auto,
        allow_degraded: false,
        require_antialiasing: false,
        require_exact_text: true,
    },
)?;
```

Producer crates should depend only on:

- `paint-instructions`
- layout/font crates above the paint layer

Applications choose:

- exact backend crate, for platform-specific experiments
- `paint-vm-runtime`, for normal automatic selection

---

## Build and CI Strategy

Backends should be isolated so CI can build what the machine supports:

| Backend | CI expectation |
|---------|----------------|
| Direct2D | Build and test on Windows runners. |
| GDI | Build and test on Windows runners. |
| Cairo | Build and test on Linux runners with Cairo headers installed; add Pango packages when text shaping lands. |
| Skia | Build on all major OSes if cache/build time is acceptable; otherwise nightly or opt-in CI. |
| Vulkan | Build everywhere with loader headers; hardware tests opt-in. |
| OpenGL | Build everywhere; pixel tests require software GL or platform context setup. |
| WGPU | Build everywhere; adapter-dependent tests gated by availability. |
| OpenCL | Build where an OpenCL loader is available; kernel/device tests gated by runtime availability. |
| Mesa | Linux/WSL profile-detection tests; pixel tests require llvmpipe/lavapipe or OSMesa/EGL setup. |
| CoreGraphics | Build and test on macOS runners. |

Every backend crate should include:

- `README.md` with support matrix.
- `CHANGELOG.md`.
- `required_capabilities.json` if the repo's package metadata expects it.
- Unit tests for each implemented instruction.
- Shared fixture tests through the compatibility suite.

---

## Implementation Notes

The implementation should avoid forcing all backends into the same internal
architecture. Direct2D and Cairo are stateful vector APIs. GDI is stateful but
quirky. Skia is a high-level canvas. Vulkan, OpenGL, WGPU, and Metal are GPU
pipelines that benefit from shared tessellation and shader code.

The shared contract is the scene semantics, not the internal technique.

That gives us room to write each backend idiomatically while still making the
pipeline predictable.
