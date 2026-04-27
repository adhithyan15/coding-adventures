//! # diagram-layout-chart
//!
//! Layout engine for chart-family diagrams (DG04).
//!
//! Converts a `ChartDiagram` (semantic IR with no geometry) into a
//! `LayoutedChartDiagram` (geometry ready for `diagram-to-paint`).
//!
//! Supported chart kinds:
//!   * **XY** — bar and line series on categorical x-axis, numeric y-axis
//!   * **Pie** — angular slices starting at 12 o'clock
//!   * **Sankey** — left-to-right proportional bands

use diagram_ir::{
    ChartDiagram, ChartKind, ChartSeries, LegendEntry,
    LayoutedChartDiagram, LayoutedChartItem, Orientation, Point, SeriesKind,
};

pub const VERSION: &str = "0.1.0";

const MARGIN: f64 = 24.0;
const TITLE_H: f64 = 32.0;
const Y_LBL_W: f64 = 48.0;
const X_LBL_H: f64 = 24.0;
const LEGEND_H: f64 = 28.0;
const TICK_LEN: f64 = 6.0;
const GRID_COUNT: usize = 5;

const SERIES_COLORS: &[&str] = &[
    "#3b82f6", "#ef4444", "#22c55e", "#f59e0b", "#a855f7", "#14b8a6",
];

/// Lay out a `ChartDiagram` on a canvas of `cw × ch` pixels.
pub fn layout_chart_diagram(diagram: &ChartDiagram, cw: f64, ch: f64) -> LayoutedChartDiagram {
    match diagram.kind {
        ChartKind::Xy     => layout_xy(diagram, cw, ch),
        ChartKind::Pie    => layout_pie(diagram, cw, ch),
        ChartKind::Sankey => layout_sankey(diagram, cw, ch),
    }
}

// ── XY layout ────────────────────────────────────────────────────────────

fn resolve_y_range(diagram: &ChartDiagram) -> (f64, f64) {
    if let Some(ref ya) = diagram.y_axis {
        if ya.min < ya.max { return (ya.min, ya.max); }
    }
    let all: Vec<f64> = diagram.series.iter().flat_map(|s| s.data.iter().copied()).collect();
    if all.is_empty() { return (0.0, 100.0); }
    let mn = all.iter().cloned().fold(f64::INFINITY, f64::min).min(0.0);
    let mx = all.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    (mn, if mx == mn { mn + 1.0 } else { mx })
}

fn layout_xy(diagram: &ChartDiagram, cw: f64, ch: f64) -> LayoutedChartDiagram {
    let has_title  = diagram.title.is_some();
    let has_series = !diagram.series.is_empty();
    let lh = if has_series { LEGEND_H } else { 0.0 };

    // Plot area bounds
    let pt = MARGIN + if has_title { TITLE_H } else { 0.0 };  // top
    let pl = MARGIN + Y_LBL_W;                                 // left
    let pb = ch - MARGIN - X_LBL_H - lh;                      // bottom
    let pr = cw - MARGIN;                                       // right
    let pw = (pr - pl).max(1.0);
    let ph = (pb - pt).max(1.0);

    let (ym, yx) = resolve_y_range(diagram);
    let yr = (yx - ym).max(1.0);

    let cats: Vec<String> = diagram.x_axis.as_ref()
        .map(|a| a.categories.clone())
        .unwrap_or_default();
    let nc = cats.len().max(1);
    let nb = diagram.series.iter().filter(|s| s.kind == SeriesKind::Bar).count();
    let cat_w = pw / nc as f64;
    let bar_w = if nb > 0 { (cat_w * 0.7 / nb as f64).max(4.0) } else { cat_w * 0.7 };

    let mut items: Vec<LayoutedChartItem> = Vec::new();

    // Title
    if let Some(ref t) = diagram.title {
        items.push(LayoutedChartItem::DataLabel {
            x: cw / 2.0, y: MARGIN + TITLE_H * 0.5, text: t.clone(),
        });
    }

    // Y-axis grid lines + tick labels
    for i in 0..=GRID_COUNT {
        let frac = i as f64 / GRID_COUNT as f64;
        let val  = ym + frac * yr;
        let y    = pb - frac * ph;
        items.push(LayoutedChartItem::GridLine { x1: pl, y1: y, x2: pr, y2: y });
        items.push(LayoutedChartItem::AxisTick {
            x: pl - TICK_LEN - 4.0, y,
            label: format!("{val:.0}"),
            orientation: Orientation::Horizontal,
        });
    }

    // X-axis category labels
    for (i, cat) in cats.iter().enumerate() {
        let cx = pl + (i as f64 + 0.5) * cat_w;
        items.push(LayoutedChartItem::AxisTick {
            x: cx, y: pb + TICK_LEN + 4.0,
            label: cat.clone(),
            orientation: Orientation::Vertical,
        });
    }

    // Axis spines
    items.push(LayoutedChartItem::AxisSpine {
        x1: pl, y1: pb, x2: pr, y2: pb, orientation: Orientation::Horizontal,
    });
    items.push(LayoutedChartItem::AxisSpine {
        x1: pl, y1: pt, x2: pl, y2: pb, orientation: Orientation::Vertical,
    });

    // Series (bars + lines)
    let mut bar_series_idx = 0usize;
    let mut legend_entries: Vec<LegendEntry> = Vec::new();

    for (si, series) in diagram.series.iter().enumerate() {
        let color = SERIES_COLORS[si % SERIES_COLORS.len()].to_string();
        if let Some(ref lbl) = series.label {
            legend_entries.push(LegendEntry { color: color.clone(), label: lbl.clone() });
        }
        match series.kind {
            SeriesKind::Bar => {
                for (ci, &val) in series.data.iter().enumerate() {
                    let bh = ((val - ym) / yr * ph).max(0.0);
                    let bx = pl + ci as f64 * cat_w + cat_w * 0.15
                           + bar_series_idx as f64 * bar_w;
                    let by = pb - bh;
                    items.push(LayoutedChartItem::Bar {
                        x: bx, y: by, width: bar_w, height: bh, color: color.clone(),
                    });
                }
                bar_series_idx += 1;
            }
            SeriesKind::Line => {
                let pts: Vec<Point> = series.data.iter().enumerate().map(|(ci, &val)| {
                    let lx = pl + (ci as f64 + 0.5) * cat_w;
                    let ly = pb - (val - ym) / yr * ph;
                    Point { x: lx, y: ly }
                }).collect();
                if !pts.is_empty() {
                    items.push(LayoutedChartItem::LinePath { points: pts, color: color.clone() });
                }
            }
        }
    }

    // Legend
    if !legend_entries.is_empty() {
        items.push(LayoutedChartItem::Legend {
            x: pl, y: ch - lh / 2.0, entries: legend_entries,
        });
    }

    LayoutedChartDiagram { width: cw, height: ch, title_box: None, items }
}

// ── Pie layout ────────────────────────────────────────────────────────────

const PIE_COLORS: &[&str] = &[
    "#3b82f6", "#ef4444", "#22c55e", "#f59e0b", "#a855f7",
    "#14b8a6", "#f97316", "#8b5cf6",
];

fn layout_pie(diagram: &ChartDiagram, cw: f64, ch: f64) -> LayoutedChartDiagram {
    let cx = cw / 2.0;
    let cy = ch / 2.0;
    let r  = (cw.min(ch) / 2.0 - MARGIN * 2.0).max(10.0);
    let total: f64 = diagram.slices.iter().map(|s| s.value).sum();
    let total = if total == 0.0 { 1.0 } else { total };
    let mut angle = -std::f64::consts::FRAC_PI_2; // start at 12 o'clock
    let mut items: Vec<LayoutedChartItem> = Vec::new();

    if let Some(ref t) = diagram.title {
        items.push(LayoutedChartItem::DataLabel {
            x: cw / 2.0, y: MARGIN + TITLE_H * 0.5, text: t.clone(),
        });
    }

    for (i, slice) in diagram.slices.iter().enumerate() {
        let delta = slice.value / total * std::f64::consts::TAU;
        let end   = angle + delta;
        let color = PIE_COLORS[i % PIE_COLORS.len()].to_string();
        items.push(LayoutedChartItem::PieArc {
            cx, cy, r, start_angle: angle, end_angle: end,
            color, label: slice.label.clone(),
        });
        angle = end;
    }

    LayoutedChartDiagram { width: cw, height: ch, title_box: None, items }
}

// ── Sankey layout ─────────────────────────────────────────────────────────

fn layout_sankey(diagram: &ChartDiagram, cw: f64, ch: f64) -> LayoutedChartDiagram {
    let total: f64 = diagram.flows.iter().map(|f| f.weight).sum();
    let total = if total == 0.0 { 1.0 } else { total };
    let plot_h = ch - MARGIN * 2.0;
    let mut items: Vec<LayoutedChartItem> = Vec::new();
    let mut y_off = MARGIN;
    for (i, flow) in diagram.flows.iter().enumerate() {
        let band_h = (flow.weight / total * plot_h).max(2.0);
        let color  = SERIES_COLORS[i % SERIES_COLORS.len()].to_string();
        items.push(LayoutedChartItem::SankeyBand {
            from_x: MARGIN, from_y: y_off,
            to_x: cw - MARGIN, to_y: y_off,
            width: band_h, color,
        });
        items.push(LayoutedChartItem::DataLabel {
            x: MARGIN + 4.0, y: y_off + band_h / 2.0,
            text: format!("{} → {} ({})", flow.source, flow.target, flow.weight),
        });
        y_off += band_h + 4.0;
    }
    LayoutedChartDiagram { width: cw, height: ch, title_box: None, items }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::*;

    fn xy_diagram() -> ChartDiagram {
        ChartDiagram {
            title: Some("Test".into()),
            kind: ChartKind::Xy,
            x_axis: Some(Axis {
                kind: AxisKind::Categorical,
                title: None,
                categories: vec!["Jan".into(), "Feb".into(), "Mar".into()],
                min: 0.0, max: 0.0,
            }),
            y_axis: Some(Axis {
                kind: AxisKind::Numeric, title: None, categories: vec![],
                min: 0.0, max: 100.0,
            }),
            series: vec![
                ChartSeries { kind: SeriesKind::Bar, label: Some("A".into()), data: vec![40.0, 60.0, 50.0] },
                ChartSeries { kind: SeriesKind::Line, label: Some("B".into()), data: vec![35.0, 55.0, 48.0] },
            ],
            slices: vec![], sankey_nodes: vec![], flows: vec![],
            orientation: ChartOrientation::Vertical,
        }
    }

    #[test] fn version_exists() { assert_eq!(crate::VERSION, "0.1.0"); }

    #[test]
    fn xy_layout_produces_items() {
        let d = layout_chart_diagram(&xy_diagram(), 600.0, 400.0);
        assert!(d.width > 0.0);
        assert!(!d.items.is_empty());
    }

    #[test]
    fn bar_count_matches_data_points() {
        let d = layout_chart_diagram(&xy_diagram(), 600.0, 400.0);
        let bars: Vec<_> = d.items.iter().filter(|it| matches!(it, LayoutedChartItem::Bar{..})).collect();
        // 3 data points in the one bar series
        assert_eq!(bars.len(), 3);
    }

    #[test]
    fn pie_layout_produces_arcs() {
        let diagram = ChartDiagram {
            title: None, kind: ChartKind::Pie,
            x_axis: None, y_axis: None, series: vec![],
            slices: vec![
                PieSlice { label: "A".into(), value: 60.0 },
                PieSlice { label: "B".into(), value: 40.0 },
            ],
            sankey_nodes: vec![], flows: vec![],
            orientation: ChartOrientation::Vertical,
        };
        let d = layout_chart_diagram(&diagram, 400.0, 400.0);
        let arcs: Vec<_> = d.items.iter().filter(|it| matches!(it, LayoutedChartItem::PieArc{..})).collect();
        assert_eq!(arcs.len(), 2);
    }

    #[test]
    fn sankey_layout_produces_bands() {
        let diagram = ChartDiagram {
            title: None, kind: ChartKind::Sankey,
            x_axis: None, y_axis: None, series: vec![], slices: vec![],
            sankey_nodes: vec![
                SankeyNode { id: "a".into(), label: None },
                SankeyNode { id: "b".into(), label: None },
            ],
            flows: vec![
                SankeyFlow { source: "a".into(), target: "b".into(), weight: 10.0 },
                SankeyFlow { source: "a".into(), target: "c".into(), weight: 5.0 },
            ],
            orientation: ChartOrientation::Horizontal,
        };
        let d = layout_chart_diagram(&diagram, 600.0, 400.0);
        let bands: Vec<_> = d.items.iter().filter(|it| matches!(it, LayoutedChartItem::SankeyBand{..})).collect();
        assert_eq!(bands.len(), 2);
    }
}
