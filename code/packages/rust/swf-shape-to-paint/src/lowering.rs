//! Lowering of SWF parsed shapes and reconstructed contours to `paint-instructions` (`P2D00`).

#![deny(unsafe_code)]

use bezier2d::{FlatteningError, QuadraticBezier};
use paint_instructions::{
    FillRule, GradientKind, GradientStop, PaintBase, PaintGradient, PaintInstruction, PaintPath,
    PaintScene, PathCommand,
};
use point2d::Point;

use crate::parser::ParsedShape;
use crate::reconstruction::{reconstruct_all_fills, reconstruct_all_strokes};
use crate::records::ShapeEdge;
use crate::styles::{SwfFillStyle, SwfGradient};

/// Evaluates a point along a curved edge at parameter `t` in [0, 1] using `bezier2d`.
pub fn evaluate_quad_edge(edge: &ShapeEdge, t: f64) -> (f64, f64) {
    let p0 = edge.edge_start_pixels();
    let p2 = edge.edge_end_pixels();
    let p1 = edge.edge_control_pixels().unwrap_or(p0);

    let quad = QuadraticBezier::new(
        Point::new(p0.0, p0.1),
        Point::new(p1.0, p1.1),
        Point::new(p2.0, p2.1),
    );
    let pt = quad.evaluate(t);
    (pt.x, pt.y)
}

/// Computes the exact bounding box `(min_x, min_y, max_x, max_y)` of an edge using `bezier2d`.
pub fn quad_edge_bounding_box(edge: &ShapeEdge) -> (f64, f64, f64, f64) {
    let p0 = edge.edge_start_pixels();
    let p2 = edge.edge_end_pixels();

    if let Some(ctrl) = edge.edge_control_pixels() {
        let quad = QuadraticBezier::new(
            Point::new(p0.0, p0.1),
            Point::new(ctrl.0, ctrl.1),
            Point::new(p2.0, p2.1),
        );
        let bbox = quad.bounding_box();
        (bbox.x, bbox.y, bbox.x + bbox.width, bbox.y + bbox.height)
    } else {
        (
            p0.0.min(p2.0),
            p0.1.min(p2.1),
            p0.0.max(p2.0),
            p0.1.max(p2.1),
        )
    }
}

/// Flattens a curved edge into a polyline using `bezier2d` adaptive subdivision.
pub fn flatten_quad_edge(
    edge: &ShapeEdge,
    tolerance: f64,
) -> Result<Vec<(f64, f64)>, FlatteningError> {
    let p0 = edge.edge_start_pixels();
    let p2 = edge.edge_end_pixels();

    if let Some(ctrl) = edge.edge_control_pixels() {
        let quad = QuadraticBezier::new(
            Point::new(p0.0, p0.1),
            Point::new(ctrl.0, ctrl.1),
            Point::new(p2.0, p2.1),
        );
        let poly = quad.to_polyline(tolerance)?;
        Ok(poly.into_iter().map(|p| (p.x, p.y)).collect())
    } else {
        Ok(vec![p0, p2])
    }
}

fn lower_gradient(grad: &SwfGradient, grad_id: String) -> (PaintInstruction, String) {
    let tx_px = grad.matrix.translate_x as f64 / 20.0;
    let ty_px = grad.matrix.translate_y as f64 / 20.0;
    let sx = grad.matrix.scale_x;
    let sy = grad.matrix.scale_y;
    let _r0 = grad.matrix.rotate_skew_0;
    let r1 = grad.matrix.rotate_skew_1;

    let nominal_extent = 819.2; // 16384 twips / 20

    let kind = match grad.kind {
        crate::styles::SwfGradientKind::Linear => {
            let x1 = -nominal_extent * sx + tx_px;
            let y1 = -nominal_extent * r1 + ty_px;
            let x2 = nominal_extent * sx + tx_px;
            let y2 = nominal_extent * r1 + ty_px;
            GradientKind::Linear { x1, y1, x2, y2 }
        }
        crate::styles::SwfGradientKind::Radial | crate::styles::SwfGradientKind::FocalRadial => {
            let r = nominal_extent * (sx * sx + r1 * r1).sqrt().max(sy.abs()).max(0.001);
            GradientKind::Radial {
                cx: tx_px,
                cy: ty_px,
                r,
            }
        }
    };

    let stops = grad
        .stops
        .iter()
        .map(|s| GradientStop {
            offset: s.grad_stop_offset(),
            color: s.color.color_to_css_string(),
        })
        .collect();

    let grad_instr = PaintInstruction::Gradient(PaintGradient {
        base: PaintBase {
            id: Some(grad_id.clone()),
            metadata: None,
        },
        kind,
        stops,
    });

    let fill_ref = format!("url(#{grad_id})");
    (grad_instr, fill_ref)
}

/// Lowers a parsed shape into a complete `paint-instructions::PaintScene`.
pub fn lower_shape_to_paint_scene(shape: &ParsedShape) -> PaintScene {
    let width_px = shape.bounds.rect_width_pixels().max(1.0);
    let height_px = shape.bounds.rect_height_pixels().max(1.0);

    let mut scene = PaintScene::new(width_px, height_px);
    scene.background = "transparent".to_string();

    let fill_groups = reconstruct_all_fills(shape);
    let stroke_runs = reconstruct_all_strokes(shape);

    let fill_rule = if shape.uses_non_zero_winding {
        FillRule::NonZero
    } else {
        FillRule::EvenOdd
    };

    // 1. Lower fills
    for (group_idx, fg) in fill_groups.iter().enumerate() {
        if fg.contours.is_empty() {
            continue;
        }

        let Some(fill_style) = shape.fill_styles.get((fg.style_index - 1) as usize) else {
            continue;
        };

        let mut commands = Vec::new();

        for contour in &fg.contours {
            if contour.segments.is_empty() {
                continue;
            }

            let start = contour.segments[0].edge_start_pixels();
            commands.push(PathCommand::MoveTo {
                x: start.0,
                y: start.1,
            });

            for seg in &contour.segments {
                let end = seg.edge_end_pixels();
                if let Some(ctrl) = seg.edge_control_pixels() {
                    commands.push(PathCommand::QuadTo {
                        cx: ctrl.0,
                        cy: ctrl.1,
                        x: end.0,
                        y: end.1,
                    });
                } else {
                    commands.push(PathCommand::LineTo { x: end.0, y: end.1 });
                }
            }

            if contour.is_closed {
                commands.push(PathCommand::Close);
            }
        }

        match fill_style {
            SwfFillStyle::Solid(color) => {
                scene.instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands,
                    fill: Some(color.color_to_css_string()),
                    fill_rule: Some(fill_rule.clone()),
                    stroke: None,
                    stroke_width: None,
                    stroke_cap: None,
                    stroke_join: None,
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            }
            SwfFillStyle::LinearGradient(grad)
            | SwfFillStyle::RadialGradient(grad)
            | SwfFillStyle::FocalGradient(grad) => {
                let grad_id = format!("shape_{}_fill_{}", shape.shape_id, group_idx);
                let (grad_instr, fill_ref) = lower_gradient(grad, grad_id);
                scene.instructions.push(grad_instr);

                scene.instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands,
                    fill: Some(fill_ref),
                    fill_rule: Some(fill_rule.clone()),
                    stroke: None,
                    stroke_width: None,
                    stroke_cap: None,
                    stroke_join: None,
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            }
            SwfFillStyle::RepeatingBitmap(bm)
            | SwfFillStyle::ClippedBitmap(bm)
            | SwfFillStyle::NonSmoothedRepeatingBitmap(bm)
            | SwfFillStyle::NonSmoothedClippedBitmap(bm) => {
                let fill_ref = format!("swf-bitmap://{}", bm.bitmap_id);
                scene.instructions.push(PaintInstruction::Path(PaintPath {
                    base: PaintBase::default(),
                    commands,
                    fill: Some(fill_ref),
                    fill_rule: Some(fill_rule.clone()),
                    stroke: None,
                    stroke_width: None,
                    stroke_cap: None,
                    stroke_join: None,
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            }
        }
    }

    // 2. Lower strokes
    for run in &stroke_runs {
        if run.edges.is_empty() {
            continue;
        }

        let Some(line_style) = shape.line_styles.get((run.line_style_index - 1) as usize) else {
            continue;
        };

        let mut commands = Vec::new();
        let start = run.edges[0].edge_start_pixels();
        commands.push(PathCommand::MoveTo {
            x: start.0,
            y: start.1,
        });

        for edge in &run.edges {
            let end = edge.edge_end_pixels();
            if let Some(ctrl) = edge.edge_control_pixels() {
                commands.push(PathCommand::QuadTo {
                    cx: ctrl.0,
                    cy: ctrl.1,
                    x: end.0,
                    y: end.1,
                });
            } else {
                commands.push(PathCommand::LineTo { x: end.0, y: end.1 });
            }
        }

        if run.is_closed {
            commands.push(PathCommand::Close);
        }

        let stroke_color = line_style.color.color_to_css_string();
        let stroke_width = line_style.line_width_pixels();
        let stroke_cap = line_style.cap_style.cap_to_paint_cap();
        let stroke_join = line_style.join_style.join_to_paint_join();

        scene.instructions.push(PaintInstruction::Path(PaintPath {
            base: PaintBase::default(),
            commands,
            fill: None,
            fill_rule: None,
            stroke: Some(stroke_color),
            stroke_width: Some(stroke_width),
            stroke_cap: Some(stroke_cap),
            stroke_join: Some(stroke_join),
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
    }

    scene
}
