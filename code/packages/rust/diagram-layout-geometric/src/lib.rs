//! # diagram-layout-geometric
//!
//! Layout engine for geometric diagrams (DG04).
//!
//! Geometric diagrams use absolute user coordinates — the layout engine only
//! resolves the canvas size (from an explicit `width`/`height` or from the
//! bounding box of all elements) and passes elements through unchanged.

use std::collections::{HashMap, HashSet};
use diagram_ir::{CubicCurveSegment, CynefinDiagram, GeoElement, GeometricDiagram, InfoDiagram, IshikawaDiagram, IshikawaLook, LayoutedCynefinBoundary, LayoutedCynefinDiagram, LayoutedCynefinDomain, LayoutedCynefinItem, LayoutedCynefinTransition, LayoutedGeometricDiagram, LayoutedInfoDiagram, LayoutedIshikawaBone, LayoutedIshikawaDiagram, LayoutedIshikawaHatch, LayoutedVennCircle, LayoutedVennDebugCell, LayoutedVennDebugCircle, LayoutedVennDiagram, LayoutedVennLabel, LayoutedWardleyDiagram, LayoutedWardleyEvolution, LayoutedWardleyForce, LayoutedWardleyLink, LayoutedWardleyNode, LayoutedWardleyPipeline, Point, VennDiagram, WardleyDiagram};

pub const VERSION: &str = "0.4.0";

const MARGIN: f64 = 20.0;

/// Match Mermaid 11.16.1's fixed Info SVG geometry.
pub fn layout_info(diagram: &InfoDiagram) -> LayoutedInfoDiagram {
    LayoutedInfoDiagram { width: 400.0, height: 100.0, label: format!("v{}", diagram.version), x: 0.0, y: 16.0, font_size: 32.0 }
}

/// Produce stable Venn geometry. Sizes affect radii; exact area-proportional
/// overlap optimization is deliberately outside this partial slice.
pub fn layout_venn(diagram: &VennDiagram) -> LayoutedVennDiagram {
    let width = diagram.config.width;
    let height = diagram.config.height;
    let padding = diagram.config.padding.min(width / 4.0).min(height / 4.0);
    let title_offset = if diagram.title.is_some() { 34.0 } else { 0.0 };
    let sets: Vec<_> = diagram.regions.iter().filter(|region| region.sets.len() == 1).collect();
    let max_size = sets.iter().map(|set| set.size).fold(1.0_f64, f64::max);
    let available = (width - padding * 2.0).min(height - title_offset - padding * 2.0).max(1.0);
    let orbit = available * 0.16;
    let base_radius = available * 0.25;
    let center_x = width / 2.0;
    let center_y = title_offset + (height - title_offset) / 2.0;
    let count = sets.len().max(1) as f64;
    let scale = width / 1600.0;
    let circles: Vec<_> = sets.iter().enumerate().map(|(index, region)| {
        let angle = if sets.len() == 2 { std::f64::consts::PI * index as f64 } else {
            -std::f64::consts::FRAC_PI_2 + std::f64::consts::TAU * index as f64 / count
        };
        LayoutedVennCircle {
            id: region.sets[0].clone(),
            label: region.label.clone().unwrap_or_else(|| region.sets[0].clone()),
            cx: center_x + angle.cos() * orbit, cy: center_y + angle.sin() * orbit,
            radius: base_radius * (0.72 + 0.28 * (region.size / max_size).sqrt()),
            style: region.style.clone(),
        }
    }).collect();
    let centers: HashMap<_, _> = circles.iter().map(|circle| (circle.id.as_str(), (circle.cx, circle.cy))).collect();
    let mut labels = Vec::new();
    for (index, circle) in circles.iter().enumerate() {
        let angle = if circles.len() == 2 { std::f64::consts::PI * index as f64 } else {
            -std::f64::consts::FRAC_PI_2 + std::f64::consts::TAU * index as f64 / count
        };
        labels.push(LayoutedVennLabel { text: circle.label.clone(),
            x: circle.cx + angle.cos() * circle.radius * 0.58,
            y: circle.cy + angle.sin() * circle.radius * 0.58, font_size: 48.0 * scale, style: circle.style.clone() });
    }
    for region in diagram.regions.iter().filter(|region| region.sets.len() > 1) {
        if let (Some(label), Some((x, y))) = (&region.label, centroid(&region.sets, &centers)) {
            labels.push(LayoutedVennLabel { text: label.clone(), x, y, font_size: 48.0 * scale, style: region.style.clone() });
        }
    }
    let mut text_groups = Vec::<(Vec<String>, Vec<&diagram_ir::VennText>)>::new();
    for text in &diagram.texts {
        if let Some((_, texts)) = text_groups.iter_mut().find(|(sets, _)| *sets == text.sets) {
            texts.push(text);
        } else {
            text_groups.push((text.sets.clone(), vec![text]));
        }
    }
    for (sets, texts) in text_groups {
        let Some((_, cells)) = venn_text_grid_geometry(diagram, &sets, texts.len(), &circles, &centers) else { continue; };
        for (text, cell) in texts.into_iter().zip(cells) {
            labels.push(LayoutedVennLabel {
                text: text.label.clone().unwrap_or_else(|| text.id.clone()),
                x: cell.x + cell.width / 2.0,
                y: cell.y + cell.height / 2.0,
                font_size: 40.0 * scale,
                style: text.style.clone(),
            });
        }
    }
    let (debug_circles, debug_cells) = if diagram.config.use_debug_layout {
        venn_debug_geometry(diagram, &circles, &centers)
    } else { (Vec::new(), Vec::new()) };
    LayoutedVennDiagram { width, height, title: diagram.title.clone(), config: diagram.config.clone(), circles, labels,
        debug_circles, debug_cells }
}

fn venn_debug_geometry(diagram: &VennDiagram, circles: &[LayoutedVennCircle], centers: &HashMap<&str, (f64, f64)>)
    -> (Vec<LayoutedVennDebugCircle>, Vec<LayoutedVennDebugCell>) {
    let mut seen = HashSet::new();
    let mut debug_circles = Vec::new();
    let mut debug_cells = Vec::new();
    for text in &diagram.texts {
        let key = text.sets.join("\0");
        if !seen.insert(key) { continue; }
        let count = diagram.texts.iter().filter(|node| node.sets == text.sets).count();
        if let Some((circle, cells)) = venn_text_grid_geometry(diagram, &text.sets, count, circles, centers) {
            debug_circles.push(circle);
            debug_cells.extend(cells);
        }
    }
    (debug_circles, debug_cells)
}

fn venn_text_grid_geometry(
    diagram: &VennDiagram,
    sets: &[String],
    count: usize,
    circles: &[LayoutedVennCircle],
    centers: &HashMap<&str, (f64, f64)>,
) -> Option<(LayoutedVennDebugCircle, Vec<LayoutedVennDebugCell>)> {
    let scale = diagram.config.width / 1600.0;
    let (cx, cy) = centroid(sets, centers)?;
    let area_circles = sets.iter().filter_map(|id|
        circles.iter().find(|circle| circle.id == *id)).collect::<Vec<_>>();
    let min_radius = area_circles.iter().map(|circle| circle.radius).fold(f64::INFINITY, f64::min);
    let mut inner_radius = area_circles.iter().map(|circle|
        circle.radius - (cx - circle.cx).hypot(cy - circle.cy))
        .fold(f64::INFINITY, f64::min).max(0.0);
    if inner_radius == 0.0 && min_radius.is_finite() { inner_radius = min_radius * 0.6; }
    let cols = (count as f64).sqrt().ceil().max(1.0) as usize;
    let rows = count.div_ceil(cols).max(1);
    let inner_width = (80.0 * scale).max(inner_radius * 1.9);
    let inner_height = (60.0 * scale).max(inner_radius * 1.9);
    let has_label = diagram.regions.iter().any(|region| region.sets == sets
        && region.label.as_ref().is_some_and(|label| !label.is_empty()));
    let label_offset = if has_label { (32.0 * scale).min(inner_radius * 0.25) } else { 0.0 }
        + if count <= 2 { 30.0 * scale } else { 0.0 };
    let cell_width = inner_width / cols as f64;
    let cell_height = inner_height / rows as f64;
    let start_x = cx - inner_width / 2.0;
    let start_y = cy - inner_height / 2.0 + label_offset;
    let cells = (0..count).map(|index| LayoutedVennDebugCell {
        x: start_x + cell_width * (index % cols) as f64,
        y: start_y + cell_height * (index / cols) as f64,
        width: cell_width,
        height: cell_height,
    }).collect();
    Some((LayoutedVennDebugCircle { cx, cy, radius: inner_radius }, cells))
}

fn centroid(sets: &[String], centers: &HashMap<&str, (f64, f64)>) -> Option<(f64, f64)> {
    let points: Vec<_> = sets.iter().filter_map(|set| centers.get(set.as_str())).collect();
    (!points.is_empty()).then(|| { let count = points.len() as f64;
        (points.iter().map(|point| point.0).sum::<f64>() / count,
         points.iter().map(|point| point.1).sum::<f64>() / count) })
}

/// Lay out an Ishikawa causal tree as an alternating fishbone.
pub fn layout_ishikawa(diagram: &IshikawaDiagram, canvas_width: f64) -> LayoutedIshikawaDiagram {
    let width = canvas_width.max(560.0);
    let font_scale = diagram.config.font_size / 16.0;
    let max_depth = diagram.causes.iter().map(|cause| cause.depth).max().unwrap_or(1) as f64;
    let height = (width * 0.58).max(360.0).max(260.0 + max_depth * 70.0);
    let padding = diagram.config.diagram_padding.min(width / 4.0).min(height / 4.0);
    let spine_y = height / 2.0; let spine_from = Point { x: padding + 16.0, y: spine_y };
    let effect_base_width = (diagram.effect.chars().count() as f64 * 7.0 + 6.0).max(60.0);
    let effect_width = effect_base_width * 2.4; let effect_height = 68.0; let effect_x = width - effect_width - padding;
    let spine_to = Point { x: effect_x, y: spine_y };
    let hand_drawn = diagram.config.look == IshikawaLook::HandDrawn;
    let spine_path = ishikawa_line_path(&spine_from, &spine_to, hand_drawn, diagram.config.hand_drawn_seed, 0);
    let effect_y = spine_y - effect_height / 2.0;
    let effect_outline = ishikawa_effect_outline(effect_x, effect_y, effect_width, effect_height,
        hand_drawn, diagram.config.hand_drawn_seed, 1_000);
    let effect_hatches = if hand_drawn { ishikawa_hatches(&effect_outline) } else { Vec::new() };
    let roots: Vec<_> = diagram.causes.iter().filter(|cause| cause.parent_id.is_none()).collect();
    let spacing = (spine_to.x - spine_from.x) / (roots.len() + 1).max(2) as f64;
    let mut bones = Vec::new(); let mut anchors = HashMap::<&str, Point>::new();
    for (index, cause) in roots.iter().enumerate() {
        let attach = Point { x: spine_from.x + spacing * (index + 1) as f64, y: spine_y };
        let sign = if index.is_multiple_of(2) { -1.0 } else { 1.0 };
        let vertical_reach = ((height / 2.0 - padding) * 0.72).max(40.0);
        let tip = Point { x: attach.x - spacing * 0.48, y: spine_y + sign * vertical_reach };
        let path = ishikawa_line_path(&tip, &attach, hand_drawn, diagram.config.hand_drawn_seed, index + 1);
        let arrowhead = ishikawa_arrowhead(&path);
        let label_position = Point { x: tip.x + spacing * 0.24, y: (tip.y + spine_y) / 2.0 + sign * 42.0 };
        let (label_width, label_height) = ishikawa_label_size(&cause.label, font_scale);
        let label_outline = ishikawa_box_outline(label_position.x - label_width / 2.0,
            label_position.y - label_height / 2.0, label_width, label_height, hand_drawn,
            diagram.config.hand_drawn_seed, 2_000 + index);
        let hatches = if hand_drawn { ishikawa_hatches(&label_outline) } else { Vec::new() };
        bones.push(LayoutedIshikawaBone { from: tip.clone(), to: attach, label: cause.label.clone(),
            label_position, label_width, label_height, depth: 1, path, arrowhead, label_outline, hatches });
        anchors.insert(&cause.id, tip);
    }
    for cause in diagram.causes.iter().filter(|cause| cause.parent_id.is_some()) {
        let parent = cause.parent_id.as_deref().and_then(|id| anchors.get(id)).cloned().unwrap_or(spine_from.clone());
        let sign = if parent.y < spine_y { -1.0 } else { 1.0 };
        let length = (90.0 / cause.depth as f64).max(38.0);
        let tip = Point { x: parent.x - length, y: parent.y + sign * length * 0.55 };
        let path = ishikawa_line_path(&tip, &parent, hand_drawn, diagram.config.hand_drawn_seed, bones.len() + 1);
        let arrowhead = ishikawa_arrowhead(&path);
        let label_position = Point { x: tip.x, y: tip.y - sign * 18.0 };
        let (label_width, label_height) = ishikawa_label_size(&cause.label, font_scale);
        let label_outline = ishikawa_box_outline(label_position.x - label_width / 2.0,
            label_position.y - label_height / 2.0, label_width, label_height, hand_drawn,
            diagram.config.hand_drawn_seed, 2_000 + bones.len());
        let hatches = if hand_drawn { ishikawa_hatches(&label_outline) } else { Vec::new() };
        bones.push(LayoutedIshikawaBone { from: tip.clone(), to: parent,
            label: cause.label.clone(), label_position, label_width, label_height, depth: cause.depth,
            path, arrowhead, label_outline, hatches });
        anchors.insert(&cause.id, tip);
    }
    resolve_ishikawa_label_collisions(&mut bones, spine_y, hand_drawn, diagram.config.hand_drawn_seed);
    let mut layout = LayoutedIshikawaDiagram { width, height, effect: diagram.effect.clone(), config: diagram.config.clone(), effect_x, effect_y,
        effect_width, effect_height, spine_from, spine_to, spine_path, effect_outline, effect_hatches, bones };
    fit_ishikawa_content(&mut layout);
    layout
}

fn fit_ishikawa_content(layout: &mut LayoutedIshikawaDiagram) {
    let mut min_x = layout.effect_x;
    let mut min_y = layout.effect_y;
    let mut max_x = layout.effect_x + layout.effect_width;
    let mut max_y = layout.effect_y + layout.effect_height;
    let mut include = |point: &Point| {
        min_x = min_x.min(point.x); min_y = min_y.min(point.y);
        max_x = max_x.max(point.x); max_y = max_y.max(point.y);
    };
    for point in layout.spine_path.iter().chain(&layout.effect_outline) { include(point); }
    for bone in &layout.bones {
        for point in bone.path.iter().chain(&bone.arrowhead).chain(&bone.label_outline) { include(point); }
        include(&Point { x: bone.label_position.x - bone.label_width / 2.0, y: bone.label_position.y - bone.label_height / 2.0 });
        include(&Point { x: bone.label_position.x + bone.label_width / 2.0, y: bone.label_position.y + bone.label_height / 2.0 });
    }
    let margin = layout.config.diagram_padding.max(4.0);
    let dx = (margin - min_x).max(0.0);
    let dy = (margin - min_y).max(0.0);
    if dx > 0.0 || dy > 0.0 {
        let shift = |point: &mut Point| { point.x += dx; point.y += dy; };
        shift(&mut layout.spine_from); shift(&mut layout.spine_to);
        for point in &mut layout.spine_path { shift(point); }
        for point in &mut layout.effect_outline { shift(point); }
        for hatch in &mut layout.effect_hatches { shift(&mut hatch.from); shift(&mut hatch.to); }
        layout.effect_x += dx; layout.effect_y += dy;
        for bone in &mut layout.bones {
            shift(&mut bone.from); shift(&mut bone.to); shift(&mut bone.label_position);
            for point in &mut bone.path { shift(point); }
            for point in &mut bone.arrowhead { shift(point); }
            for point in &mut bone.label_outline { shift(point); }
            for hatch in &mut bone.hatches { shift(&mut hatch.from); shift(&mut hatch.to); }
        }
    }
    layout.width = layout.width.max(max_x + dx + margin);
    layout.height = layout.height.max(max_y + dy + margin);
}

fn resolve_ishikawa_label_collisions(bones: &mut [LayoutedIshikawaBone], spine_y: f64, hand_drawn: bool, seed: i64) {
    for above in [true, false] {
        let mut indices: Vec<_> = bones.iter().enumerate()
            .filter_map(|(index, bone)| ((bone.label_position.y < spine_y) == above).then_some(index)).collect();
        indices.sort_by(|left, right| (bones[*left].label_position.y - spine_y).abs()
            .total_cmp(&(bones[*right].label_position.y - spine_y).abs()));
        let mut placed = Vec::<usize>::new();
        for index in indices {
            let half_width = bones[index].label_width / 2.0; let half_height = bones[index].label_height / 2.0;
            let mut target = bones[index].label_position.y;
            for other_index in &placed {
                let other = &bones[*other_index];
                let horizontal_overlap = bones[index].label_position.x + half_width + 4.0 > other.label_position.x - other.label_width / 2.0
                    && bones[index].label_position.x - half_width - 4.0 < other.label_position.x + other.label_width / 2.0;
                if !horizontal_overlap { continue; }
                if above && target + half_height + 4.0 > other.label_position.y - other.label_height / 2.0 {
                    target = target.min(other.label_position.y - other.label_height / 2.0 - half_height - 4.0);
                } else if !above && target - half_height - 4.0 < other.label_position.y + other.label_height / 2.0 {
                    target = target.max(other.label_position.y + other.label_height / 2.0 + half_height + 4.0);
                }
            }
            bones[index].label_position.y = target; placed.push(index);
        }
    }
    for (index, bone) in bones.iter_mut().enumerate() {
        bone.label_outline = ishikawa_box_outline(bone.label_position.x - bone.label_width / 2.0,
            bone.label_position.y - bone.label_height / 2.0, bone.label_width, bone.label_height,
            hand_drawn, seed, 2_000 + index);
        bone.hatches = if hand_drawn { ishikawa_hatches(&bone.label_outline) } else { Vec::new() };
    }
}

fn ishikawa_line_path(from: &Point, to: &Point, hand_drawn: bool, seed: i64, index: usize) -> Vec<Point> {
    if !hand_drawn { return vec![from.clone(), to.clone()]; }
    let dx = to.x - from.x; let dy = to.y - from.y; let length = dx.hypot(dy).max(1.0);
    let perpendicular = (-dy / length, dx / length);
    let mut state = (seed as u64).wrapping_add((index as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15));
    let mut offset = || { state ^= state << 13; state ^= state >> 7; state ^= state << 17; (state as f64 / u64::MAX as f64 - 0.5) * 6.0 };
    let mut points = vec![from.clone()];
    for ratio in [0.25, 0.5, 0.75] {
        let jitter = offset();
        points.push(Point { x: from.x + dx * ratio + perpendicular.0 * jitter,
            y: from.y + dy * ratio + perpendicular.1 * jitter });
    }
    points.push(to.clone()); points
}

fn ishikawa_arrowhead(path: &[Point]) -> Vec<Point> {
    let [.., before, tip] = path else { return Vec::new(); };
    let dx = tip.x - before.x; let dy = tip.y - before.y; let length = dx.hypot(dy).max(1.0);
    let unit = (dx / length, dy / length); let perpendicular = (-unit.1, unit.0);
    let base = Point { x: tip.x - unit.0 * 12.0, y: tip.y - unit.1 * 12.0 };
    vec![tip.clone(), Point { x: base.x + perpendicular.0 * 5.0, y: base.y + perpendicular.1 * 5.0 },
        Point { x: base.x - perpendicular.0 * 5.0, y: base.y - perpendicular.1 * 5.0 }]
}

fn ishikawa_box_outline(x: f64, y: f64, width: f64, height: f64, hand_drawn: bool, seed: i64, index: usize) -> Vec<Point> {
    let corners = [Point { x, y }, Point { x: x + width, y }, Point { x: x + width, y: y + height },
        Point { x, y: y + height }, Point { x, y }];
    let mut outline = vec![corners[0].clone()];
    for side in 0..4 {
        let points = ishikawa_line_path(&corners[side], &corners[side + 1], hand_drawn, seed, index * 4 + side);
        outline.extend(points.into_iter().skip(1));
    }
    outline
}

fn ishikawa_effect_outline(x: f64, y: f64, width: f64, height: f64, hand_drawn: bool, seed: i64, index: usize) -> Vec<Point> {
    let mut shape = vec![Point { x, y }, Point { x, y: y + height }];
    for step in 1..=12 {
        let t = step as f64 / 12.0; let one_minus_t = 1.0 - t;
        shape.push(Point { x: one_minus_t * one_minus_t * x + 2.0 * one_minus_t * t * (x + width) + t * t * x,
            y: one_minus_t * one_minus_t * (y + height) + 2.0 * one_minus_t * t * (y + height / 2.0) + t * t * y });
    }
    if !hand_drawn { return shape; }
    let mut outline = vec![shape[0].clone()];
    for side in 0..shape.len() - 1 {
        let points = ishikawa_line_path(&shape[side], &shape[side + 1], true, seed, index * 16 + side);
        outline.extend(points.into_iter().skip(1));
    }
    outline
}

fn ishikawa_hatches(outline: &[Point]) -> Vec<LayoutedIshikawaHatch> {
    let angle = -41.0_f64.to_radians(); let direction = (angle.cos(), angle.sin());
    let normal = (-direction.1, direction.0); let project = |point: &Point| point.x * normal.0 + point.y * normal.1;
    let minimum = outline.iter().map(&project).fold(f64::INFINITY, f64::min);
    let maximum = outline.iter().map(&project).fold(f64::NEG_INFINITY, f64::max);
    let mut hatches = Vec::new(); let mut offset = (minimum / 5.0).floor() * 5.0;
    while offset <= maximum {
        let mut intersections = Vec::new();
        for edge in outline.windows(2) {
            let from = project(&edge[0]) - offset; let to = project(&edge[1]) - offset;
            if from * to <= 0.0 && (from - to).abs() > 1e-9 {
                let ratio = from / (from - to);
                let point = Point { x: edge[0].x + (edge[1].x - edge[0].x) * ratio,
                    y: edge[0].y + (edge[1].y - edge[0].y) * ratio };
                intersections.push((point.x * direction.0 + point.y * direction.1, point));
            }
        }
        intersections.sort_by(|left, right| left.0.total_cmp(&right.0));
        intersections.dedup_by(|left, right| (left.0 - right.0).abs() < 1e-7);
        let pair_size = 2;
        for pair in intersections.chunks_exact(pair_size) {
            hatches.push(LayoutedIshikawaHatch { from: pair[0].1.clone(), to: pair[1].1.clone() });
        }
        offset += 5.0;
    }
    hatches
}

fn ishikawa_label_size(label: &str, font_scale: f64) -> (f64, f64) {
    let mut lines = Vec::<String>::new();
    for word in label.split_whitespace() {
        if lines.last().is_some_and(|line| line.chars().count() + 1 + word.chars().count() <= 15) {
            let line = lines.last_mut().expect("checked above");
            line.push(' '); line.push_str(word);
        } else {
            lines.push(word.to_string());
        }
    }
    if lines.is_empty() { lines.push(String::new()); }
    let max_chars = lines.iter().map(|line| line.chars().count()).max().unwrap_or(0) as f64;
    let width = (max_chars * 7.0 + 40.0).clamp(72.0, 240.0) * font_scale;
    let height = ((lines.len() as f64 * 16.0 * 1.05) + 4.0).max(28.0) * font_scale;
    (width, height)
}

/// Map Wardley visibility/evolution coordinates onto a deterministic Cartesian canvas.
pub fn layout_wardley(diagram: &WardleyDiagram) -> LayoutedWardleyDiagram {
    let width = diagram.config.width.max(420.0); let height = diagram.config.height.max(300.0);
    let left = diagram.config.padding.min(width / 3.0); let right = left;
    let top = diagram.config.padding.min(height / 3.0); let bottom = top;
    let point = |visibility: f64, evolution: f64| Point {
        x: left + evolution * (width - left - right), y: top + (1.0 - visibility) * (height - top - bottom),
    };
    let mut nodes: Vec<_> = diagram.nodes.iter().map(|node| LayoutedWardleyNode { id: node.id.clone(), label: node.label.clone(),
        position: point(node.visibility, node.evolution), anchor: node.anchor, pipeline_parent: node.pipeline_parent,
        label_offset_x: node.label_offset_x, label_offset_y: node.label_offset_y,
        source_strategy: node.source_strategy, inertia: node.inertia }).collect();
    let node_indices: HashMap<_, _> = nodes.iter().enumerate().map(|(index, node)| (node.id.clone(), index)).collect();
    let pipelines = diagram.pipelines.iter().filter_map(|pipeline| {
        let mut component_positions: Vec<_> = pipeline.component_ids.iter().filter_map(|id| {
            nodes.get(*node_indices.get(id.as_str())?).map(|node| node.position.clone())
        }).collect();
        component_positions.sort_by(|left, right| left.x.total_cmp(&right.x));
        let first = component_positions.first()?; let last = component_positions.last()?;
        let height = diagram.config.node_radius * 4.0; let y = first.y - height / 2.0;
        if let Some(parent_index) = node_indices.get(pipeline.parent_id.as_str()) {
            nodes[*parent_index].position = Point { x: (first.x + last.x) / 2.0,
                y: y - diagram.config.node_radius * 1.6 / 6.0 };
        }
        Some(LayoutedWardleyPipeline { x: first.x - 15.0, y, width: last.x - first.x + 30.0, height, component_positions })
    }).collect();
    let positions: HashMap<_, _> = diagram.nodes.iter().zip(nodes.iter()).map(|(source, layout)| (source.label.as_str(), layout.position.clone())).collect();
    let layout_nodes: HashMap<_, _> = diagram.nodes.iter().zip(nodes.iter()).map(|(source, layout)| (source.label.as_str(), layout)).collect();
    let links = diagram.links.iter().filter_map(|link| {
        let source = layout_nodes.get(link.source.as_str())?; let target = layout_nodes.get(link.target.as_str())?;
        let dx = target.position.x - source.position.x; let dy = target.position.y - source.position.y;
        let distance = dx.hypot(dy); let (ux, uy) = if distance > f64::EPSILON { (dx / distance, dy / distance) } else { (0.0, 0.0) };
        let source_radius = if source.pipeline_parent { diagram.config.node_radius * 1.6 / 2.0_f64.sqrt() } else { diagram.config.node_radius };
        let target_radius = if target.pipeline_parent { diagram.config.node_radius * 1.6 / 2.0_f64.sqrt() } else { diagram.config.node_radius };
        let midpoint = Point { x: (source.position.x + target.position.x) / 2.0,
            y: (source.position.y + target.position.y) / 2.0 };
        let mut label_angle = dy.atan2(dx).to_degrees();
        if !(-90.0..=90.0).contains(&label_angle) { label_angle += 180.0; }
        Some(LayoutedWardleyLink {
            from: Point { x: source.position.x + ux * source_radius, y: source.position.y + uy * source_radius },
            to: Point { x: target.position.x - ux * target_radius, y: target.position.y - uy * target_radius },
            dashed: link.dashed, label: link.label.clone(),
            label_position: Point { x: midpoint.x + uy * 8.0, y: midpoint.y - ux * 8.0 }, label_angle,
            flow: link.flow,
        })
    }).collect();
    let evolves = diagram.evolves.iter().filter_map(|evolve| {
        let node = diagram.nodes.iter().find(|node| node.label == evolve.component)?;
        Some(LayoutedWardleyEvolution { from: positions.get(node.label.as_str())?.clone(), to: point(node.visibility, evolve.target) })
    }).collect();
    let notes = diagram.notes.iter().map(|note| diagram_ir::LayoutedWardleyNote {
        text: note.text.clone(), position: point(note.visibility, note.evolution) }).collect();
    let annotations_box = diagram.annotations_box.map(|(visibility, evolution)| point(visibility, evolution));
    let annotations = diagram.annotations.iter().map(|annotation| diagram_ir::LayoutedWardleyAnnotation {
        number: annotation.number, text: annotation.text.clone(),
        positions: annotation.coordinates.iter().map(|(visibility, evolution)| point(*visibility, *evolution)).collect(),
    }).collect();
    let forces = diagram.forces.iter().map(|force| LayoutedWardleyForce {
        label: force.label.clone(), position: point(force.visibility, force.evolution), kind: force.kind,
    }).collect();
    LayoutedWardleyDiagram { width, height, title: diagram.title.clone(),
        accessibility_title: diagram.accessibility_title.clone(), accessibility_description: diagram.accessibility_description.clone(),
        config: diagram.config.clone(),
        stages: diagram.stages.clone(), stage_boundaries: diagram.stage_boundaries.clone(), nodes, links, evolves, notes,
        annotations_box, annotations, forces, pipelines }
}

/// Place the four Cynefin quadrants and central confusion domain deterministically.
pub fn layout_cynefin(diagram: &CynefinDiagram) -> LayoutedCynefinDiagram {
    let width = diagram.config.width + diagram.config.padding * 2.0;
    let height = diagram.config.height + diagram.config.padding * 2.0;
    let top = diagram.config.padding; let left = diagram.config.padding;
    let domain_width = diagram.config.width / 2.0;
    let domain_height = diagram.config.height / 2.0;
    let specs = [("complex", left, top), ("complicated", left + domain_width, top),
        ("chaotic", left, top + domain_height), ("clear", left + domain_width, top + domain_height)];
    let mut domains: Vec<_> = specs.into_iter().map(|(name, x, y)| LayoutedCynefinDomain { name: name.into(),
        items: diagram.domains.iter().find(|domain| domain.name == name).map_or_else(Vec::new, |domain| domain.items.clone()),
        item_badges: Vec::new(), overflow_count: 0, x, y, width: domain_width, height: domain_height,
        center: Point { x: x + domain_width / 2.0, y: y + domain_height / 2.0 },
        label_position: Point { x: x + domain_width / 2.0, y: y + domain_height / 2.0 - if diagram.config.show_domain_descriptions { 30.0 } else { 0.0 } },
        model_position: diagram.config.show_domain_descriptions.then(|| Point { x: x + domain_width / 2.0, y: y + domain_height / 2.0 - 10.0 }),
        practice_position: diagram.config.show_domain_descriptions.then(|| Point { x: x + domain_width / 2.0, y: y + domain_height / 2.0 + 5.0 }),
        confusion: false }).collect();
    let confusion_items = diagram.domains.iter().find(|domain| domain.name == "confusion").map_or_else(Vec::new, |domain| domain.items.clone());
    let confusion_overflow = confusion_items.len().saturating_sub(3);
    domains.push(LayoutedCynefinDomain { name: "confusion".into(),
        items: confusion_items.into_iter().take(3).collect(), item_badges: Vec::new(), overflow_count: confusion_overflow,
        x: left + diagram.config.width * 0.35, y: top + diagram.config.height * 0.35,
        width: diagram.config.width * 0.3, height: diagram.config.height * 0.3,
        center: Point { x: left + diagram.config.width / 2.0, y: top + diagram.config.height / 2.0 },
        label_position: Point { x: left + diagram.config.width / 2.0, y: top + diagram.config.height / 2.0
            - if diagram.config.show_domain_descriptions { 10.0 } else { 0.0 } },
        model_position: None,
        practice_position: diagram.config.show_domain_descriptions.then(|| Point { x: left + diagram.config.width / 2.0,
            y: top + diagram.config.height / 2.0 + 8.0 }), confusion: true });
    for domain in &mut domains { domain.item_badges = cynefin_item_badges(domain, diagram.config.show_domain_descriptions); }
    let domain_by_name: HashMap<_, _> = domains.iter().map(|domain| (domain.name.as_str(), domain)).collect();
    let transitions = diagram.transitions.iter().filter_map(|transition| {
        let from = domain_by_name.get(transition.from.as_str())?;
        let to = domain_by_name.get(transition.to.as_str())?;
        let dx = to.center.x - from.center.x;
        let dy = to.center.y - from.center.y;
        let length = dx.hypot(dy);
        let middle = Point { x: (from.center.x + to.center.x) / 2.0, y: (from.center.y + to.center.y) / 2.0 };
        let control = Point { x: middle.x - dy / length * length * 0.15,
            y: middle.y + dx / length * length * 0.15 };
        let tangent_x = to.center.x - control.x;
        let tangent_y = to.center.y - control.y;
        let tangent_length = tangent_x.hypot(tangent_y);
        let unit_x = tangent_x / tangent_length;
        let unit_y = tangent_y / tangent_length;
        let normal_x = -unit_y;
        let normal_y = unit_x;
        let arrow_tip = Point { x: to.center.x + unit_x * 0.6, y: to.center.y + unit_y * 0.6 };
        let arrow_base = Point { x: to.center.x - unit_x * 5.4, y: to.center.y - unit_y * 5.4 };
        let arrowhead = [arrow_tip,
            Point { x: arrow_base.x + normal_x * 3.0, y: arrow_base.y + normal_y * 3.0 },
            Point { x: arrow_base.x - normal_x * 3.0, y: arrow_base.y - normal_y * 3.0 }];
        let label_position = transition.label.as_ref().map(|_| Point { x: control.x, y: control.y - 6.0 });
        Some(LayoutedCynefinTransition { from: from.center.clone(), control, to: to.center.clone(),
            label: transition.label.clone(), label_position, arrowhead })
    }).collect();
    let boundaries = cynefin_boundaries(&diagram.config, left, top);
    let cliff = cynefin_cliff(&diagram.config, left, top);
    LayoutedCynefinDiagram { width, height, title: diagram.title.clone(),
        title_position: Point { x: left + diagram.config.width / 2.0, y: top / 2.0 },
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        show_domain_descriptions: diagram.config.show_domain_descriptions, style: diagram.config.style.clone(),
        domains, boundaries, cliff, transitions }
}

fn cynefin_item_badges(domain: &LayoutedCynefinDomain, show_descriptions: bool) -> Vec<LayoutedCynefinItem> {
    const HEIGHT: f64 = 26.0;
    let start_y = if domain.confusion { domain.center.y + if show_descriptions { 22.0 } else { 14.0 } }
        else { domain.center.y + if show_descriptions { 25.0 } else { 15.0 } };
    let mut badges: Vec<_> = domain.items.iter().enumerate().map(|(index, label)| {
        let width = label.encode_utf16().count() as f64 * 7.0 + 20.0;
        LayoutedCynefinItem { label: label.clone(), x: domain.center.x - width / 2.0,
            y: start_y + index as f64 * 30.0, width, height: HEIGHT, overflow: false }
    }).collect();
    if domain.overflow_count > 0 {
        let label = format!("+{} more", domain.overflow_count);
        let width = label.encode_utf16().count() as f64 * 7.0 + 20.0;
        badges.push(LayoutedCynefinItem { label, x: domain.center.x - width / 2.0,
            y: start_y + domain.items.len() as f64 * 30.0, width, height: HEIGHT, overflow: true });
    }
    badges
}

fn cynefin_boundaries(config: &diagram_ir::CynefinConfig, left: f64, top: f64) -> Vec<LayoutedCynefinBoundary> {
    let vertical_points: Vec<_> = (0..=7).map(|index| Point { x: left + config.width / 2.0
        + (cynefin_seeded_random(config.seed.wrapping_add(index * 17)) * 2.0 - 1.0) * config.boundary_amplitude,
        y: top + index as f64 * config.height / 7.0 }).collect();
    let horizontal_points: Vec<_> = (0..=7).map(|index| Point { x: left + index as f64 * config.width / 7.0,
        y: top + config.height / 2.0
            + (cynefin_seeded_random(config.seed.wrapping_add(100).wrapping_add(index * 23)) * 2.0 - 1.0) * config.boundary_amplitude }).collect();
    vec![cynefin_curve(&vertical_points, config.seed, config.boundary_amplitude, true),
        cynefin_curve(&horizontal_points, config.seed.wrapping_add(100), config.boundary_amplitude, false)]
}

fn cynefin_curve(points: &[Point], seed: i32, amplitude: f64, vertical: bool) -> LayoutedCynefinBoundary {
    let segments = points.windows(2).enumerate().map(|(index, pair)| {
        let first = &pair[0]; let second = &pair[1]; let direction = if index % 2 == 0 { 1.0 } else { -1.0 };
        let offset = amplitude * 1.5 * direction * cynefin_seeded_random(seed
            .wrapping_add(index as i32 * if vertical { 31 } else { 37 })
            .wrapping_add(if vertical { 7 } else { 11 }));
        if vertical { let middle = (first.y + second.y) / 2.0; CubicCurveSegment {
            control1: Point { x: first.x + offset, y: middle }, control2: Point { x: second.x - offset, y: middle }, end: second.clone() }
        } else { let middle = (first.x + second.x) / 2.0; CubicCurveSegment {
            control1: Point { x: middle, y: first.y + offset }, control2: Point { x: middle, y: second.y - offset }, end: second.clone() } }
    }).collect();
    LayoutedCynefinBoundary { start: points[0].clone(), segments }
}

fn cynefin_seeded_random(seed: i32) -> f64 {
    let mut value = seed.wrapping_add(1_831_565_813);
    value = (value ^ ((value as u32 >> 15) as i32)).wrapping_mul(value | 1);
    value ^= value.wrapping_add((value ^ ((value as u32 >> 7) as i32)).wrapping_mul(value | 61));
    f64::from((value ^ ((value as u32 >> 14) as i32)) as u32) / 4_294_967_296.0
}

fn cynefin_cliff(config: &diagram_ir::CynefinConfig, left: f64, top: f64) -> LayoutedCynefinBoundary {
    let center_x = left + config.width / 2.0;
    let top_y = top + config.height / 2.0;
    let bottom_y = top + config.height;
    let height = bottom_y - top_y;
    let amplitude = config.width * 0.03;
    LayoutedCynefinBoundary { start: Point { x: center_x, y: top_y }, segments: vec![
        CubicCurveSegment { control1: Point { x: center_x + amplitude, y: top_y + height * 0.2 },
            control2: Point { x: center_x - amplitude * 1.5, y: top_y + height * 0.55 },
            end: Point { x: center_x + amplitude * 0.5, y: top_y + height * 0.75 } },
        CubicCurveSegment { control1: Point { x: center_x - amplitude, y: top_y + height * 0.85 },
            control2: Point { x: center_x + amplitude * 0.3, y: top_y + height * 0.95 },
            end: Point { x: center_x, y: bottom_y } },
    ] }
}

/// Resolve canvas size and produce a `LayoutedGeometricDiagram`.
pub fn layout_geometric_diagram(d: &GeometricDiagram) -> LayoutedGeometricDiagram {
    let (w, h) = match (d.width, d.height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None)    => { let (_, _, _, my) = bbox(&d.elements); (w, my + MARGIN) }
        (None,    Some(h)) => { let (_, _, mx, _) = bbox(&d.elements); (mx + MARGIN, h) }
        (None,    None)    => {
            let (_, _, mx, my) = bbox(&d.elements);
            ((mx + MARGIN).max(100.0), (my + MARGIN).max(100.0))
        }
    };
    LayoutedGeometricDiagram { width: w, height: h, elements: d.elements.clone() }
}

// ── Bounding-box helpers ──────────────────────────────────────────────────

/// Returns `(min_x, min_y, max_x, max_y)` across all elements.
fn bbox(els: &[GeoElement]) -> (f64, f64, f64, f64) {
    if els.is_empty() { return (0.0, 0.0, 200.0, 100.0); }
    let mut mn_x = f64::INFINITY;
    let mut mn_y = f64::INFINITY;
    let mut mx_x = f64::NEG_INFINITY;
    let mut mx_y = f64::NEG_INFINITY;
    for e in els {
        let (x0, y0, x1, y1) = aabb(e);
        mn_x = mn_x.min(x0);
        mn_y = mn_y.min(y0);
        mx_x = mx_x.max(x1);
        mx_y = mx_y.max(y1);
    }
    (mn_x, mn_y, mx_x, mx_y)
}

/// Axis-aligned bounding box for a single element.
fn aabb(e: &GeoElement) -> (f64, f64, f64, f64) {
    match e {
        GeoElement::Box { x, y, w, h, .. } => (*x, *y, x + w, y + h),
        GeoElement::Circle { cx, cy, r, .. } => (cx - r, cy - r, cx + r, cy + r),
        GeoElement::Line { x1, y1, x2, y2, .. } => (x1.min(*x2), y1.min(*y2), x1.max(*x2), y1.max(*y2)),
        GeoElement::Arc { cx, cy, r, .. } => (cx - r, cy - r, cx + r, cy + r),
        GeoElement::Text { x, y, text, .. } => {
            // Rough estimate: 7.5 px per character wide, 16 px tall.
            let w = text.len() as f64 * 7.5;
            (*x, *y - 14.0, x + w, y + 4.0)
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::*;

    fn box_diagram() -> GeometricDiagram {
        GeometricDiagram {
            title: None,
            width: None, height: None,
            elements: vec![
                GeoElement::Box {
                    id: "b1".into(), x: 50.0, y: 50.0, w: 120.0, h: 60.0,
                    corner_radius: 4.0, label: Some("Input".into()),
                    fill: None, stroke: None,
                },
                GeoElement::Circle {
                    id: "c1".into(), cx: 300.0, cy: 80.0, r: 40.0,
                    label: None, fill: None, stroke: None,
                },
                GeoElement::Line {
                    id: "l1".into(), x1: 170.0, y1: 80.0, x2: 260.0, y2: 80.0,
                    arrow_end: true, arrow_start: false, stroke: None,
                },
            ],
        }
    }

    #[test] fn version_exists() { assert_eq!(crate::VERSION, "0.4.0"); }

    #[test]
    fn auto_size_includes_all_elements() {
        let d = layout_geometric_diagram(&box_diagram());
        // Circle extends to x=340, so canvas must be wider.
        assert!(d.width >= 340.0 + MARGIN, "width={} must be >= {}", d.width, 340.0 + MARGIN);
        // Canvas must be at least tall enough for circle bottom (80+40=120) + margin.
        assert!(d.height >= 120.0 + MARGIN);
    }

    #[test]
    fn explicit_size_respected() {
        let mut dg = box_diagram();
        dg.width  = Some(800.0);
        dg.height = Some(600.0);
        let d = layout_geometric_diagram(&dg);
        assert_eq!(d.width,  800.0);
        assert_eq!(d.height, 600.0);
    }

    #[test]
    fn elements_pass_through() {
        let dg = box_diagram();
        let n = dg.elements.len();
        let d = layout_geometric_diagram(&dg);
        assert_eq!(d.elements.len(), n);
    }

    #[test]
    fn empty_gets_min_canvas() {
        let dg = GeometricDiagram { title: None, width: None, height: None, elements: vec![] };
        let d = layout_geometric_diagram(&dg);
        assert!(d.width  >= 100.0);
        assert!(d.height >= 100.0);
    }

    #[test]
    fn text_aabb_estimated() {
        let e = GeoElement::Text {
            id: "t1".into(), x: 10.0, y: 50.0, text: "hello".into(),
            align: TextAlign::Left,
        };
        let (x0, _y0, x1, _y1) = aabb(&e);
        assert!(x1 > x0, "text must have positive width");
    }

    #[test]
    fn partial_size_fills_missing_axis() {
        let mut dg = box_diagram();
        dg.width = Some(900.0);
        let d = layout_geometric_diagram(&dg);
        assert_eq!(d.width, 900.0);
        assert!(d.height > 0.0);
    }

    #[test]
    fn venn_layout_is_deterministic_and_size_aware() {
        let diagram = VennDiagram { title: None,
            config: diagram_ir::VennConfig { width: 600.0, height: 360.0, padding: 20.0, ..Default::default() }, regions: vec![
            VennRegion { sets: vec!["A".into()], size: 20.0, label: None, style: VennStyle::default() },
            VennRegion { sets: vec!["B".into()], size: 10.0, label: None, style: VennStyle::default() },
            VennRegion { sets: vec!["A".into(), "B".into()], size: 3.0, label: Some("AB".into()), style: VennStyle::default() },
        ], texts: vec![] };
        let first = layout_venn(&diagram);
        assert_eq!(first, layout_venn(&diagram));
        assert!(first.circles[0].radius > first.circles[1].radius);
        assert!(first.labels.iter().any(|label| label.text == "AB"));
        assert!(first.labels.iter().all(|label| label.font_size == 18.0));
        assert_eq!((first.width, first.height, first.config.padding), (600.0, 360.0, 20.0));
        assert_eq!(first.config.theme_colors, diagram.config.theme_colors);
        assert!(first.circles.iter().all(|circle| circle.cx - circle.radius >= 20.0
            && circle.cx + circle.radius <= 580.0 && circle.cy - circle.radius >= 20.0
            && circle.cy + circle.radius <= 340.0));
    }

    #[test]
    fn venn_debug_layout_resolves_text_area_circles_and_grid_cells() {
        let diagram = VennDiagram { title: None,
            config: diagram_ir::VennConfig { width: 800.0, height: 450.0, use_debug_layout: true, ..Default::default() },
            regions: vec![
                VennRegion { sets: vec!["A".into()], size: 20.0, label: Some("Alpha".into()), style: VennStyle::default() },
                VennRegion { sets: vec!["B".into()], size: 18.0, label: None, style: VennStyle::default() },
            ],
            texts: vec![
                diagram_ir::VennText { sets: vec!["A".into()], id: "one".into(), label: None, style: Default::default() },
                diagram_ir::VennText { sets: vec!["A".into()], id: "two".into(), label: None, style: Default::default() },
            ] };
        let layout = layout_venn(&diagram);
        assert_eq!(layout.debug_circles.len(), 1);
        assert_eq!(layout.debug_cells.len(), 2);
        assert!(layout.debug_circles[0].radius > 0.0);
        assert_eq!(layout.debug_cells[0].width, layout.debug_cells[1].width);
        assert!(layout.debug_cells[0].x < layout.debug_cells[1].x);
        let one = layout.labels.iter().find(|label| label.text == "one").expect("one label");
        let two = layout.labels.iter().find(|label| label.text == "two").expect("two label");
        assert!(one.x < two.x, "attached text must use the resolved grid cells");
    }

    #[test]
    fn ishikawa_layout_alternates_major_causes() {
        let diagram = IshikawaDiagram { effect: "Problem".into(), causes: vec![
            IshikawaCause { id: "a".into(), label: "A".into(), parent_id: None, depth: 1 },
            IshikawaCause { id: "b".into(), label: "B".into(), parent_id: None, depth: 1 },
        ], config: Default::default() };
        let layout = layout_ishikawa(&diagram, 640.0);
        assert!(layout.bones[0].from.y < layout.spine_from.y);
        assert!(layout.bones[1].from.y > layout.spine_from.y);
        assert_eq!(layout.bones[0].label_height, 28.0);
        assert_eq!(layout.bones[0].label_width, 72.0);
        assert_eq!(layout.spine_path.len(), 2);
        assert_eq!(layout.bones[0].path.len(), 2);
        assert_eq!(layout.bones[0].arrowhead.len(), 3);
        assert_eq!(layout.bones[0].arrowhead[0], layout.bones[0].to);
        assert_eq!(layout.effect_outline.len(), 14);
        assert_eq!(layout.bones[0].label_outline.len(), 5);
        assert!(layout.effect_hatches.is_empty());
        assert!(layout.bones[0].hatches.is_empty());

        let mut hand_drawn = diagram.clone();
        hand_drawn.config.look = IshikawaLook::HandDrawn;
        hand_drawn.config.hand_drawn_seed = 17;
        let hand_drawn_layout = layout_ishikawa(&hand_drawn, 640.0);
        assert_eq!(hand_drawn_layout.spine_path.len(), 5);
        assert_eq!(hand_drawn_layout.bones[0].path.len(), 5);
        assert_eq!(hand_drawn_layout.bones[0].arrowhead[0], hand_drawn_layout.bones[0].to);
        assert_eq!(hand_drawn_layout.effect_outline.len(), 53);
        assert_eq!(hand_drawn_layout.bones[0].label_outline.len(), 17);
        assert!(!hand_drawn_layout.effect_hatches.is_empty());
        assert!(!hand_drawn_layout.bones[0].hatches.is_empty());
        assert_eq!(hand_drawn_layout, layout_ishikawa(&hand_drawn, 640.0));

        let mut large_text = diagram.clone();
        large_text.config.font_size = 24.0;
        let large_text_layout = layout_ishikawa(&large_text, 640.0);
        assert_eq!(large_text_layout.bones[0].label_height, 42.0);
        assert_eq!(large_text_layout.bones[0].label_width, 108.0);

        let mut padded = diagram.clone();
        padded.config.diagram_padding = 64.0;
        let padded_layout = layout_ishikawa(&padded, 640.0);
        assert_eq!(padded_layout.spine_from.x, 80.0);
        assert_eq!(padded_layout.width - (padded_layout.effect_x + padded_layout.effect_width), 64.0);
        assert!(
            (padded_layout.bones[0].from.y - padded_layout.spine_from.y).abs()
                < (layout.bones[0].from.y - layout.spine_from.y).abs()
        );

        let mut deep = diagram;
        deep.causes.push(IshikawaCause { id: "c".into(), label: "A deliberately long nested cause label".into(),
            parent_id: Some("a".into()), depth: 3 });
        let deep_layout = layout_ishikawa(&deep, 560.0);
        assert!(deep_layout.height >= 470.0);
        assert!(deep_layout.bones[2].label_height > 28.0);
        for (index, left) in deep_layout.bones.iter().enumerate() {
            for right in deep_layout.bones.iter().skip(index + 1) {
                let horizontal_overlap = (left.label_position.x - right.label_position.x).abs()
                    < (left.label_width + right.label_width) / 2.0;
                let vertical_overlap = (left.label_position.y - right.label_position.y).abs()
                    < (left.label_height + right.label_height) / 2.0;
                assert!(!(horizontal_overlap && vertical_overlap));
            }
        }
        assert!(deep_layout.bones.iter().all(|bone| bone.label_outline.iter().all(|point|
            point.x >= 0.0 && point.x <= deep_layout.width && point.y >= 0.0 && point.y <= deep_layout.height)));
    }


    #[test]
    fn wardley_layout_maps_visibility_up_and_evolution_right() {
        let diagram = WardleyDiagram { title: None, accessibility_title: Some("Strategy map".into()),
            accessibility_description: Some("Components and dependencies".into()), config: diagram_ir::WardleyConfig { width: 600.0, height: 400.0,
            padding: 40.0, theme: diagram_ir::WardleyTheme { annotation_stroke: "#7c3aed".into(),
                annotation_text_color: "#f97316".into(), annotation_fill: "#fef3c7".into(), ..Default::default() },
            ..Default::default() }, stages: vec!["Genesis".into(), "Commodity".into()], stage_boundaries: vec![0.35, 1.0],
            nodes: vec![diagram_ir::WardleyNode { id: "a".into(), label: "A".into(), visibility: 0.8, evolution: 0.2, anchor: false, pipeline_parent: true, label_offset_x: Some(-20.0), label_offset_y: Some(8.0), source_strategy: Some(diagram_ir::WardleySourceStrategy::Build), inertia: true },
                diagram_ir::WardleyNode { id: "b".into(), label: "B".into(), visibility: 0.2, evolution: 0.8, anchor: false, pipeline_parent: false, label_offset_x: None, label_offset_y: None, source_strategy: None, inertia: false },
                diagram_ir::WardleyNode { id: "c".into(), label: "A1".into(), visibility: 0.8, evolution: 0.1, anchor: false, pipeline_parent: false, label_offset_x: None, label_offset_y: None, source_strategy: None, inertia: false },
                diagram_ir::WardleyNode { id: "d".into(), label: "A2".into(), visibility: 0.8, evolution: 0.4, anchor: false, pipeline_parent: false, label_offset_x: None, label_offset_y: None, source_strategy: None, inertia: false }],
            links: vec![diagram_ir::WardleyLink { source: "A".into(), target: "B".into(), dashed: true,
                label: Some("flow".into()), flow: Some(diagram_ir::WardleyLinkFlow::Bidirectional) }],
            evolves: vec![], notes: vec![diagram_ir::WardleyNote { text: "Watch".into(), visibility: 0.5, evolution: 0.5 }],
            annotations_box: Some((0.2, 0.8)), annotations: vec![diagram_ir::WardleyAnnotation { number: 1,
                text: "Risk".into(), coordinates: vec![(0.6, 0.7)] }], forces: vec![diagram_ir::WardleyForce {
                    label: "Adoption".into(), visibility: 0.4, evolution: 0.25, kind: diagram_ir::WardleyForceKind::Accelerator }],
            pipelines: vec![diagram_ir::WardleyPipeline { parent_id: "a".into(), component_ids: vec!["c".into(), "d".into()] }] };
        let layout = layout_wardley(&diagram);
        assert!(layout.nodes[0].position.y < layout.nodes[1].position.y);
        assert!(layout.nodes[0].position.x < layout.nodes[1].position.x);
        assert_eq!(layout.nodes[0].position.x, 170.0);
        assert_eq!(layout.notes[0].position, Point { x: 300.0, y: 200.0 });
        assert_eq!(layout.annotations[0].positions[0], Point { x: 404.0, y: 168.0 });
        assert_eq!(layout.forces[0].position, Point { x: 170.0, y: 232.0 });
        assert_eq!(layout.forces[0].kind, diagram_ir::WardleyForceKind::Accelerator);
        assert_eq!((layout.pipelines[0].x, layout.pipelines[0].width), (77.0, 186.0));
        assert_eq!(layout.pipelines[0].component_positions.len(), 2);
        assert_eq!(layout.stage_boundaries, [0.35, 1.0]);
        assert_eq!((layout.nodes[0].label_offset_x, layout.nodes[0].label_offset_y), (Some(-20.0), Some(8.0)));
        assert_eq!(layout.nodes[0].source_strategy, Some(diagram_ir::WardleySourceStrategy::Build));
        assert!(layout.nodes[0].inertia);
        assert_eq!(layout.accessibility_title.as_deref(), Some("Strategy map"));
        assert_eq!(layout.accessibility_description.as_deref(), Some("Components and dependencies"));
        assert_eq!((&layout.config.theme.annotation_stroke, &layout.config.theme.annotation_text_color,
            &layout.config.theme.annotation_fill), (&"#7c3aed".to_string(), &"#f97316".to_string(), &"#fef3c7".to_string()));
        assert!(layout.links[0].dashed);
        assert_eq!(layout.links[0].label.as_deref(), Some("flow"));
        assert_eq!(layout.links[0].flow, Some(diagram_ir::WardleyLinkFlow::Bidirectional));
        assert!(layout.links[0].from.x > layout.nodes[0].position.x);
        assert!(layout.links[0].to.x < layout.nodes[1].position.x);
    }


    #[test]
    fn cynefin_layout_places_domains_in_fixed_semantic_quadrants() {
        let layout = layout_cynefin(&CynefinDiagram { title: None, accessibility_title: None, accessibility_description: None,
            config: diagram_ir::CynefinConfig { width: 640.0, height: 420.0, padding: 24.0, show_domain_descriptions: false,
                boundary_amplitude: 12.0, seed: 17, style: diagram_ir::CynefinStyle::default() },
            domains: vec![diagram_ir::CynefinDomain { name: "confusion".into(),
                items: vec!["One".into(), "Two".into(), "Three".into(), "Four".into(), "Five".into()] }],
            transitions: vec![diagram_ir::CynefinTransition { from: "complex".into(), to: "clear".into(), label: Some("Shift".into()) }] });
        let complex = layout.domains.iter().find(|domain| domain.name == "complex").unwrap();
        let clear = layout.domains.iter().find(|domain| domain.name == "clear").unwrap();
        assert!(complex.center.x < clear.center.x && complex.center.y < clear.center.y);
        assert!(layout.domains.iter().find(|domain| domain.name == "confusion").unwrap().confusion);
        assert_eq!(layout.domains.iter().find(|domain| domain.name == "confusion").unwrap().overflow_count, 2);
        assert_eq!(layout.domains.iter().find(|domain| domain.name == "confusion").unwrap().item_badges.len(), 4);
        assert_eq!(complex.label_position, complex.center);
        assert_eq!(complex.model_position, None);
        assert_eq!(complex.practice_position, None);
        assert_eq!(layout.title_position, Point { x: 344.0, y: 12.0 });
        assert_eq!((layout.width, layout.height), (688.0, 468.0));
        assert!(!layout.show_domain_descriptions);
        assert_eq!(layout.boundaries.len(), 2);
        assert!(layout.boundaries.iter().all(|boundary| boundary.segments.len() == 7));
        assert_eq!(layout.cliff.start, Point { x: 344.0, y: 234.0 });
        assert_eq!(layout.cliff.segments.len(), 2);
        assert_eq!(layout.cliff.segments[1].end, Point { x: 344.0, y: 444.0 });
        assert_eq!(layout.transitions[0].from, complex.center);
        assert_eq!(layout.transitions[0].to, clear.center);
        assert_ne!(layout.transitions[0].control, Point { x: 344.0, y: 234.0 });
        assert_eq!(layout.transitions[0].label_position, Some(Point {
            x: layout.transitions[0].control.x, y: layout.transitions[0].control.y - 6.0 }));
        let arrow_tip = &layout.transitions[0].arrowhead[0];
        assert!(((arrow_tip.x - clear.center.x).hypot(arrow_tip.y - clear.center.y) - 0.6).abs() < 1e-9);
        for arrow_base in &layout.transitions[0].arrowhead[1..] {
            assert!(((arrow_base.x - clear.center.x).hypot(arrow_base.y - clear.center.y) - 6.177_378_084_592_2).abs() < 1e-9);
        }
    }

    #[test]
    fn cynefin_layout_matches_upstream_description_text_anchors() {
        let layout = layout_cynefin(&CynefinDiagram { title: Some("Framework".into()), accessibility_title: None,
            accessibility_description: None, config: diagram_ir::CynefinConfig { width: 800.0, height: 600.0,
                padding: 40.0, show_domain_descriptions: true, boundary_amplitude: 8.0, seed: 0,
                style: diagram_ir::CynefinStyle::default() }, domains: vec![], transitions: vec![] });
        let complex = layout.domains.iter().find(|domain| domain.name == "complex").unwrap();
        let confusion = layout.domains.iter().find(|domain| domain.name == "confusion").unwrap();
        assert_eq!(layout.title_position, Point { x: 440.0, y: 20.0 });
        assert_eq!(complex.label_position, Point { x: 240.0, y: 160.0 });
        assert_eq!(complex.model_position, Some(Point { x: 240.0, y: 180.0 }));
        assert_eq!(complex.practice_position, Some(Point { x: 240.0, y: 195.0 }));
        assert_eq!(confusion.label_position, Point { x: 440.0, y: 330.0 });
        assert_eq!(confusion.model_position, None);
        assert_eq!(confusion.practice_position, Some(Point { x: 440.0, y: 348.0 }));
    }

    #[test]
    fn info_layout_matches_upstream_fixed_canvas_and_label() {
        let layout = layout_info(&InfoDiagram { version: "11.16.1".into() });
        assert_eq!((layout.width, layout.height), (400.0, 100.0));
        assert_eq!(layout.label, "v11.16.1");
        assert_eq!(layout.font_size, 32.0);
    }
}
