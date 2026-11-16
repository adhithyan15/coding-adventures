//! Lowering of Flash display hierarchies to `paint-instructions::PaintScene` (`P2D00`).

use affine2d::Affine2D;
use paint_instructions::{
    PaintBase, PaintClip, PaintGroup, PaintInstruction, PaintRect, PaintScene, PaintText,
    PathCommand, TextAlign,
};
use swf_container::SwfMatrix;
use swf_shape_to_paint::lower_shape_to_paint_scene;

use crate::display_object::{DisplayObject, DisplaySlot};
use crate::error::DisplayListError;
use crate::movie_clip::{MovieClip, MAX_HIERARCHY_DEPTH};

/// Converts an SWF affine transformation matrix into an `Affine2D` matrix.
pub fn swf_matrix_to_affine(m: &SwfMatrix) -> Affine2D {
    Affine2D::new(
        m.scale_x,
        m.rotate_skew_0,
        m.rotate_skew_1,
        m.scale_y,
        m.translate_x as f64 / 20.0,
        m.translate_y as f64 / 20.0,
    )
}

/// Extracts path commands and bounds from a mask display object.
fn extract_mask_geometry(
    obj: &DisplayObject,
    hierarchy_depth: usize,
) -> (f64, f64, f64, f64, Option<Vec<PathCommand>>) {
    let bounds = obj
        .obj_bounds_pixels_bounded(hierarchy_depth)
        .unwrap_or((0.0, 0.0, 0.0, 0.0));
    let width = (bounds.2 - bounds.0).max(1.0);
    let height = (bounds.3 - bounds.1).max(1.0);

    let path_commands = match obj {
        DisplayObject::Shape(s) => {
            let scene = lower_shape_to_paint_scene(s);
            let mut commands = Vec::new();
            for inst in scene.instructions {
                if let PaintInstruction::Path(p) = inst {
                    commands.extend(p.commands);
                }
            }
            if commands.is_empty() {
                None
            } else {
                Some(commands)
            }
        }
        DisplayObject::Morph(m) => {
            if let Ok(s) = m.morph_evaluate_current() {
                let scene = lower_shape_to_paint_scene(&s);
                let mut commands = Vec::new();
                for inst in scene.instructions {
                    if let PaintInstruction::Path(p) = inst {
                        commands.extend(p.commands);
                    }
                }
                if commands.is_empty() {
                    None
                } else {
                    Some(commands)
                }
            } else {
                None
            }
        }
        _ => None,
    };

    (bounds.0, bounds.1, width, height, path_commands)
}

/// Recursively lowers a single display slot into a list of paint instructions.
pub fn lower_slot_to_instructions(
    slot: &DisplaySlot,
    parent_path: &str,
    hierarchy_depth: usize,
) -> Result<Vec<PaintInstruction>, DisplayListError> {
    if hierarchy_depth >= MAX_HIERARCHY_DEPTH {
        return Err(DisplayListError::HierarchyDepthExceeded);
    }

    if !slot.visible {
        return Ok(Vec::new());
    }

    let node_id = if let Some(ref name) = slot.name {
        format!("{parent_path}/slot_{}_{}", slot.depth, name)
    } else {
        format!("{parent_path}/slot_{}", slot.depth)
    };

    let local_affine = swf_matrix_to_affine(&slot.transform);
    let alpha = (slot.color_transform.a_mult as f64 / 256.0).clamp(0.0, 1.0);

    let inner_instructions = match &slot.object {
        DisplayObject::Shape(s) => {
            let shape_scene = lower_shape_to_paint_scene(s);
            shape_scene.instructions
        }
        DisplayObject::Morph(m) => {
            let evaluated = m.morph_evaluate_current()?;
            let shape_scene = lower_shape_to_paint_scene(&evaluated);
            shape_scene.instructions
        }
        DisplayObject::Text(t) => {
            let mut text_instructions = Vec::new();

            // Background rectangle if enabled
            if let Some(bg) = t.background_fill {
                let mut rect = PaintRect::filled(
                    t.bounds.0,
                    t.bounds.1,
                    t.bounds.2,
                    t.bounds.3,
                    &bg.color_to_css_string(),
                );
                rect.base = PaintBase {
                    id: Some(format!("{node_id}_bg")),
                    metadata: None,
                };
                text_instructions.push(PaintInstruction::Rect(rect));
            }

            // Border outline if enabled
            if t.has_border {
                let mut rect = PaintRect::filled(
                    t.bounds.0,
                    t.bounds.1,
                    t.bounds.2,
                    t.bounds.3,
                    "transparent",
                );
                rect.fill = None;
                rect.stroke = Some(t.border_color.color_to_css_string());
                rect.stroke_width = Some(1.0);
                rect.base = PaintBase {
                    id: Some(format!("{node_id}_border")),
                    metadata: None,
                };
                text_instructions.push(PaintInstruction::Rect(rect));
            }

            // Text glyphs
            let paint_text = PaintText {
                base: PaintBase {
                    id: Some(format!("{node_id}_text")),
                    metadata: None,
                },
                x: t.bounds.0,
                y: t.bounds.1 + t.font_size,
                text: t.text.clone(),
                font_ref: t.font_name.clone(),
                font_size: t.font_size,
                fill: Some(t.text_color.color_to_css_string()),
                text_align: Some(TextAlign::Left),
            };
            text_instructions.push(PaintInstruction::Text(paint_text));

            text_instructions
        }
        DisplayObject::Button(b) => {
            if let Some(active) = b.button_active_record() {
                let temp_slot = DisplaySlot {
                    depth: slot.depth,
                    character_id: None,
                    object: active.clone(),
                    transform: SwfMatrix::default_matrix(),
                    color_transform: slot.color_transform,
                    clip_depth: None,
                    ratio: None,
                    name: None,
                    visible: true,
                    blend_mode: slot.blend_mode,
                };
                lower_slot_to_instructions(&temp_slot, &node_id, hierarchy_depth + 1)?
            } else {
                Vec::new()
            }
        }
        DisplayObject::Clip(c) => lower_clip_to_instructions(c, &node_id, hierarchy_depth + 1)?,
    };

    if inner_instructions.is_empty() {
        return Ok(Vec::new());
    }

    // Wrap in a PaintGroup to apply transform and opacity
    let group = PaintGroup {
        base: PaintBase {
            id: Some(node_id),
            metadata: None,
        },
        children: inner_instructions,
        transform: Some(local_affine.to_array()),
        opacity: if (alpha - 1.0).abs() > 1e-4 {
            Some(alpha)
        } else {
            None
        },
    };

    Ok(vec![PaintInstruction::Group(group)])
}

/// Lowers all display slots in a movie clip in depth order, applying stencil clipping masks.
pub fn lower_clip_to_instructions(
    clip: &MovieClip,
    parent_path: &str,
    hierarchy_depth: usize,
) -> Result<Vec<PaintInstruction>, DisplayListError> {
    if hierarchy_depth >= MAX_HIERARCHY_DEPTH {
        return Err(DisplayListError::HierarchyDepthExceeded);
    }

    if !clip.visible {
        return Ok(Vec::new());
    }

    let slots = clip.clip_slots_depth_order();
    let mut instructions = Vec::new();

    let mut i = 0;
    while i < slots.len() {
        let slot = slots[i];

        if let Some(mask_clip_depth) = slot.clip_depth {
            if mask_clip_depth > slot.depth {
                // This slot acts as a stencil mask for subsequent sibling slots
                let (mx, my, mw, mh, mask_path) =
                    extract_mask_geometry(&slot.object, hierarchy_depth + 1);

                // Find and lower all slots masked by this clipping mask (depths slot.depth + 1 ..= mask_clip_depth)
                let mut masked_children = Vec::new();
                let mut j = i + 1;
                while j < slots.len() && slots[j].depth <= mask_clip_depth {
                    let child_insts =
                        lower_slot_to_instructions(slots[j], parent_path, hierarchy_depth + 1)?;
                    masked_children.extend(child_insts);
                    j += 1;
                }

                let clip_id = format!("{parent_path}/clip_mask_{}_{}", slot.depth, mask_clip_depth);
                let paint_clip = PaintClip {
                    base: PaintBase {
                        id: Some(clip_id),
                        metadata: None,
                    },
                    x: mx,
                    y: my,
                    width: mw,
                    height: mh,
                    path: mask_path,
                    children: masked_children,
                };
                instructions.push(PaintInstruction::Clip(paint_clip));

                i = j;
                continue;
            }
        }

        let slot_insts = lower_slot_to_instructions(slot, parent_path, hierarchy_depth + 1)?;
        instructions.extend(slot_insts);
        i += 1;
    }

    Ok(instructions)
}

/// Lowers an entire movie clip timeline into a complete `PaintScene`.
pub fn lower_movie_clip_to_scene(
    clip: &MovieClip,
    width_px: f64,
    height_px: f64,
    background: &str,
) -> Result<PaintScene, DisplayListError> {
    let mut scene = PaintScene::new(width_px.max(1.0), height_px.max(1.0));
    scene.background = background.to_string();
    scene.id = Some("flash_scene".to_string());

    let instructions = lower_clip_to_instructions(clip, "root", 0)?;
    scene.instructions = instructions;

    Ok(scene)
}
