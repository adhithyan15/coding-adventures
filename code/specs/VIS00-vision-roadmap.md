# VIS00 — Computer Vision Roadmap

## Purpose

Locating a QR code inside an arbitrary photograph — as opposed to
decoding one already cropped to a clean `ModuleGrid`, which
`qr-decoder` already does (`MA04-qr-decoder.md`) — turns out not to be
a QR problem. It is one instance of a general one: **find a known
pattern somewhere in an image, then work out the transform that
straightens it back into the shape a downstream reader expects.**
Every 2D barcode format solves this the same way; so does deskewing a
photographed document, or a dozen problems this repo hasn't been asked
to solve yet.

This is a deliberate scope decision, not a default: rather than write
a QR-shaped locator as a one-off (the path `qr-code`/`qr-decoder`
themselves took, and a reasonable one for a closed, well-specified
format), this roadmap invests in the missing **computer-vision
primitives layer** once, so that QR-in-a-photo — and everything after
it — is assembly on top of existing packages rather than a fresh
research problem each time.

### Why this is the leveraged move, concretely

This workspace already ships QR-code-adjacent 2D barcode *encoders*
with **zero decoders** for any of them: `aztec-code`, `data-matrix`,
`pdf417`, `micro-qr`. QR itself had no decoder either, until
`qr-decoder` closed the "already-located grid" half and this roadmap's
own motivating issue (#14456) opened the "find it in a photo" half.
The reason none of the other four have decoders is the same reason QR
didn't: nobody had built the shared primitives — numeric linear
algebra, connected-component labeling, geometric transform estimation,
generalized pattern scanning — that every one of them needs. Built
once at the primitive layer, each format's own locator becomes a
comparably-sized assembly task to `qr-decoder` itself, not a second
from-scratch computer-vision project. The same geometric-estimation
primitive also unlocks photo/document deskewing independent of
barcodes entirely.

## Layer map

```text
  L4  Product            mosaic camera/gallery effects, a scanner reference app
       │                 (independent track — device integration, not CV)
  L3  Applications       qr-locate, then aztec/data-matrix/pdf417/micro-qr-locate,
       │                 document deskew — pure assembly once L0-L2 exist
  L2  Vision primitives  VIS01 linear algebra, VIS02 connected components,
       │                 VIS03 geometric estimation, VIS04 pattern scanning
       │                 (this roadmap's actual investment — nothing here exists today)
  L1  Pixel operations   IMG00-IMG07: convolution, LUTs, point ops, geometric
       │                 warps, compositing, GPU bridge, RAW pipeline
  L0  Pixels & codecs    IC00-IC18: the shared Image buffer + PNG/JPEG/TIFF/
                         BMP/GIF/ICO/RAW/WebP(lossless) decode
```

Each layer depends only on the ones below it. L4 is a parallel,
independent track (device/platform integration, not a vision
algorithm) and can start any time; it does not gate or get gated by
L0–L3.

## What already exists, and what's actually missing

Verified against the real source, not the series' own aspirational
tables (which have already drifted once — see the IMG07 note below).

### L0 — Pixels & codecs (`IC00`–`IC18`)

| Package | Status | Note |
|---|---|---|
| `pixel-container` (`IC00`) | done | The shared `Image` buffer every layer above builds on. No changes needed. |
| `image-codec-jpeg` (`IC04`) | done | Real baseline JPEG decode — the format that matters most, since most camera/gallery photos are JPEG. |
| `image-codec-png`/`tiff`/`bmp`/`gif`/`ico` and the RAW family | done | Broad existing decode coverage; lower priority specifically for a camera/gallery path. |
| `image-codec-webp` (`IC05`) | **partial** | VP8L (lossless) decode only. Lossy VP8 — what an actual camera would emit — is not implemented. Closing this is an amendment to the existing `IC05` spec, not a new package. |
| HEIC/HEIF | **missing entirely** | No package exists anywhere in the repo. This is the default iOS camera-roll format since iOS 11 — a real gap for "pick an existing picture" on the most common mobile platform, and a substantially larger build than the other codecs (HEVC-coded tiles inside an ISOBMFF/MIAF container). Claiming `IC19` for this. |

### L1 — Pixel operations (`IMG00`–`IMG07`)

Convolution, LUTs, point operations, geometric warps (apply-only —
see L2), compositing, and a GPU bridge all already exist and are not
touched by this roadmap. Two gaps:

- **Adaptive/local thresholding.** `image-point-ops::threshold()`/
  `threshold_luminance()` are a single fixed cutoff applied uniformly
  across the whole image. A photographed QR code rarely has even
  lighting corner to corner — one shadow across half the frame pushes
  that half below any single global threshold, no matter where it's
  set. Locating a real photographed code needs a *local* threshold
  (Sauvola or Bradley's integral-image method: compare each pixel to
  the mean of its own neighborhood, not one number for the whole
  image). Claiming `IMG09` for this, as an amendment to the point-ops
  package.
- **Morphological operations.** `IMG00`'s own series table (§ "Series
  Overview") already names this exact gap: `IMG07 — Morphological
  operations — erosion, dilation, opening, closing`. That slot was
  later spent on the RAW colour pipeline instead
  (`IMG07-image-raw-pipeline.md`), and morphology was never given a
  replacement number — this roadmap is where that drift gets noticed
  and corrected. Morphology is exactly what cleans up a binarized
  image before pattern detection runs: closing small gaps in a finder
  pattern's border, removing single-pixel noise a local threshold
  leaves behind. Claiming `IMG08` for it (the next free slot, since
  `IMG07` is now permanently the RAW pipeline).

### L2 — Vision primitives: the actual investment

Nothing in this layer exists in a usable form anywhere in the repo
today. Each gets its own future spec; this roadmap only scopes them.

**`VIS01` — Linear algebra.** A homography — the transform that
un-skews a photographed rectangle back to a square — is the solution
to an 8-unknown linear system. No reusable version of that solve
exists: `matrix` has no `solve`/`invert` of any kind; `blas-library`
has matrix multiply (SGEMM/SGEMV/SGER) but no exposed solver;
`cas-matrix`/`cas-solve` *do* have the real numeric algorithms (LU
decomposition with partial pivoting, exact Gauss–Jordan row reduction,
a working `solve_linear_system`) but operate on the symbolic
computer-algebra `IRNode` tree type, not plain `f32`/`f64` arrays —
correct math, architecturally the wrong layer for a per-frame numeric
solve (allocation-heavy, built for exact rational arithmetic, not
real-time geometry). `VIS01` is a small, dense, real-number
Gaussian-elimination-with-partial-pivoting solver — the numeric
sibling `cas-solve` was always missing, scoped to the small (≤10×10)
dense systems geometric estimation actually produces, not a general
sparse/large-matrix solver.

**`VIS02` — Connected components.** Two-pass union–find labeling over
a binary image: which lit pixels touch which. Turns "a blob of dark
pixels" into "one object with a bounding box, a centroid, and a pixel
count" — the shape a finder-pattern candidate needs to be in before it
can be measured or compared against its neighbors.

**`VIS03` — Geometric estimation.** Given point correspondences —
landmarks located in the photographed image, and where they *should*
sit in a straightened one — solve for the transform between them,
built on `VIS01`. Affine (rotation + scale + skew) is exact from 3
point pairs; full projective homography (the real camera-photo case,
also correcting perspective/keystoning) needs 4 and the standard DLT
(direct linear transform) construction. `image-geometric-transforms::
perspective_warp` already *applies* a given `[[f32; 3]; 3]` matrix
(confirmed exact signature: inverse-warp sampling, output-plane to
input-plane) — `VIS03` is what produces that matrix in the first
place. `RANSAC`-style outlier rejection (for when not every detected
candidate point is trustworthy) is in scope for this package once the
plain 4-point solve is proven, not a separate package.

**`VIS04` — Pattern scanning.** The general form of "scan a row of
pixels for a 1:1:3:1:1 ratio of alternating dark/light runs,"
generalized so it isn't QR-specific. Every 2D barcode format locates
itself in an image the same way: a distinctive run-length signature,
scanned across rows and columns, cross-checked between the two, then
clustered into candidate centers. QR, Aztec, Data Matrix, and PDF417
each use a different ratio; the scan → cross-check → cluster skeleton
is the same algorithm written once, parameterized by the ratio
sequence.

### L3 — Applications (pure assembly once L0–L2 exist)

The QR pipeline this roadmap's motivating issue (#14456) needs,
end to end, once `VIS01`–`VIS04` exist:

```text
file bytes
  │  image-codec-jpeg / heic (L0)
  ▼
PixelContainer
  │  adaptive threshold (IMG09) + morphology cleanup (IMG08)  (L1)
  ▼
binarized image
  │  VIS04 pattern scan + VIS02 connected-component clustering
  ▼
3-4 finder-pattern candidate centers
  │  VIS03 geometric estimation (homography, via VIS01)
  ▼
3×3 transform matrix
  │  image-geometric-transforms::perspective_warp (already exists, unchanged)
  ▼
sampled ModuleGrid
  │  qr_decoder::decode (already exists, unchanged — MA04)
  ▼
decoded bytes
```

Every step above `image-codec-jpeg` and below `qr_decoder::decode` is
new; both endpoints are already shipped. Once this slice exists,
`aztec-code`/`data-matrix`/`pdf417`/`micro-qr` each get an equivalent
locator built from the same four `VIS` packages plus their own
format-specific ratio/decode logic — new L3 work each time, no new L2
work. Document/photo deskewing is `VIS03` alone, no barcode-specific
code at all.

### L4 — Product: camera, gallery, and an app

Independent of L0–L3 — this is device integration inside the Mosaic
app framework (`UI38-mosaic-native-application-runtime.md`), not a
vision algorithm, and can start in parallel at any point.

Mosaic's host-effects model (`UI38` §6) has **no camera effect defined
at all**, and its already-specified *files* effect — the closest thing
to a gallery/picture picker — isn't wired into any of the five native
backends yet either (confirmed via `engram-mosaic-app`'s own README,
which documents exactly this gap: effect payloads serialize onto the
wire but no generated host reads them yet). Both a live-camera-preview
effect and a pick-existing-picture effect are genuinely new surface
area, needed on however many of SwiftUI/XAML/Qt/Compose/Flutter this
work targets.

`task-mosaic-app` is a real, working reference app running on all five
native backends today, but only exercises local-storage persistence —
it's the right template to clone the shape of, it just hasn't touched
a device API yet.

## Suggested build order

1. **`VIS01` → `VIS02` → `VIS03` → `VIS04`.** Each is small and
   independently testable; `VIS03` depends on `VIS01`, the rest are
   largely parallel. Nothing here is gated by a product decision.
2. **`IMG08` (morphology) + `IMG09` (adaptive threshold).** Small, and
   `VIS04`'s real-photo accuracy depends on binarization quality —
   worth sequencing alongside phase 1, not after it.
3. **QR locate (L3).** First real proof of the L2 foundation, and
   what issue #14456 was tracking — ships a working "photo → decoded
   string" pipeline before any app exists to call it from.
4. **Mosaic camera + gallery effects (L4).** Can start as early as
   phase 1, in parallel — depends on a platform-priority decision
   (§ Open questions), not on any of the CV work above.
5. **Scanner reference app.** Wires phase 3 and phase 4 together.
6. **Later:** Aztec/Data Matrix/PDF417/Micro QR locate, each a new L3
   slice on the existing L2 foundation. Document/photo deskew as a
   standalone `VIS03` consumer, no barcode work required.

## Open questions

These are genuine product/scope decisions, left open deliberately
rather than defaulted:

- **How broad should the first pass of L2 be?** Building all four
  `VIS` packages to a general standard is slower to a first
  QR-in-photo demo, but nearly free for every barcode format and the
  deskew use case afterward. Building only the `VIS03`/`VIS04` slice
  QR itself needs is faster to demo, revisited when a second format is
  actually wanted.
- **Which Mosaic backend(s) first for the camera/gallery effects?**
  SwiftUI, XAML, Qt, Compose, and Flutter are all native-complete for
  `task-mosaic-app` today; proving the device-API pattern on one or
  two before generalizing to all five keeps phase 4 from becoming five
  parallel unknowns at once.
- **Live camera scanning, or gallery/snapshot only, for the first
  working version?** Live video adds a real-time performance budget —
  a fast reject-this-frame pre-filter ahead of the full `VIS04` scan,
  frame debouncing — on top of everything above. A snapshot-only first
  cut proves the whole L0–L4 chain without that constraint; live
  scanning then becomes a performance pass on an already-correct
  pipeline, not a correctness problem solved under a frame-rate
  budget simultaneously.

## Language scope

Rust-first, matching the precedent already set by `qr-decoder`
(`MA04-qr-decoder.md` §5) and the entire `vault-pm-*` family — no
tooling in this repo enforces cross-language package parity, and the
only concrete consumer driving L2/L3 today (a future `qr-locate`
crate) is Rust. `IMG`/`IC` themselves are multi-language series; a
port of `VIS01`–`VIS04` to those same languages is legitimate future
work once the Rust primitives are proven, not a blocker to starting
here.

## Cross-references

- `MA04-qr-decoder.md` — the already-shipped "decode an already-located
  grid" half this roadmap's L3 work completes.
- `qr-code.md` — the QR encoder; its own "Future Extensions" already
  points here for the image-locating half.
- `UI38-mosaic-native-application-runtime.md` — the app framework L4
  builds on.
- Issue #14456 — the original motivating request ("locate a QR code
  within an arbitrary raster image"), reframed by this roadmap into
  the general L2 investment above rather than a QR-specific build.
