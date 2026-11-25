//! Canonical deterministic SHA-256 hashing of `PaintScene` and PCM audio buffers (`SWF07`).
//!
//! Ensures bit-exact cross-platform digests for headless CI verification by:
//! - Normalizing `-0.0_f64` to `+0.0_f64` prior to byte encoding.
//! - Sorting optional metadata `HashMap` entries lexicographically before hashing.
//! - Length-prefixing every string, slice, and instruction stream.

#![forbid(unsafe_code)]

use std::collections::HashMap;

use coding_adventures_sha256::Sha256Hasher;
use paint_instructions::{
    BlendMode, FillRule, FilterEffect, GradientKind, GradientStop, ImageSrc, PaintBase, PaintClip,
    PaintEllipse, PaintGlyphRun, PaintGradient, PaintGroup, PaintImage, PaintInstruction,
    PaintLayer, PaintLine, PaintPath, PaintRect, PaintScene, PaintText, PathCommand, StrokeCap,
    StrokeJoin, TextAlign,
};

/// Maximum recursion depth when hashing nested `PaintGroup` / `PaintLayer` / `PaintClip` trees (CWE-674).
pub const MAX_SCENE_DIGEST_DEPTH: usize = 128;

/// 32-byte canonical SHA-256 scene digest wrapper (avoids raw `[u8; 32]` in `pub fn` signatures).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneDigest {
    bytes: [u8; 32],
}

impl SceneDigest {
    /// Returns the digest byte at `index` (`0..32`), or `None` if out of bounds.
    #[inline]
    pub fn digest_byte_at(&self, index: usize) -> Option<u8> {
        self.bytes.get(index).copied()
    }

    /// Borrows the 32-byte SHA-256 digest as a slice.
    #[inline]
    pub fn digest_as_slice(&self) -> &[u8] {
        &self.bytes
    }

    /// Formats the digest as lowercase 64-character hexadecimal.
    pub fn digest_to_hex(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(64);
        for &b in &self.bytes {
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0x0F) as usize] as char);
        }
        out
    }
}

#[inline]
fn hash_u8(h: &mut Sha256Hasher, v: u8) {
    h.update(&[v]);
}

#[inline]
fn hash_u32(h: &mut Sha256Hasher, v: u32) {
    h.update(&v.to_le_bytes());
}

#[inline]
fn hash_u64(h: &mut Sha256Hasher, v: u64) {
    h.update(&v.to_le_bytes());
}

#[inline]
fn hash_f64(h: &mut Sha256Hasher, v: f64) {
    let norm = if v.is_nan() {
        f64::NAN
    } else if v == 0.0 {
        0.0
    } else {
        v
    };
    h.update(&norm.to_bits().to_le_bytes());
}

fn hash_str(h: &mut Sha256Hasher, s: &str) {
    hash_u64(h, s.len() as u64);
    h.update(s.as_bytes());
}

fn hash_opt_str(h: &mut Sha256Hasher, s: &Option<String>) {
    match s {
        Some(val) => {
            hash_u8(h, 1);
            hash_str(h, val);
        }
        None => hash_u8(h, 0),
    }
}

fn hash_opt_f64(h: &mut Sha256Hasher, v: Option<f64>) {
    match v {
        Some(val) => {
            hash_u8(h, 1);
            hash_f64(h, val);
        }
        None => hash_u8(h, 0),
    }
}

fn hash_opt_dash(h: &mut Sha256Hasher, dash: &Option<Vec<f64>>) {
    match dash {
        Some(items) => {
            hash_u8(h, 1);
            hash_u64(h, items.len() as u64);
            for &x in items {
                hash_f64(h, x);
            }
        }
        None => hash_u8(h, 0),
    }
}

fn hash_metadata(h: &mut Sha256Hasher, meta: &Option<HashMap<String, String>>) {
    match meta {
        Some(map) => {
            hash_u8(h, 1);
            let mut entries: Vec<(&String, &String)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            hash_u64(h, entries.len() as u64);
            for (k, v) in entries {
                hash_str(h, k);
                hash_str(h, v);
            }
        }
        None => hash_u8(h, 0),
    }
}

fn hash_base(h: &mut Sha256Hasher, base: &PaintBase) {
    hash_opt_str(h, &base.id);
    hash_metadata(h, &base.metadata);
}

fn hash_path_command(h: &mut Sha256Hasher, cmd: &PathCommand) {
    match cmd {
        PathCommand::MoveTo { x, y } => {
            hash_u8(h, 0);
            hash_f64(h, *x);
            hash_f64(h, *y);
        }
        PathCommand::LineTo { x, y } => {
            hash_u8(h, 1);
            hash_f64(h, *x);
            hash_f64(h, *y);
        }
        PathCommand::QuadTo { cx, cy, x, y } => {
            hash_u8(h, 2);
            hash_f64(h, *cx);
            hash_f64(h, *cy);
            hash_f64(h, *x);
            hash_f64(h, *y);
        }
        PathCommand::CubicTo {
            cx1,
            cy1,
            cx2,
            cy2,
            x,
            y,
        } => {
            hash_u8(h, 3);
            hash_f64(h, *cx1);
            hash_f64(h, *cy1);
            hash_f64(h, *cx2);
            hash_f64(h, *cy2);
            hash_f64(h, *x);
            hash_f64(h, *y);
        }
        PathCommand::ArcTo {
            rx,
            ry,
            x_rotation,
            large_arc,
            sweep,
            x,
            y,
        } => {
            hash_u8(h, 4);
            hash_f64(h, *rx);
            hash_f64(h, *ry);
            hash_f64(h, *x_rotation);
            hash_u8(h, u8::from(*large_arc));
            hash_u8(h, u8::from(*sweep));
            hash_f64(h, *x);
            hash_f64(h, *y);
        }
        PathCommand::Close => hash_u8(h, 5),
    }
}

fn hash_fill_rule(h: &mut Sha256Hasher, r: &Option<FillRule>) {
    match r {
        None => hash_u8(h, 0),
        Some(FillRule::NonZero) => hash_u8(h, 1),
        Some(FillRule::EvenOdd) => hash_u8(h, 2),
    }
}

fn hash_stroke_cap(h: &mut Sha256Hasher, c: &Option<StrokeCap>) {
    match c {
        None => hash_u8(h, 0),
        Some(StrokeCap::Butt) => hash_u8(h, 1),
        Some(StrokeCap::Round) => hash_u8(h, 2),
        Some(StrokeCap::Square) => hash_u8(h, 3),
    }
}

fn hash_stroke_join(h: &mut Sha256Hasher, j: &Option<StrokeJoin>) {
    match j {
        None => hash_u8(h, 0),
        Some(StrokeJoin::Miter) => hash_u8(h, 1),
        Some(StrokeJoin::Round) => hash_u8(h, 2),
        Some(StrokeJoin::Bevel) => hash_u8(h, 3),
    }
}

fn hash_blend_mode(h: &mut Sha256Hasher, m: &Option<BlendMode>) {
    let code = match m {
        None => 0,
        Some(BlendMode::Normal) => 1,
        Some(BlendMode::Multiply) => 2,
        Some(BlendMode::Screen) => 3,
        Some(BlendMode::Overlay) => 4,
        Some(BlendMode::Darken) => 5,
        Some(BlendMode::Lighten) => 6,
        Some(BlendMode::ColorDodge) => 7,
        Some(BlendMode::ColorBurn) => 8,
        Some(BlendMode::HardLight) => 9,
        Some(BlendMode::SoftLight) => 10,
        Some(BlendMode::Difference) => 11,
        Some(BlendMode::Exclusion) => 12,
        Some(BlendMode::Hue) => 13,
        Some(BlendMode::Saturation) => 14,
        Some(BlendMode::Color) => 15,
        Some(BlendMode::Luminosity) => 16,
    };
    hash_u8(h, code);
}

fn hash_filter_effect(h: &mut Sha256Hasher, f: &FilterEffect) {
    match f {
        FilterEffect::Blur { radius } => {
            hash_u8(h, 0);
            hash_f64(h, *radius);
        }
        FilterEffect::DropShadow {
            dx,
            dy,
            blur,
            color,
        } => {
            hash_u8(h, 1);
            hash_f64(h, *dx);
            hash_f64(h, *dy);
            hash_f64(h, *blur);
            hash_str(h, color);
        }
        FilterEffect::ColorMatrix { matrix } => {
            hash_u8(h, 2);
            hash_u64(h, matrix.len() as u64);
            for &x in matrix {
                hash_f64(h, x);
            }
        }
        FilterEffect::Brightness { amount } => {
            hash_u8(h, 3);
            hash_f64(h, *amount);
        }
        FilterEffect::Contrast { amount } => {
            hash_u8(h, 4);
            hash_f64(h, *amount);
        }
        FilterEffect::Saturate { amount } => {
            hash_u8(h, 5);
            hash_f64(h, *amount);
        }
        FilterEffect::HueRotate { angle } => {
            hash_u8(h, 6);
            hash_f64(h, *angle);
        }
        FilterEffect::Invert { amount } => {
            hash_u8(h, 7);
            hash_f64(h, *amount);
        }
        FilterEffect::Opacity { amount } => {
            hash_u8(h, 8);
            hash_f64(h, *amount);
        }
    }
}

fn hash_transform(h: &mut Sha256Hasher, t: &Option<[f64; 6]>) {
    match t {
        Some(mat) => {
            hash_u8(h, 1);
            for &x in mat {
                hash_f64(h, x);
            }
        }
        None => hash_u8(h, 0),
    }
}

fn hash_instruction(h: &mut Sha256Hasher, instr: &PaintInstruction, depth: usize) {
    if depth >= MAX_SCENE_DIGEST_DEPTH {
        hash_u8(h, 0xFE);
        hash_u64(h, depth as u64);
        return;
    }
    match instr {
        PaintInstruction::Rect(PaintRect {
            base,
            x,
            y,
            width,
            height,
            fill,
            stroke,
            stroke_width,
            corner_radius,
            stroke_dash,
            stroke_dash_offset,
        }) => {
            hash_u8(h, 1);
            hash_base(h, base);
            hash_f64(h, *x);
            hash_f64(h, *y);
            hash_f64(h, *width);
            hash_f64(h, *height);
            hash_opt_str(h, fill);
            hash_opt_str(h, stroke);
            hash_opt_f64(h, *stroke_width);
            hash_opt_f64(h, *corner_radius);
            hash_opt_dash(h, stroke_dash);
            hash_opt_f64(h, *stroke_dash_offset);
        }
        PaintInstruction::Ellipse(PaintEllipse {
            base,
            cx,
            cy,
            rx,
            ry,
            fill,
            stroke,
            stroke_width,
            stroke_dash,
            stroke_dash_offset,
        }) => {
            hash_u8(h, 2);
            hash_base(h, base);
            hash_f64(h, *cx);
            hash_f64(h, *cy);
            hash_f64(h, *rx);
            hash_f64(h, *ry);
            hash_opt_str(h, fill);
            hash_opt_str(h, stroke);
            hash_opt_f64(h, *stroke_width);
            hash_opt_dash(h, stroke_dash);
            hash_opt_f64(h, *stroke_dash_offset);
        }
        PaintInstruction::Path(PaintPath {
            base,
            commands,
            fill,
            fill_rule,
            stroke,
            stroke_width,
            stroke_cap,
            stroke_join,
            stroke_dash,
            stroke_dash_offset,
        }) => {
            hash_u8(h, 3);
            hash_base(h, base);
            hash_u64(h, commands.len() as u64);
            for cmd in commands {
                hash_path_command(h, cmd);
            }
            hash_opt_str(h, fill);
            hash_fill_rule(h, fill_rule);
            hash_opt_str(h, stroke);
            hash_opt_f64(h, *stroke_width);
            hash_stroke_cap(h, stroke_cap);
            hash_stroke_join(h, stroke_join);
            hash_opt_dash(h, stroke_dash);
            hash_opt_f64(h, *stroke_dash_offset);
        }
        PaintInstruction::Text(PaintText {
            base,
            x,
            y,
            text,
            font_ref,
            font_size,
            fill,
            text_align,
        }) => {
            hash_u8(h, 4);
            hash_base(h, base);
            hash_f64(h, *x);
            hash_f64(h, *y);
            hash_str(h, text);
            hash_opt_str(h, font_ref);
            hash_f64(h, *font_size);
            hash_opt_str(h, fill);
            let align_code = match text_align {
                None => 0,
                Some(TextAlign::Left) => 1,
                Some(TextAlign::Center) => 2,
                Some(TextAlign::Right) => 3,
            };
            hash_u8(h, align_code);
        }
        PaintInstruction::GlyphRun(PaintGlyphRun {
            base,
            glyphs,
            font_ref,
            font_size,
            fill,
        }) => {
            hash_u8(h, 5);
            hash_base(h, base);
            hash_u64(h, glyphs.len() as u64);
            for g in glyphs {
                hash_u32(h, g.glyph_id);
                hash_f64(h, g.x);
                hash_f64(h, g.y);
            }
            hash_str(h, font_ref);
            hash_f64(h, *font_size);
            hash_opt_str(h, fill);
        }
        PaintInstruction::Group(PaintGroup {
            base,
            children,
            transform,
            opacity,
        }) => {
            hash_u8(h, 6);
            hash_base(h, base);
            hash_transform(h, transform);
            hash_opt_f64(h, *opacity);
            hash_u64(h, children.len() as u64);
            for child in children {
                hash_instruction(h, child, depth + 1);
            }
        }
        PaintInstruction::Layer(PaintLayer {
            base,
            children,
            filters,
            blend_mode,
            opacity,
            transform,
        }) => {
            hash_u8(h, 7);
            hash_base(h, base);
            match filters {
                Some(fs) => {
                    hash_u8(h, 1);
                    hash_u64(h, fs.len() as u64);
                    for f in fs {
                        hash_filter_effect(h, f);
                    }
                }
                None => hash_u8(h, 0),
            }
            hash_blend_mode(h, blend_mode);
            hash_opt_f64(h, *opacity);
            hash_transform(h, transform);
            hash_u64(h, children.len() as u64);
            for child in children {
                hash_instruction(h, child, depth + 1);
            }
        }
        PaintInstruction::Line(PaintLine {
            base,
            x1,
            y1,
            x2,
            y2,
            stroke,
            stroke_width,
            stroke_cap,
            stroke_dash,
            stroke_dash_offset,
        }) => {
            hash_u8(h, 8);
            hash_base(h, base);
            hash_f64(h, *x1);
            hash_f64(h, *y1);
            hash_f64(h, *x2);
            hash_f64(h, *y2);
            hash_str(h, stroke);
            hash_opt_f64(h, *stroke_width);
            hash_stroke_cap(h, stroke_cap);
            hash_opt_dash(h, stroke_dash);
            hash_opt_f64(h, *stroke_dash_offset);
        }
        PaintInstruction::Clip(PaintClip {
            base,
            x,
            y,
            width,
            height,
            path,
            children,
        }) => {
            hash_u8(h, 9);
            hash_base(h, base);
            hash_f64(h, *x);
            hash_f64(h, *y);
            hash_f64(h, *width);
            hash_f64(h, *height);
            match path {
                Some(cmds) => {
                    hash_u8(h, 1);
                    hash_u64(h, cmds.len() as u64);
                    for cmd in cmds {
                        hash_path_command(h, cmd);
                    }
                }
                None => hash_u8(h, 0),
            }
            hash_u64(h, children.len() as u64);
            for child in children {
                hash_instruction(h, child, depth + 1);
            }
        }
        PaintInstruction::Gradient(PaintGradient { base, kind, stops }) => {
            hash_u8(h, 10);
            hash_base(h, base);
            match kind {
                GradientKind::Linear { x1, y1, x2, y2 } => {
                    hash_u8(h, 0);
                    hash_f64(h, *x1);
                    hash_f64(h, *y1);
                    hash_f64(h, *x2);
                    hash_f64(h, *y2);
                }
                GradientKind::Radial { cx, cy, r } => {
                    hash_u8(h, 1);
                    hash_f64(h, *cx);
                    hash_f64(h, *cy);
                    hash_f64(h, *r);
                }
            }
            hash_u64(h, stops.len() as u64);
            for GradientStop { offset, color } in stops {
                hash_f64(h, *offset);
                hash_str(h, color);
            }
        }
        PaintInstruction::Image(PaintImage {
            base,
            x,
            y,
            width,
            height,
            src,
            opacity,
        }) => {
            hash_u8(h, 11);
            hash_base(h, base);
            hash_f64(h, *x);
            hash_f64(h, *y);
            hash_f64(h, *width);
            hash_f64(h, *height);
            match src {
                ImageSrc::Uri(u) => {
                    hash_u8(h, 0);
                    hash_str(h, u);
                }
                ImageSrc::Pixels(pc) => {
                    hash_u8(h, 1);
                    hash_u32(h, pc.width);
                    hash_u32(h, pc.height);
                    hash_u64(h, pc.data.len() as u64);
                    h.update(&pc.data);
                }
            }
            hash_opt_f64(h, *opacity);
        }
    }
}

/// Computes the canonical deterministic SHA-256 digest of a [`PaintScene`].
pub fn compute_scene_digest(scene: &PaintScene) -> SceneDigest {
    let mut hasher = Sha256Hasher::new();
    hasher.update(b"SWF07_PAINT_SCENE_V1");
    hash_f64(&mut hasher, scene.width);
    hash_f64(&mut hasher, scene.height);
    hash_str(&mut hasher, &scene.background);
    hash_opt_str(&mut hasher, &scene.id);
    hash_metadata(&mut hasher, &scene.metadata);
    hash_u64(&mut hasher, scene.instructions.len() as u64);
    for instr in &scene.instructions {
        hash_instruction(&mut hasher, instr, 0);
    }
    SceneDigest {
        bytes: hasher.digest(),
    }
}

/// Computes the canonical lowercase hex SHA-256 digest of a [`PaintScene`].
pub fn compute_scene_digest_hex(scene: &PaintScene) -> String {
    compute_scene_digest(scene).digest_to_hex()
}

/// Computes the deterministic SHA-256 hex digest of a mono `i16` PCM audio sample buffer.
pub fn compute_pcm_digest_hex(samples: &[i16]) -> String {
    let mut hasher = Sha256Hasher::new();
    hasher.update(b"SWF07_PCM_AUDIO_V1");
    hash_u64(&mut hasher, samples.len() as u64);
    for &sample in samples {
        hasher.update(&sample.to_le_bytes());
    }
    hasher.hex_digest()
}
