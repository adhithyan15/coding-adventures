//! # diagram-layout-geometric
//!
//! Layout engine for geometric diagrams (DG04).
//!
//! Geometric diagrams use absolute user coordinates — the layout engine only
//! resolves the canvas size (from an explicit `width`/`height` or from the
//! bounding box of all elements) and passes elements through unchanged.

use std::collections::HashMap;
use diagram_ir::{GeoElement, GeometricDiagram, IshikawaDiagram, LayoutedGeometricDiagram, LayoutedIshikawaBone, LayoutedIshikawaDiagram, LayoutedVennCircle, LayoutedVennDiagram, LayoutedVennLabel, LayoutedWardleyDiagram, LayoutedWardleyEvolution, LayoutedWardleyLink, LayoutedWardleyNode, Point, VennDiagram, WardleyDiagram};

pub const VERSION: &str = "0.1.0";

const MARGIN: f64 = 20.0;

/// Produce stable Venn geometry. Sizes affect radii; exact area-proportional
/// overlap optimization is deliberately outside this partial slice.
pub fn layout_venn(diagram: &VennDiagram, canvas_width: f64) -> LayoutedVennDiagram {
    let width = canvas_width.max(360.0);
    let height = (width * 0.68).max(300.0);
    let title_offset = if diagram.title.is_some() { 34.0 } else { 0.0 };
    let sets: Vec<_> = diagram.regions.iter().filter(|region| region.sets.len() == 1).collect();
    let max_size = sets.iter().map(|set| set.size).fold(1.0_f64, f64::max);
    let orbit = (width.min(height - title_offset) * 0.16).max(36.0);
    let base_radius = (width.min(height - title_offset) * 0.25).max(64.0);
    let center_x = width / 2.0;
    let center_y = title_offset + (height - title_offset) / 2.0;
    let count = sets.len().max(1) as f64;
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
            y: circle.cy + angle.sin() * circle.radius * 0.58, style: circle.style.clone() });
    }
    for region in diagram.regions.iter().filter(|region| region.sets.len() > 1) {
        if let (Some(label), Some((x, y))) = (&region.label, centroid(&region.sets, &centers)) {
            labels.push(LayoutedVennLabel { text: label.clone(), x, y, style: region.style.clone() });
        }
    }
    for text in &diagram.texts {
        if let Some((x, y)) = centroid(&text.sets, &centers) {
            labels.push(LayoutedVennLabel { text: text.label.clone().unwrap_or_else(|| text.id.clone()), x, y: y + 20.0, style: text.style.clone() });
        }
    }
    LayoutedVennDiagram { width, height, title: diagram.title.clone(), circles, labels }
}

fn centroid(sets: &[String], centers: &HashMap<&str, (f64, f64)>) -> Option<(f64, f64)> {
    let points: Vec<_> = sets.iter().filter_map(|set| centers.get(set.as_str())).collect();
    (!points.is_empty()).then(|| { let count = points.len() as f64;
        (points.iter().map(|point| point.0).sum::<f64>() / count,
         points.iter().map(|point| point.1).sum::<f64>() / count) })
}

/// Lay out an Ishikawa causal tree as an alternating fishbone.
pub fn layout_ishikawa(diagram: &IshikawaDiagram, canvas_width: f64) -> LayoutedIshikawaDiagram {
    let width = canvas_width.max(560.0); let height = (width * 0.58).max(360.0);
    let spine_y = height / 2.0; let spine_from = Point { x: 36.0, y: spine_y };
    let effect_width = 150.0; let effect_height = 58.0; let effect_x = width - effect_width - 20.0;
    let spine_to = Point { x: effect_x, y: spine_y };
    let roots: Vec<_> = diagram.causes.iter().filter(|cause| cause.parent_id.is_none()).collect();
    let spacing = (spine_to.x - spine_from.x) / (roots.len() + 1).max(2) as f64;
    let mut bones = Vec::new(); let mut anchors = HashMap::<&str, Point>::new();
    for (index, cause) in roots.iter().enumerate() {
        let attach = Point { x: spine_from.x + spacing * (index + 1) as f64, y: spine_y };
        let sign = if index.is_multiple_of(2) { -1.0 } else { 1.0 };
        let tip = Point { x: attach.x - spacing * 0.48, y: spine_y + sign * height * 0.32 };
        bones.push(LayoutedIshikawaBone { from: tip.clone(), to: attach, label: cause.label.clone(),
            label_position: Point { x: tip.x + spacing * 0.24, y: (tip.y + spine_y) / 2.0 + sign * 42.0 }, depth: 1 });
        anchors.insert(&cause.id, tip);
    }
    for cause in diagram.causes.iter().filter(|cause| cause.parent_id.is_some()) {
        let parent = cause.parent_id.as_deref().and_then(|id| anchors.get(id)).cloned().unwrap_or(spine_from.clone());
        let sign = if parent.y < spine_y { -1.0 } else { 1.0 };
        let length = (90.0 / cause.depth as f64).max(38.0);
        let tip = Point { x: parent.x - length, y: parent.y + sign * length * 0.55 };
        bones.push(LayoutedIshikawaBone { from: tip.clone(), to: parent,
            label: cause.label.clone(), label_position: Point { x: tip.x, y: tip.y - sign * 12.0 }, depth: cause.depth });
        anchors.insert(&cause.id, tip);
    }
    LayoutedIshikawaDiagram { width, height, effect: diagram.effect.clone(), effect_x, effect_y: spine_y - effect_height / 2.0,
        effect_width, effect_height, spine_from, spine_to, bones }
}

/// Map Wardley visibility/evolution coordinates onto a deterministic Cartesian canvas.
pub fn layout_wardley(diagram: &WardleyDiagram) -> LayoutedWardleyDiagram {
    let width = diagram.width.max(420.0); let height = diagram.height.max(300.0);
    let left = 62.0; let right = 28.0; let top = if diagram.title.is_some() { 48.0 } else { 24.0 }; let bottom = 52.0;
    let point = |visibility: f64, evolution: f64| Point {
        x: left + evolution * (width - left - right), y: top + (1.0 - visibility) * (height - top - bottom),
    };
    let nodes: Vec<_> = diagram.nodes.iter().map(|node| LayoutedWardleyNode { id: node.id.clone(), label: node.label.clone(),
        position: point(node.visibility, node.evolution), anchor: node.anchor }).collect();
    let positions: HashMap<_, _> = diagram.nodes.iter().zip(nodes.iter()).map(|(source, layout)| (source.label.as_str(), layout.position.clone())).collect();
    let links = diagram.links.iter().filter_map(|link| Some(LayoutedWardleyLink {
        from: positions.get(link.source.as_str())?.clone(), to: positions.get(link.target.as_str())?.clone(),
    })).collect();
    let evolves = diagram.evolves.iter().filter_map(|evolve| {
        let node = diagram.nodes.iter().find(|node| node.label == evolve.component)?;
        Some(LayoutedWardleyEvolution { from: positions.get(node.label.as_str())?.clone(), to: point(node.visibility, evolve.target) })
    }).collect();
    LayoutedWardleyDiagram { width, height, title: diagram.title.clone(), stages: diagram.stages.clone(), nodes, links, evolves }
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

    #[test] fn version_exists() { assert_eq!(crate::VERSION, "0.1.0"); }

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
        let diagram = VennDiagram { title: None, regions: vec![
            VennRegion { sets: vec!["A".into()], size: 20.0, label: None, style: VennStyle::default() },
            VennRegion { sets: vec!["B".into()], size: 10.0, label: None, style: VennStyle::default() },
            VennRegion { sets: vec!["A".into(), "B".into()], size: 3.0, label: Some("AB".into()), style: VennStyle::default() },
        ], texts: vec![] };
        let first = layout_venn(&diagram, 600.0);
        assert_eq!(first, layout_venn(&diagram, 600.0));
        assert!(first.circles[0].radius > first.circles[1].radius);
        assert!(first.labels.iter().any(|label| label.text == "AB"));
    }

    #[test]
    fn ishikawa_layout_alternates_major_causes() {
        let diagram = IshikawaDiagram { effect: "Problem".into(), causes: vec![
            IshikawaCause { id: "a".into(), label: "A".into(), parent_id: None, depth: 1 },
            IshikawaCause { id: "b".into(), label: "B".into(), parent_id: None, depth: 1 },
        ] };
        let layout = layout_ishikawa(&diagram, 640.0);
        assert!(layout.bones[0].from.y < layout.spine_from.y);
        assert!(layout.bones[1].from.y > layout.spine_from.y);
    }


    #[test]
    fn wardley_layout_maps_visibility_up_and_evolution_right() {
        let diagram = WardleyDiagram { title: None, width: 600.0, height: 400.0, stages: vec!["Genesis".into(), "Commodity".into()],
            nodes: vec![diagram_ir::WardleyNode { id: "a".into(), label: "A".into(), visibility: 0.8, evolution: 0.2, anchor: false },
                diagram_ir::WardleyNode { id: "b".into(), label: "B".into(), visibility: 0.2, evolution: 0.8, anchor: false }],
            links: vec![], evolves: vec![] };
        let layout = layout_wardley(&diagram);
        assert!(layout.nodes[0].position.y < layout.nodes[1].position.y);
        assert!(layout.nodes[0].position.x < layout.nodes[1].position.x);
    }
}
