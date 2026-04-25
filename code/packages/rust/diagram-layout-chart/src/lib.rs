use diagram_ir::{
    Axis, AxisKind, ChartDiagram, ChartKind, ChartSeries, ChartOrientation, LegendEntry,
    LayoutedChartDiagram, LayoutedChartItem, LayoutedLabel, Orientation, Point, SeriesKind,
};

pub const VERSION: &str = "0.1.0";

const MARGIN: f64 = 24.0;
const TITLE_H: f64 = 32.0;
const Y_LBL_W: f64 = 48.0;
const X_LBL_H: f64 = 24.0;
const LEGEND_H: f64 = 28.0;
const TICK_LEN: f64 = 6.0;
const GRID_COUNT: usize = 5;
const SERIES_COLORS: &[&str] = &["#3b82f6","#ef4444","#22c55e","#f59e0b","#a855f7","#14b8a6"];

pub fn layout_chart_diagram(diagram: &ChartDiagram, canvas_width: f64, canvas_height: f64) -> LayoutedChartDiagram {
    match diagram.kind {
        ChartKind::Xy => layout_xy(diagram, canvas_width, canvas_height),
        ChartKind::Pie => layout_pie(diagram, canvas_width, canvas_height),
        ChartKind::Sankey => layout_sankey(diagram, canvas_width, canvas_height),
    }
}

fn layout_xy(diagram: &ChartDiagram, cw: f64, ch: f64) -> LayoutedChartDiagram {
    let has_title = diagram.title.is_some();
    let has_series = !diagram.series.is_empty();
    let legend_h = if has_series { LEGEND_H } else { 0.0 };
    let title_top = MARGIN;
    let plot_top = MARGIN + if has_title { TITLE_H } else { 0.0 };
    let plot_left = MARGIN + Y_LBL_W;
    let plot_bottom = ch - MARGIN - X_LBL_H - legend_h;
    let plot_right = cw - MARGIN;
    let plot_w = (plot_right - plot_left).max(1.0);
    let plot_h = (plot_bottom - plot_top).max(1.0);
    let (y_min, y_max) = resolve_y_range(diagram);
    let y_range = (y_max - y_min).max(1.0);
    let categories: Vec<String> = diagram.x_axis.as_ref().map(|a| a.categories.clone()).unwrap_or_default();
    let n_cats = categories.len().max(1);
    let bar_series_count = diagram.series.iter().filter(|s| s.kind == SeriesKind::Bar).count();
    let cat_w = plot_w / n_cats as f64;
    let bar_w = if bar_series_count > 0 { (cat_w * 0.7 / bar_series_count as f64).max(4.0) } else { cat_w * 0.7 };
    let mut items: Vec<LayoutedChartItem> = Vec::new();
    if let Some(ref title) = diagram.title {
        items.push(LayoutedChartItem::DataLabel { x: cw / 2.0, y: title_top + TITLE_H * 0.5, text: title.clone() });
    }
    for i in 0..=GRID_COUNT {
        let frac = i as f64 / GRID_COUNT as f64;
        let value = y_min + frac * y_range;
        let y = plot_bottom - frac * plot_h;
        items.push(LayoutedChartItem::GridLine { x1: plot_left, y1: y, x2: plot_right, y2: y });
        items.push(LayoutedChartItem::AxisTick { x: plot_left - TICK_LEN - 4.0, y, label: format!("{:.0}", value), orientation: Orientation::Horizontal });
    }
    items.push(LayoutedChartItem::AxisSpine { x1: plot_left, y1: plot_top, x2: plot_left, y2: plot_bottom, orientation: Orientation::Vertical });
    items.push(LayoutedChartItem::AxisSpine { x1: plot_left, y1: plot_bottom, x2: plot_right, y2: plot_bottom, orientation: Orientation::Horizontal });
    let mut bar_idx = 0usize;
    let mut legend_entries: Vec<LegendEntry> = Vec::new();
    for (s_idx, series) in diagram.series.iter().enumerate() {
        let color = SERIES_COLORS[s_idx % SERIES_COLORS.len()].to_string();
        let label = series.label.clone().unwrap_or_else(|| format!("Series {}", s_idx + 1));
        legend_entries.push(LegendEntry { color: color.clone(), label });
        match series.kind {
            SeriesKind::Bar => {
                for (cat_idx, &value) in series.data.iter().enumerate() {
                    let cat_x = plot_left + cat_idx as f64 * cat_w;
                    let bar_x = cat_x + cat_w * 0.15 + bar_idx as f64 * bar_w;
                    let scaled_h = ((value - y_min) / y_range * plot_h).max(0.0);
                    items.push(LayoutedChartItem::Bar { x: bar_x, y: plot_bottom - scaled_h, width: bar_w, height: scaled_h, color: color.clone() });
                }
                bar_idx += 1;
            }
            SeriesKind::Line => {
                let points: Vec<Point> = series.data.iter().enumerate().map(|(ci, &v)| {
                    Point { x: plot_left + ci as f64 * cat_w + cat_w * 0.5, y: plot_bottom - ((v - y_min) / y_range * plot_h).max(0.0) }
                }).collect();
                items.push(LayoutedChartItem::LinePath { points, color: color.clone() });
            }
        }
    }
    for (ci, lbl) in categories.iter().enumerate() {
        items.push(LayoutedChartItem::AxisTick { x: plot_left + ci as f64 * cat_w + cat_w * 0.5, y: plot_bottom + TICK_LEN + 4.0, label: lbl.clone(), orientation: Orientation::Vertical });
    }
    if !legend_entries.is_empty() {
        items.push(LayoutedChartItem::Legend { x: plot_left, y: plot_bottom + X_LBL_H + 4.0, entries: legend_entries });
    }
    let title_box = diagram.title.as_ref().map(|t: &String| LayoutedLabel { x: cw / 2.0, y: title_top + TITLE_H * 0.5, text: t.clone() });
    LayoutedChartDiagram { width: cw, height: ch, title_box, items }
}

fn resolve_y_range(diagram: &ChartDiagram) -> (f64, f64) {
    if let Some(Axis { kind: AxisKind::Numeric, min, max, .. }) = &diagram.y_axis { return (*min, *max); }
    let mut lo = f64::INFINITY; let mut hi = f64::NEG_INFINITY;
    for s in &diagram.series { for &v in &s.data { lo = lo.min(v); hi = hi.max(v); } }
    if lo > hi { (0.0, 100.0) } else { (0.0_f64.min(lo), hi * 1.1) }
}

fn layout_pie(diagram: &ChartDiagram, cw: f64, ch: f64) -> LayoutedChartDiagram {
    let has_title = diagram.title.is_some();
    let title_h = if has_title { TITLE_H } else { 0.0 };
    let cx = cw / 2.0; let cy = (ch + title_h) / 2.0; let r = (cw.min(ch) * 0.36).max(10.0);
    let total: f64 = diagram.slices.iter().map(|s| s.value).sum::<f64>().max(1.0);
    let mut items: Vec<LayoutedChartItem> = Vec::new();
    if let Some(ref title) = diagram.title {
        items.push(LayoutedChartItem::DataLabel { x: cw / 2.0, y: MARGIN + TITLE_H * 0.5, text: title.clone() });
    }
    let mut angle = -std::f64::consts::FRAC_PI_2;
    for (i, slice) in diagram.slices.iter().enumerate() {
        let span = slice.value / total * std::f64::consts::TAU;
        let end_angle = angle + span;
        items.push(LayoutedChartItem::PieArc { cx, cy, r, start_angle: angle, end_angle, color: SERIES_COLORS[i % SERIES_COLORS.len()].to_string(), label: slice.label.clone() });
        angle = end_angle;
    }
    let title_box = diagram.title.as_ref().map(|t: &String| LayoutedLabel { x: cw / 2.0, y: MARGIN + TITLE_H * 0.5, text: t.clone() });
    LayoutedChartDiagram { width: cw, height: ch, title_box, items }
}

fn layout_sankey(diagram: &ChartDiagram, cw: f64, ch: f64) -> LayoutedChartDiagram {
    if diagram.flows.is_empty() { return LayoutedChartDiagram { width: cw, height: ch, title_box: None, items: vec![] }; }
    let mut sources: Vec<String> = diagram.flows.iter().map(|f| f.source.clone()).collect();
    sources.sort(); sources.dedup();
    let mut targets: Vec<String> = diagram.flows.iter().map(|f| f.target.clone()).collect();
    targets.sort(); targets.dedup();
    let left_x = MARGIN * 2.0; let right_x = cw - MARGIN * 2.0;
    let total_h = ch - MARGIN * 2.0;
    let src_slot = total_h / sources.len().max(1) as f64;
    let tgt_slot = total_h / targets.len().max(1) as f64;
    let total_weight: f64 = diagram.flows.iter().map(|f| f.weight).sum::<f64>().max(1.0);
    let mut items: Vec<LayoutedChartItem> = Vec::new();
    for (i, flow) in diagram.flows.iter().enumerate() {
        let si = sources.iter().position(|s| s == &flow.source).unwrap_or(0);
        let ti = targets.iter().position(|t| t == &flow.target).unwrap_or(0);
        let band_w = (flow.weight / total_weight * (total_h * 0.4)).max(2.0);
        items.push(LayoutedChartItem::SankeyBand { from_x: left_x, from_y: MARGIN + si as f64 * src_slot + src_slot / 2.0, to_x: right_x, to_y: MARGIN + ti as f64 * tgt_slot + tgt_slot / 2.0, width: band_w, color: SERIES_COLORS[i % SERIES_COLORS.len()].to_string() });
    }
    let title_box = diagram.title.as_ref().map(|t: &String| LayoutedLabel { x: cw / 2.0, y: MARGIN + TITLE_H * 0.5, text: t.clone() });
    LayoutedChartDiagram { width: cw, height: ch, title_box, items }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::{Axis, AxisKind, ChartDiagram, ChartKind, ChartOrientation, ChartSeries, PieSlice, SankeyFlow, SankeyNode, SeriesKind};
    fn xy_diagram() -> ChartDiagram {
        ChartDiagram { title: Some("Test".to_string()), kind: ChartKind::Xy,
            x_axis: Some(Axis { kind: AxisKind::Categorical, title: None, categories: vec!["Jan".into(),"Feb".into(),"Mar".into()], min: 0.0, max: 0.0 }),
            y_axis: Some(Axis { kind: AxisKind::Numeric, title: None, categories: vec![], min: 0.0, max: 100.0 }),
            series: vec![ChartSeries{kind:SeriesKind::Bar,label:Some("R".into()),data:vec![40.0,60.0,45.0]},
                         ChartSeries{kind:SeriesKind::Line,label:Some("T".into()),data:vec![50.0,50.0,50.0]}],
            slices:vec![], sankey_nodes:vec![], flows:vec![], orientation:ChartOrientation::Vertical }
    }
    #[test] fn version_exists() { assert_eq!(VERSION, "0.1.0"); }
    #[test] fn xy_layout_produces_items() {
        let out = layout_chart_diagram(&xy_diagram(), 800.0, 500.0);
        assert_eq!(out.width, 800.0); assert!(out.title_box.is_some());
        assert!(out.items.iter().any(|i| matches!(i, LayoutedChartItem::Bar{..})));
        assert!(out.items.iter().any(|i| matches!(i, LayoutedChartItem::LinePath{..})));
    }
    #[test] fn bar_count_matches_data_points() {
        let out = layout_chart_diagram(&xy_diagram(), 800.0, 500.0);
        let bars: Vec<_> = out.items.iter().filter(|i| matches!(i, LayoutedChartItem::Bar{..})).collect();
        assert_eq!(bars.len(), 3);
    }
    #[test] fn pie_layout_produces_arcs() {
        let d = ChartDiagram { title:None, kind:ChartKind::Pie, x_axis:None, y_axis:None, series:vec![],
            slices:vec![PieSlice{label:"A".into(),value:60.0},PieSlice{label:"B".into(),value:40.0}],
            sankey_nodes:vec![], flows:vec![], orientation:ChartOrientation::default() };
        let out = layout_chart_diagram(&d, 600.0, 400.0);
        assert_eq!(out.items.iter().filter(|i| matches!(i,LayoutedChartItem::PieArc{..})).count(), 2);
    }
    #[test] fn sankey_layout_produces_bands() {
        let d = ChartDiagram { title:None, kind:ChartKind::Sankey, x_axis:None, y_axis:None, series:vec![], slices:vec![],
            sankey_nodes:vec![SankeyNode{id:"A".into(),label:None},SankeyNode{id:"B".into(),label:None}],
            flows:vec![SankeyFlow{source:"A".into(),target:"B".into(),weight:100.0}], orientation:ChartOrientation::default() };
        let out = layout_chart_diagram(&d, 600.0, 400.0);
        assert_eq!(out.items.iter().filter(|i| matches!(i,LayoutedChartItem::SankeyBand{..})).count(), 1);
    }
}
