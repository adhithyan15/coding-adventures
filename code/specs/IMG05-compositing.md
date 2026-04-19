# IMG05 — Compositing and Alpha Blending

## Overview

**Compositing** is the process of combining two or more images into a single
result. Every layer system, paint back-end, and document renderer in this
stack ultimately asks the same question: given a foreground pixel and a
background pixel, what is the output pixel?

The answer depends on three things:

1. **Alpha** — how opaque the foreground is at that position
2. **Blend mode** — what mathematical relationship governs how the colours interact
3. **Premultiplication** — whether the colour channels have already been multiplied
   by alpha

This spec covers Porter-Duff compositing operators, Photoshop-style blend
modes, masking, and the full multi-layer compositing pipeline. All operations
take `Image` (IC00) as both input and output, and all blending
is performed in **linear light** (see IMG00 §2, IMG03 §1).

---

## 1. The Alpha Channel

`Image` stores four channels per pixel: R, G, B, A. The alpha
channel A encodes **opacity**:

```
A = 0   → fully transparent (the pixel contributes nothing to composites)
A = 255 → fully opaque (the pixel completely covers what is behind it)
```

Alpha can represent:
- **Shape** (hard edges: A ∈ {0, 255}) — cutout silhouettes
- **Coverage** (anti-aliased edges: A ∈ [0, 255]) — smooth boundaries
- **Translucency** (interior pixels with A < 255) — glass, fog, watermarks

---

## 2. Premultiplied vs Straight Alpha

This is the single most common source of compositing bugs.

### Straight alpha (unassociated alpha)

The colour channels R, G, B store the pixel's colour *independently* of
alpha. A fully transparent pixel may have any colour value in its RGB channels —
those values are simply ignored when alpha = 0.

```
Straight:  A pixel with colour red and 50% opacity:  R=255  G=0  B=0  A=128
```

### Premultiplied alpha (associated alpha)

The colour channels store the pixel's colour *already scaled by* the alpha
factor. A fully transparent pixel has R=G=B=0. The colour is "baked in" to
the channel values.

```
Premultiplied:  A pixel with colour red and 50% opacity:  R=128  G=0  B=0  A=128
                (R = 255 × 128/255 ≈ 128)
```

### Why premultiplied is preferred for compositing

The **over** operator (§3.1) in premultiplied form:

```
Premultiplied over:
  out.rgb = fg.rgb + bg.rgb * (1 − fg.a)   ← just one multiply per pixel
  out.a   = fg.a  + bg.a  * (1 − fg.a)
```

The straight-alpha form requires an extra divide at output:

```
Straight over:
  out.rgb = (fg.rgb * fg.a + bg.rgb * bg.a * (1 − fg.a)) / out.a
  out.a   = fg.a + bg.a * (1 − fg.a)
```

Premultiplied avoids the divide-by-alpha (which is numerically unstable when
alpha is near zero) and is the preferred format for render pipelines. However,
premultiplied RGBA8 suffers from **precision loss** in dark semi-transparent
pixels because the colour channels are small (colour = premultiplied / alpha
loses bits). For this reason:

- **Storage and I/O**: straight alpha (what PNG, JPEG, and TIFF store)
- **Processing and compositing**: premultiplied alpha

`Image` stores **straight alpha** (matching IC codecs). Operations in
this spec convert to premultiplied internally, composite, then convert back.

### Conversion formulas

```
Straight → premultiplied:
  R_pre = R_straight × A / 255    (integer: R_pre = (R_straight * A + 127) / 255)
  G_pre = G_straight × A / 255
  B_pre = B_straight × A / 255

Premultiplied → straight:
  R_straight = (A > 0) ? R_pre × 255 / A : 0
  (same for G, B)
```

---

## 3. Porter-Duff Compositing Operators

Thomas Porter and Tom Duff (SIGGRAPH 1984) described a complete algebra of
compositing operators. Every operator is defined by what fraction of the
foreground (F) and background (B) pixel survives into the output.

### The general form

Let F and B be the foreground and background pixels, both in premultiplied
linear f32 (α ∈ [0, 1]). Each Porter-Duff operator is characterised by two
factors (fF, fB) — the fraction of each pixel that contributes:

```
out.rgb = F.rgb * fF + B.rgb * fB
out.a   = F.a   * fF + B.a   * fB
```

The twelve standard operators and their factors:

```
Operator        fF                  fB                  Description
─────────────────────────────────────────────────────────────────────
clear           0                   0                   Both erased
copy            1                   0                   F only
destination     0                   1                   B only
over            1                   1 − F.a             F over B (most common)
in              B.a                 0                   F where both exist
out             1 − B.a             0                   F where B is absent
atop            B.a                 1 − F.a             F atop B, keep B shape
xor             1 − B.a             1 − F.a             Union minus intersection
lighter         1                   1                   Additive (clamped to 1)
over (reverse)  1 − B.a             1                   B over F
in (reverse)    0                   F.a                 B where both exist
out (reverse)   0                   1 − F.a             B where F is absent
atop (reverse)  1 − B.a             F.a                 B atop F, keep F shape
```

### 3.1 The `over` operator (the default)

`over` is by far the most common compositing operation — it places the
foreground layer over the background:

```
// F (foreground) over B (background), both premultiplied linear f32:
out.r = F.r + B.r * (1 − F.a)
out.g = F.g + B.g * (1 − F.a)
out.b = F.b + B.b * (1 − F.a)
out.a = F.a + B.a * (1 − F.a)
```

Intuition: F is in front. Each foreground pixel blocks (1−F.a) of the
background. Where F is fully opaque (F.a = 1), the background is completely
hidden. Where F is fully transparent (F.a = 0), only the background shows.

Example: place a logo watermark (fg) over a photograph (bg):

```
fg:  logo at 80% opacity, white pixels where logo is, transparent everywhere else
bg:  photograph (all pixels A=255)

At a logo pixel:   out = fg × 1.0   + bg × (1 − 0.8) = fg + bg × 0.2
At a non-logo px:  out = fg × 1.0   + bg × (1 − 0.0) = bg  (passthrough)
```

### 3.2 `in` and `out` (masking)

`in`: keeps only the parts of F that are covered by B. Used to clip a layer
to a mask shape stored in B's alpha channel:

```
out.rgb = F.rgb * B.a
out.a   = F.a   * B.a
```

`out` (not to be confused with "output"): keeps only the parts of F that are
*not* covered by B. Used to punch holes:

```
out.rgb = F.rgb * (1 − B.a)
out.a   = F.a   * (1 − B.a)
```

---

## 4. Blend Modes

Porter-Duff operators control *which* pixels survive. **Blend modes** control
*how* the surviving colours interact. Blend modes replace the simple additive
mix `F.rgb + B.rgb * (1 − F.a)` with a different colour-combining function
while keeping the Porter-Duff alpha compositing structure.

The standard formulation (after Adobe's documentation, used in Photoshop, CSS,
SVG, PDF):

```
// Apply blend mode B to foreground F and background B, both straight-alpha linear f32:
blended_rgb = blend_fn(F.rgb, B.rgb)   ← mode-specific function (see below)

// Then composite with over:
out.rgb = F.a * blended_rgb + (1 − F.a) * B.rgb
out.a   = F.a + B.a * (1 − F.a)
```

All blend mode functions below operate on single channel values in [0, 1]
in **linear light**.

### 4.1 Normal (default)

```
blend(Fc, Bc) = Fc
```

The foreground colour completely replaces the background colour (within the
foreground's alpha footprint). This is equivalent to the `over` operator.

### 4.2 Multiply

```
blend(Fc, Bc) = Fc * Bc
```

Both colours are multiplied. White (1.0) is neutral. Black (0.0) always
produces black. The result is always darker than or equal to both inputs.

Use case: darken, drop shadows, colour toning. A greyscale texture layer set
to Multiply tints the layer below it.

```
Example: 0.8 (light grey) × 0.5 (medium grey) = 0.4 (darker grey) ✓
```

### 4.3 Screen

```
blend(Fc, Bc) = 1 − (1 − Fc) * (1 − Bc)
              = Fc + Bc − Fc * Bc
```

The complement of Multiply applied to the complements. Black (0.0) is neutral.
White (1.0) always produces white. The result is always lighter than or equal
to both inputs.

Use case: lighten, glows, highlights.

Screen and Multiply are duals: `screen(1−Fc, 1−Bc) = 1 − multiply(Fc, Bc)`.

### 4.4 Overlay

```
blend(Fc, Bc) =
  if Bc < 0.5:  2 * Fc * Bc             (Multiply for dark backgrounds)
  else:         1 − 2*(1−Fc)*(1−Bc)     (Screen for light backgrounds)
```

Increases contrast. Dark areas become darker (Multiply); light areas become
lighter (Screen). Neutral grey (0.5) has no effect.

### 4.5 Hard Light

Hard Light is Overlay with foreground and background swapped:

```
blend(Fc, Bc) =
  if Fc < 0.5:  2 * Fc * Bc
  else:         1 − 2*(1−Fc)*(1−Bc)
```

Useful for rendering specular highlights or strong directional light.

### 4.6 Soft Light

A gentler version of Overlay:

```
blend(Fc, Bc) =
  if Fc < 0.5:
      Bc − (1 − 2*Fc) * Bc * (1 − Bc)
  else:
      Bc + (2*Fc − 1) * (D(Bc) − Bc)

  where D(Bc) = (Bc <= 0.25) ? ((16*Bc − 12)*Bc + 4)*Bc : sqrt(Bc)
```

Produces a soft, diffused light effect. Less harsh at the midpoint than
Overlay.

### 4.7 Difference and Exclusion

```
Difference:  blend(Fc, Bc) = |Fc − Bc|
Exclusion:   blend(Fc, Bc) = Fc + Bc − 2*Fc*Bc
```

Difference: identical colours cancel to black; complementary colours produce
white. Used for alignment checking (if two layers perfectly overlap, the
difference is black).

Exclusion: similar to Difference but lower contrast in the midtones.

### 4.8 Dodge and Burn

```
Color Dodge:  blend(Fc, Bc) = min(1, Bc / max(1 − Fc, ε))
Color Burn:   blend(Fc, Bc) = 1 − min(1, (1 − Bc) / max(Fc, ε))
```

Dodge brightens (simulates light hitting the image). Burn darkens.
The `ε` guard (e.g. 1×10⁻⁷) prevents division by zero.

### 4.9 Lighten / Darken

```
Lighten:  blend(Fc, Bc) = max(Fc, Bc)
Darken:   blend(Fc, Bc) = min(Fc, Bc)
```

Per-channel max/min. Simple but can produce hue shifts because each channel
is selected independently.

### 4.10 Blend mode summary table

```
Mode         Neutral value  Effect on midtones    Typical use
────────────────────────────────────────────────────────────────
Normal       —              Foreground replaces   Default layer
Multiply     White (1.0)    Darkens               Shadow, tint
Screen       Black (0.0)    Lightens              Glow, highlight
Overlay      Grey  (0.5)    Increases contrast    Texture overlay
Soft Light   Grey  (0.5)    Gentle contrast       Soft lighting
Hard Light   Grey  (0.5)    Strong contrast       Specular
Difference   Black (0.0)    Inverts on overlap    Alignment check
Exclusion    Black (0.0)    Low-contrast diff     Artistic effect
Color Dodge  Black (0.0)    Lightens aggressively Bloom, dodge
Color Burn   White (1.0)    Darkens aggressively  Burn tool
Lighten      Black (0.0)    Pass-through light    Brightening mask
Darken       White (1.0)    Pass-through dark     Darkening mask
```

---

## 5. Multi-Layer Compositing

Real documents are stacks of layers. The standard compositing pipeline
processes layers bottom-to-top, accumulating a running result:

```
result = layer[0]                                 // bottom layer (background)
for i in 1..layers.len():
    result = composite(layers[i], result, op, blend_mode)
```

where `composite` applies the blend mode and Porter-Duff operator.

### Layer model

```
Layer {
    pixels:     Image          // RGBA8 straight alpha
    opacity:    f32                     // [0.0, 1.0] — global layer opacity
    blend_mode: BlendMode               // how this layer combines with below
    visible:    bool
    clip_mask:  Option<Image>  // optional alpha mask (greyscale)
}
```

The global opacity scales the layer's per-pixel alpha before compositing:

```
effective_alpha = pixel.a * layer.opacity
```

The clip mask (if present) further modulates: the final alpha used for
compositing is `pixel.a * layer.opacity * mask_value(x, y)`.

### Performance: rendering order vs. evaluation order

In an interactive editing system, layers change one at a time. Re-compositing
the entire stack from scratch on every edit is wasteful. Standard optimisation:
cache the **composite buffer** at each layer boundary and re-composite only
the layers above the changed one.

This is an implementation concern, not a spec mandate. The spec defines
semantics; the runtime chooses the evaluation strategy.

---

## 6. Masking

A **mask** restricts where a compositing operation takes effect. In this spec,
masks are represented as greyscale `Image` images:

```
Mask pixel value:  0   → operation has no effect at this position
                  255  → operation has full effect
               1-254   → partial effect (blended)
```

The mask is applied to the foreground alpha before compositing:

```
fg_alpha_effective(x, y) = fg.A(x, y) * mask(x, y) / 255
```

Masks enable:

- **Gradient masks**: fade out an effect across the image
- **Shape masks**: apply an effect only within a shape boundary
- **Luminance masks**: use the brightness of one image to control blending
  in another (popular in photo editing for tone-specific adjustments)

---

## 7. The Compositing Pipeline

Full compositing pipeline for a single foreground+background pair:

```
Input:
  fg: Image (RGBA8, straight alpha, sRGB)
  bg: Image (RGBA8, straight alpha, sRGB)
  mode: BlendMode
  op: PorterDuff
  opacity: f32
  mask: Option<Image>

Step 1: Decode to linear f32
  fg_lin = decode_srgb_to_linear_f32(fg)    → RGBA f32, straight alpha
  bg_lin = decode_srgb_to_linear_f32(bg)

Step 2: Apply opacity and mask to foreground alpha
  for each pixel (x, y):
      fg_lin.A *= opacity
      if mask:  fg_lin.A *= mask.pixel(x, y) / 255.0

Step 3: Straight → premultiplied
  fg_pre = premultiply(fg_lin)
  bg_pre = premultiply(bg_lin)

Step 4: Apply blend mode to get blended colour
  blended_rgb = blend_mode_fn(fg_lin.rgb, bg_lin.rgb)   ← straight alpha inputs

Step 5: Apply Porter-Duff alpha operator
  (fF, fB) = porter_duff_factors(op, fg_pre.a, bg_pre.a)
  out_pre.rgb = blended_rgb * fg_pre.a * fF + bg_pre.rgb * fB
  // For Normal blend mode, simplifies to: fg_pre.rgb * fF + bg_pre.rgb * fB
  out_pre.a   = fg_pre.a * fF + bg_pre.a * fB

Step 6: Premultiplied → straight
  out_straight = unpremultiply(out_pre)

Step 7: Encode linear f32 → sRGB u8
  result = encode_linear_to_srgb_u8(out_straight)
```

Note on steps 4 and 5: the blend mode operates on straight-alpha colour
values (so the colour formula is not contaminated by premultiplied alpha),
while Porter-Duff operates on premultiplied values (for numerical stability).
This is the standard split used by the SVG compositing specification.

---

## 8. Special Case: Flat Alpha Blend

When the blend mode is Normal and the Porter-Duff operator is `over`, the
pipeline simplifies to the familiar "alpha blend" formula, expressed entirely
in linear f32:

```
// Both straight alpha, linear light:
a = fg.a × opacity     (effective foreground alpha)

out.r = fg.r * a + bg.r * (1 − a)
out.g = fg.g * a + bg.g * (1 − a)
out.b = fg.b * a + bg.b * (1 − a)
out.a = a + bg.a * (1 − a)
```

This is the hottest code path in any rendering system. At 1080p × 24 fps that
is 2.07 × 10⁸ pixels/second. SIMD is essential: process 4 RGBA pixels per
128-bit vector, 8 per 256-bit AVX2 vector.

---

## 9. Connection to the Paint Stack

The PaintVM (P2D01) composites layers via this spec's compositing pipeline.
When the VM processes a `PaintLayer` or `PaintGroup` instruction, it:

1. Renders each child into a temporary `Image`
2. Composites that temporary buffer onto the running result using the layer's
   blend mode, opacity, and mask

The compositing spec is the bridge between the abstract paint instruction
stream and the final flat pixel buffer handed to the OS display layer.

---

## 10. Interface

```
// Single composite (foreground over background):
fn composite(
    fg:         &Image,
    bg:         &Image,
    op:         PorterDuff,
    blend_mode: BlendMode,
    opacity:    f32,
    mask:       Option<&Image>,
) -> Image

// Convenience: Normal blend, over operator, full opacity, no mask:
fn alpha_blend(fg: &Image, bg: &Image) -> Image
fn alpha_blend_with_opacity(fg: &Image, bg: &Image, opacity: f32) -> Image

// Multi-layer compositing:
fn composite_layers(layers: &[Layer], background: &Image) -> Image

// Alpha manipulation:
fn premultiply(src: &Image) -> Image
fn unpremultiply(src: &Image) -> Image
fn apply_mask(src: &Image, mask: &Image) -> Image
fn fill_alpha(src: &Image, alpha: u8) -> Image
fn clear_colour_outside_mask(src: &Image, mask: &Image) -> Image

// enums:
enum PorterDuff {
    Clear, Copy, Destination,
    Over, In, Out, Atop, Xor, Lighter,
    DestinationOver, DestinationIn, DestinationOut, DestinationAtop,
}

enum BlendMode {
    Normal, Multiply, Screen, Overlay,
    SoftLight, HardLight,
    Difference, Exclusion,
    ColorDodge, ColorBurn,
    Lighten, Darken,
}

struct Layer {
    pixels:     Image,
    opacity:    f32,
    blend_mode: BlendMode,
    visible:    bool,
    clip_mask:  Option<Image>,
}
```
