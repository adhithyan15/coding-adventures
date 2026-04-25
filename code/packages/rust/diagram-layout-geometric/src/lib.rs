//! # diagram-layout-geometric
//!
//! Layout engine for geometric diagrams (DG04).
//!
//! Geometric diagrams use absolute user coordinates — the layout engine only
//! resolves the canvas size (from an explicit `width`/`height` or from the
//! bounding box of all elements) and passes elements through unchanged.

use diagram_ir::{GeoElement, GeometricDiagram, LayoutedGeometricDiagram};

pub const VERSION: &str = "0.1.0";

const MARGIN: f64 = 20.0;

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
}
