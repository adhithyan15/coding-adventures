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
    AxisKind, ChartDiagram, ChartKind, LayoutedChartDiagram, LayoutedChartItem, LegendEntry,
    Orientation, Point, SeriesKind,
};
use std::collections::{HashMap, VecDeque};

pub const VERSION: &str = "0.16.0";

const MARGIN: f64 = 24.0;
const TITLE_H: f64 = 32.0;
const Y_LBL_W: f64 = 48.0;
const LEGEND_H: f64 = 28.0;
const TICK_LEN: f64 = 6.0;
const GRID_COUNT: usize = 5;
const POINT_LABEL_FONT_SIZE: f64 = 12.0;

fn point_label_size(label: &str) -> (f64, f64) {
    ((label.chars().count() as f64 * 7.0 + 4.0).max(12.0), 14.4)
}

fn chart_value_label(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        let formatted = format!("{value:.6}");
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn maximum_series_len(diagram: &ChartDiagram) -> usize {
    diagram
        .series
        .iter()
        .map(|series| series.data.len())
        .max()
        .unwrap_or(0)
}

fn indexed_axis_position(
    start: f64,
    length: f64,
    index: usize,
    count: usize,
    numeric: bool,
) -> f64 {
    if numeric {
        let fraction = if count <= 1 {
            0.0
        } else {
            index as f64 / (count - 1) as f64
        };
        start + fraction * length
    } else {
        start + (index as f64 + 0.5) * length / count.max(1) as f64
    }
}

const SERIES_COLORS: &[&str] = &[
    "#3b82f6", "#ef4444", "#22c55e", "#f59e0b", "#a855f7", "#14b8a6",
];

/// Lay out a `ChartDiagram` on a canvas of `cw × ch` pixels.
pub fn layout_chart_diagram(diagram: &ChartDiagram, cw: f64, ch: f64) -> LayoutedChartDiagram {
    match diagram.kind {
        ChartKind::Xy => layout_xy(diagram, cw, ch),
        ChartKind::Pie => layout_pie(diagram, cw, ch),
        ChartKind::Sankey => layout_sankey(diagram, cw, ch),
        ChartKind::Quadrant => layout_quadrant(diagram, cw, ch),
    }
}

// ── XY layout ────────────────────────────────────────────────────────────

fn resolve_y_range(diagram: &ChartDiagram) -> (f64, f64) {
    if let Some(ref ya) = diagram.y_axis {
        if ya.min < ya.max {
            return (ya.min, ya.max);
        }
    }
    let all: Vec<f64> = diagram
        .series
        .iter()
        .flat_map(|series| series.data.iter().map(|point| point.value))
        .collect();
    if all.is_empty() {
        return (0.0, 100.0);
    }
    let mn = all.iter().cloned().fold(f64::INFINITY, f64::min).min(0.0);
    let mx = all.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    (mn, if mx == mn { mn + 1.0 } else { mx })
}

fn layout_xy(diagram: &ChartDiagram, cw: f64, ch: f64) -> LayoutedChartDiagram {
    let cw = diagram.xy_config.width.unwrap_or(cw);
    let ch = diagram.xy_config.height.unwrap_or(ch);
    match diagram.orientation {
        diagram_ir::ChartOrientation::Vertical => layout_xy_vertical(diagram, cw, ch),
        diagram_ir::ChartOrientation::Horizontal => layout_xy_horizontal(diagram, cw, ch),
    }
}

fn layout_xy_vertical(diagram: &ChartDiagram, cw: f64, ch: f64) -> LayoutedChartDiagram {
    let x_config = &diagram.xy_config.x_axis;
    let y_config = &diagram.xy_config.y_axis;
    let show_title = diagram.xy_config.show_title.unwrap_or(true);
    let has_title = show_title && diagram.title.is_some();
    let title_font_size = diagram.xy_config.title_font_size.unwrap_or(20.0);
    let title_padding = diagram.xy_config.title_padding.unwrap_or(10.0);
    let title_height = if has_title {
        title_font_size + title_padding * 2.0
    } else {
        0.0
    };
    let has_series = !diagram.series.is_empty();
    let has_x_title = x_config.show_title.unwrap_or(true)
        && diagram
        .x_axis
        .as_ref()
        .and_then(|axis| axis.title.as_ref())
        .is_some();
    let has_y_title = y_config.show_title.unwrap_or(true)
        && diagram
        .y_axis
        .as_ref()
        .and_then(|axis| axis.title.as_ref())
        .is_some();
    let x_label_font_size = x_config.label_font_size.unwrap_or(14.0);
    let y_label_font_size = y_config.label_font_size.unwrap_or(14.0);
    let x_label_padding = x_config.label_padding.unwrap_or(5.0);
    let y_label_padding = y_config.label_padding.unwrap_or(5.0);
    let x_label_space = if x_config.show_label.unwrap_or(true) {
        x_label_font_size + x_label_padding * 2.0
    } else {
        0.0
    };
    let y_label_space = if y_config.show_label.unwrap_or(true) {
        Y_LBL_W + y_label_padding
    } else {
        0.0
    };
    let x_title_font_size = x_config.title_font_size.unwrap_or(16.0);
    let y_title_font_size = y_config.title_font_size.unwrap_or(16.0);
    let x_title_space = if has_x_title {
        x_title_font_size + x_config.title_padding.unwrap_or(5.0) * 2.0
    } else {
        0.0
    };
    let y_title_space = if has_y_title {
        y_title_font_size + y_config.title_padding.unwrap_or(5.0) * 2.0
    } else {
        0.0
    };
    let lh = if has_series { LEGEND_H } else { 0.0 };

    // Plot area bounds
    let pt = MARGIN + title_height; // top
    let pl = MARGIN + y_label_space + y_title_space; // left
    let pb = ch - MARGIN - x_label_space - x_title_space - lh; // bottom
    let pr = cw - MARGIN; // right
    let pw = (pr - pl).max(1.0);
    let ph = (pb - pt).max(1.0);

    let (ym, yx) = resolve_y_range(diagram);
    let yr = (yx - ym).max(1.0);

    let cats: Vec<String> = diagram
        .x_axis
        .as_ref()
        .map(|a| a.categories.clone())
        .unwrap_or_default();
    let numeric_x_axis = diagram
        .x_axis
        .as_ref()
        .is_some_and(|axis| axis.kind == AxisKind::Numeric);
    let point_count = maximum_series_len(diagram).max(1);
    let slot_count = if numeric_x_axis {
        point_count
    } else {
        cats.len().max(1)
    };
    let nb = diagram
        .series
        .iter()
        .filter(|s| s.kind == SeriesKind::Bar)
        .count();
    let cat_w = pw / slot_count as f64;
    let bar_w = if nb > 0 {
        (cat_w * 0.7 / nb as f64).max(4.0)
    } else {
        cat_w * 0.7
    };

    let mut items: Vec<LayoutedChartItem> = Vec::new();

    // Title
    if has_title {
        let t = diagram.title.as_ref().expect("visible XY title");
        items.push(LayoutedChartItem::DataLabel {
            x: cw / 2.0,
            y: MARGIN + title_padding + title_font_size * 0.5,
            text: t.clone(),
            font_size: Some(title_font_size),
            color: None,
        });
    }

    if has_x_title {
        let axis_title = diagram
            .x_axis
            .as_ref()
            .and_then(|axis| axis.title.as_ref())
            .expect("visible XY x-axis title");
        items.push(LayoutedChartItem::DataLabel {
            x: (pl + pr) / 2.0,
            y: pb + x_label_space + x_title_space / 2.0,
            text: axis_title.clone(),
            font_size: Some(x_title_font_size),
            color: None,
        });
    }
    if has_y_title {
        let axis_title = diagram
            .y_axis
            .as_ref()
            .and_then(|axis| axis.title.as_ref())
            .expect("visible XY y-axis title");
        items.push(LayoutedChartItem::DataLabel {
            x: MARGIN + y_title_space / 2.0,
            y: (pt + pb) / 2.0,
            text: axis_title.clone(),
            font_size: Some(y_title_font_size),
            color: None,
        });
    }

    // Y-axis grid lines + tick labels
    for i in 0..=GRID_COUNT {
        let frac = i as f64 / GRID_COUNT as f64;
        let val = ym + frac * yr;
        let y = pb - frac * ph;
        items.push(LayoutedChartItem::GridLine {
            x1: pl,
            y1: y,
            x2: pr,
            y2: y,
        });
        if y_config.show_label.unwrap_or(true) {
            items.push(LayoutedChartItem::AxisTick {
                x: pl - TICK_LEN - y_label_padding,
                y,
                label: format!("{val:.0}"),
                orientation: Orientation::Horizontal,
                font_size: y_label_font_size,
            });
        }
    }

    if numeric_x_axis {
        let axis = diagram.x_axis.as_ref().expect("numeric x-axis");
        for index in 0..=GRID_COUNT {
            let fraction = index as f64 / GRID_COUNT as f64;
            if x_config.show_label.unwrap_or(true) {
                items.push(LayoutedChartItem::AxisTick {
                    x: pl + fraction * pw,
                    y: pb + TICK_LEN + x_label_padding,
                    label: format!("{:.0}", axis.min + fraction * (axis.max - axis.min)),
                    orientation: Orientation::Vertical,
                    font_size: x_label_font_size,
                });
            }
        }
    } else {
        for (i, cat) in cats.iter().enumerate() {
            let cx = indexed_axis_position(pl, pw, i, cats.len(), false);
            if x_config.show_label.unwrap_or(true) {
                items.push(LayoutedChartItem::AxisTick {
                    x: cx,
                    y: pb + TICK_LEN + x_label_padding,
                    label: cat.clone(),
                    orientation: Orientation::Vertical,
                    font_size: x_label_font_size,
                });
            }
        }
    }

    // Axis spines
    if x_config.show_axis_line.unwrap_or(true) {
        items.push(LayoutedChartItem::AxisSpine {
            x1: pl,
            y1: pb,
            x2: pr,
            y2: pb,
            orientation: Orientation::Horizontal,
            stroke_width: x_config.axis_line_width.unwrap_or(2.0),
        });
    }
    if y_config.show_axis_line.unwrap_or(true) {
        items.push(LayoutedChartItem::AxisSpine {
            x1: pl,
            y1: pt,
            x2: pl,
            y2: pb,
            orientation: Orientation::Vertical,
            stroke_width: y_config.axis_line_width.unwrap_or(2.0),
        });
    }

    // Series (bars + lines)
    let mut bar_series_idx = 0usize;
    let mut legend_entries: Vec<LegendEntry> = Vec::new();

    for (si, series) in diagram.series.iter().enumerate() {
        let color = SERIES_COLORS[si % SERIES_COLORS.len()].to_string();
        if let Some(ref lbl) = series.label {
            legend_entries.push(LegendEntry {
                color: color.clone(),
                label: lbl.clone(),
            });
        }
        match series.kind {
            SeriesKind::Bar => {
                let group_width = bar_w * nb as f64;
                for (ci, point) in series.data.iter().enumerate() {
                    let val = point.value;
                    let bh = ((val - ym) / yr * ph).max(0.0);
                    let center = indexed_axis_position(
                        pl,
                        pw,
                        ci,
                        if numeric_x_axis {
                            series.data.len()
                        } else {
                            cats.len()
                        },
                        numeric_x_axis,
                    );
                    let group_x =
                        (center - group_width / 2.0).clamp(pl, (pr - group_width).max(pl));
                    let bx = group_x + bar_series_idx as f64 * bar_w;
                    let by = pb - bh;
                    items.push(LayoutedChartItem::Bar {
                        x: bx,
                        y: by,
                        width: bar_w,
                        height: bh,
                        color: color.clone(),
                    });
                    if diagram.xy_config.show_data_label.unwrap_or(false) && bar_w > 0.0 && bh > 0.0
                    {
                        let text = chart_value_label(val);
                        let outside = diagram
                            .xy_config
                            .show_data_label_outside_bar
                            .unwrap_or(false);
                        let font_size = if outside {
                            12.0
                        } else {
                            (bar_w / (text.chars().count().max(1) as f64 * 0.7))
                                .min((bh - 10.0).max(1.0))
                                .min(12.0)
                                .floor()
                                .max(1.0)
                        };
                        let label_width = if outside {
                            (text.chars().count() as f64 * font_size * 0.7 + 4.0).max(bar_w)
                        } else {
                            bar_w
                        };
                        items.push(LayoutedChartItem::BarLabel {
                            x: bx + bar_w / 2.0 - label_width / 2.0,
                            y: if outside {
                                by - 10.0 - font_size * 1.2
                            } else {
                                by + 10.0
                            },
                            width: label_width,
                            height: font_size * 1.2,
                            text,
                            font_size,
                            color: diagram
                                .xy_config
                                .data_label_color
                                .clone()
                                .unwrap_or_else(|| "#374151".into()),
                        });
                    }
                }
                bar_series_idx += 1;
            }
            SeriesKind::Line => {
                let pts: Vec<Point> = series
                    .data
                    .iter()
                    .enumerate()
                    .map(|(ci, point)| {
                        let lx = indexed_axis_position(
                            pl,
                            pw,
                            ci,
                            if numeric_x_axis {
                                series.data.len()
                            } else {
                                cats.len()
                            },
                            numeric_x_axis,
                        );
                        let ly = pb - (point.value - ym) / yr * ph;
                        Point { x: lx, y: ly }
                    })
                    .collect();
                if !pts.is_empty() {
                    items.push(LayoutedChartItem::LinePath {
                        points: pts.clone(),
                        color: color.clone(),
                    });
                }
                for (point, position) in series.data.iter().zip(&pts) {
                    if let Some(label) = &point.label {
                        let (width, height) = point_label_size(label);
                        items.push(LayoutedChartItem::PointLabel {
                            x: position.x - width / 2.0,
                            y: position.y - 10.0 - height / 2.0,
                            width,
                            height,
                            text: label.clone(),
                            font_size: POINT_LABEL_FONT_SIZE,
                            color: color.clone(),
                        });
                    }
                }
            }
        }
    }

    // Legend
    if !legend_entries.is_empty() {
        items.push(LayoutedChartItem::Legend {
            x: pl,
            y: ch - lh / 2.0,
            entries: legend_entries,
        });
    }

    LayoutedChartDiagram {
        width: cw,
        height: ch,
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        title_box: None,
        items,
    }
}

fn layout_xy_horizontal(diagram: &ChartDiagram, cw: f64, ch: f64) -> LayoutedChartDiagram {
    let x_config = &diagram.xy_config.x_axis;
    let y_config = &diagram.xy_config.y_axis;
    let show_title = diagram.xy_config.show_title.unwrap_or(true);
    let has_title = show_title && diagram.title.is_some();
    let title_font_size = diagram.xy_config.title_font_size.unwrap_or(20.0);
    let title_padding = diagram.xy_config.title_padding.unwrap_or(10.0);
    let title_height = if has_title {
        title_font_size + title_padding * 2.0
    } else {
        0.0
    };
    let has_series = !diagram.series.is_empty();
    let has_x_title = x_config.show_title.unwrap_or(true)
        && diagram
        .x_axis
        .as_ref()
        .and_then(|axis| axis.title.as_ref())
        .is_some();
    let has_y_title = y_config.show_title.unwrap_or(true)
        && diagram
        .y_axis
        .as_ref()
        .and_then(|axis| axis.title.as_ref())
        .is_some();
    let x_label_font_size = x_config.label_font_size.unwrap_or(14.0);
    let y_label_font_size = y_config.label_font_size.unwrap_or(14.0);
    let x_label_padding = x_config.label_padding.unwrap_or(5.0);
    let y_label_padding = y_config.label_padding.unwrap_or(5.0);
    let x_label_space = if x_config.show_label.unwrap_or(true) {
        Y_LBL_W + x_label_padding
    } else {
        0.0
    };
    let y_label_space = if y_config.show_label.unwrap_or(true) {
        y_label_font_size + y_label_padding * 2.0
    } else {
        0.0
    };
    let x_title_font_size = x_config.title_font_size.unwrap_or(16.0);
    let y_title_font_size = y_config.title_font_size.unwrap_or(16.0);
    let x_title_space = if has_x_title {
        x_title_font_size + x_config.title_padding.unwrap_or(5.0) * 2.0
    } else {
        0.0
    };
    let y_title_space = if has_y_title {
        y_title_font_size + y_config.title_padding.unwrap_or(5.0) * 2.0
    } else {
        0.0
    };
    let legend_height = if has_series { LEGEND_H } else { 0.0 };

    let top = MARGIN + title_height;
    let left = MARGIN + x_label_space + x_title_space;
    let bottom = ch - MARGIN - y_label_space - y_title_space - legend_height;
    let right = cw - MARGIN;
    let plot_width = (right - left).max(1.0);
    let plot_height = (bottom - top).max(1.0);

    let (minimum, maximum) = resolve_y_range(diagram);
    let range = (maximum - minimum).max(1.0);
    let categories = diagram
        .x_axis
        .as_ref()
        .map(|axis| axis.categories.clone())
        .unwrap_or_default();
    let numeric_x_axis = diagram
        .x_axis
        .as_ref()
        .is_some_and(|axis| axis.kind == AxisKind::Numeric);
    let point_count = maximum_series_len(diagram).max(1);
    let category_count = if numeric_x_axis {
        point_count
    } else {
        categories.len().max(1)
    };
    let bar_series_count = diagram
        .series
        .iter()
        .filter(|series| series.kind == SeriesKind::Bar)
        .count();
    let category_height = plot_height / category_count as f64;
    let bar_height = if bar_series_count > 0 {
        (category_height * 0.7 / bar_series_count as f64).max(4.0)
    } else {
        category_height * 0.7
    };
    let mut items = Vec::new();

    if has_title {
        let title = diagram.title.as_ref().expect("visible XY title");
        items.push(LayoutedChartItem::DataLabel {
            x: cw / 2.0,
            y: MARGIN + title_padding + title_font_size * 0.5,
            text: title.clone(),
            font_size: Some(title_font_size),
            color: None,
        });
    }
    if has_x_title {
        let axis_title = diagram
            .x_axis
            .as_ref()
            .and_then(|axis| axis.title.as_ref())
            .expect("visible horizontal XY x-axis title");
        items.push(LayoutedChartItem::DataLabel {
            x: MARGIN + x_title_space / 2.0,
            y: (top + bottom) / 2.0,
            text: axis_title.clone(),
            font_size: Some(x_title_font_size),
            color: None,
        });
    }
    if has_y_title {
        let axis_title = diagram
            .y_axis
            .as_ref()
            .and_then(|axis| axis.title.as_ref())
            .expect("visible horizontal XY y-axis title");
        items.push(LayoutedChartItem::DataLabel {
            x: (left + right) / 2.0,
            y: bottom + y_label_space + y_title_space / 2.0,
            text: axis_title.clone(),
            font_size: Some(y_title_font_size),
            color: None,
        });
    }

    for index in 0..=GRID_COUNT {
        let fraction = index as f64 / GRID_COUNT as f64;
        let value = minimum + fraction * range;
        let x = left + fraction * plot_width;
        items.push(LayoutedChartItem::GridLine {
            x1: x,
            y1: top,
            x2: x,
            y2: bottom,
        });
        if y_config.show_label.unwrap_or(true) {
            items.push(LayoutedChartItem::AxisTick {
                x,
                y: bottom + TICK_LEN + y_label_padding,
                label: format!("{value:.0}"),
                orientation: Orientation::Vertical,
                font_size: y_label_font_size,
            });
        }
    }
    if numeric_x_axis {
        let axis = diagram.x_axis.as_ref().expect("numeric x-axis");
        for index in 0..=GRID_COUNT {
            let fraction = index as f64 / GRID_COUNT as f64;
            if x_config.show_label.unwrap_or(true) {
                items.push(LayoutedChartItem::AxisTick {
                    x: left - TICK_LEN - x_label_padding,
                    y: top + fraction * plot_height,
                    label: format!("{:.0}", axis.min + fraction * (axis.max - axis.min)),
                    orientation: Orientation::Horizontal,
                    font_size: x_label_font_size,
                });
            }
        }
    } else {
        for (index, category) in categories.iter().enumerate() {
            if x_config.show_label.unwrap_or(true) {
                items.push(LayoutedChartItem::AxisTick {
                    x: left - TICK_LEN - x_label_padding,
                    y: indexed_axis_position(top, plot_height, index, categories.len(), false),
                    label: category.clone(),
                    orientation: Orientation::Horizontal,
                    font_size: x_label_font_size,
                });
            }
        }
    }
    if y_config.show_axis_line.unwrap_or(true) {
        items.push(LayoutedChartItem::AxisSpine {
            x1: left,
            y1: bottom,
            x2: right,
            y2: bottom,
            orientation: Orientation::Horizontal,
            stroke_width: y_config.axis_line_width.unwrap_or(2.0),
        });
    }
    if x_config.show_axis_line.unwrap_or(true) {
        items.push(LayoutedChartItem::AxisSpine {
            x1: left,
            y1: top,
            x2: left,
            y2: bottom,
            orientation: Orientation::Vertical,
            stroke_width: x_config.axis_line_width.unwrap_or(2.0),
        });
    }

    let mut bar_series_index = 0usize;
    let mut legend_entries = Vec::new();
    for (series_index, series) in diagram.series.iter().enumerate() {
        let color = SERIES_COLORS[series_index % SERIES_COLORS.len()].to_string();
        if let Some(label) = &series.label {
            legend_entries.push(LegendEntry {
                color: color.clone(),
                label: label.clone(),
            });
        }
        match series.kind {
            SeriesKind::Bar => {
                let group_height = bar_height * bar_series_count as f64;
                for (category_index, point) in series.data.iter().enumerate() {
                    let width = ((point.value - minimum) / range * plot_width).max(0.0);
                    let center = indexed_axis_position(
                        top,
                        plot_height,
                        category_index,
                        if numeric_x_axis {
                            series.data.len()
                        } else {
                            categories.len()
                        },
                        numeric_x_axis,
                    );
                    let group_y = (center - group_height / 2.0)
                        .clamp(top, (bottom - group_height).max(top));
                    let y = group_y + bar_series_index as f64 * bar_height;
                    items.push(LayoutedChartItem::Bar {
                        x: left,
                        y,
                        width,
                        height: bar_height,
                        color: color.clone(),
                    });
                    if diagram.xy_config.show_data_label.unwrap_or(false)
                        && width > 0.0
                        && bar_height > 0.0
                    {
                        let text = chart_value_label(point.value);
                        let outside = diagram
                            .xy_config
                            .show_data_label_outside_bar
                            .unwrap_or(false);
                        let available_width = (width - 20.0).max(1.0);
                        let font_size = if outside {
                            12.0
                        } else {
                            (bar_height * 0.7)
                                .min(available_width / (text.chars().count().max(1) as f64 * 0.7))
                                .floor()
                                .max(1.0)
                        };
                        let label_width = if outside {
                            (text.chars().count() as f64 * font_size * 0.7 + 4.0).max(20.0)
                        } else {
                            available_width
                        };
                        items.push(LayoutedChartItem::BarLabel {
                            x: if outside {
                                left + width + 10.0
                            } else {
                                left + 10.0
                            },
                            y,
                            width: label_width,
                            height: bar_height,
                            text,
                            font_size,
                            color: diagram
                                .xy_config
                                .data_label_color
                                .clone()
                                .unwrap_or_else(|| "#374151".into()),
                        });
                    }
                }
                bar_series_index += 1;
            }
            SeriesKind::Line => {
                let points = series
                    .data
                    .iter()
                    .enumerate()
                    .map(|(category_index, point)| Point {
                        x: left + (point.value - minimum) / range * plot_width,
                        y: indexed_axis_position(
                            top,
                            plot_height,
                            category_index,
                            if numeric_x_axis {
                                series.data.len()
                            } else {
                                categories.len()
                            },
                            numeric_x_axis,
                        ),
                    })
                    .collect::<Vec<_>>();
                if !points.is_empty() {
                    items.push(LayoutedChartItem::LinePath {
                        points: points.clone(),
                        color: color.clone(),
                    });
                }
                for (point, position) in series.data.iter().zip(&points) {
                    if let Some(label) = &point.label {
                        let (width, height) = point_label_size(label);
                        items.push(LayoutedChartItem::PointLabel {
                            x: position.x + 10.0,
                            y: position.y - height / 2.0,
                            width,
                            height,
                            text: label.clone(),
                            font_size: POINT_LABEL_FONT_SIZE,
                            color: color.clone(),
                        });
                    }
                }
            }
        }
    }
    if !legend_entries.is_empty() {
        items.push(LayoutedChartItem::Legend {
            x: left,
            y: ch - legend_height / 2.0,
            entries: legend_entries,
        });
    }

    LayoutedChartDiagram {
        width: cw,
        height: ch,
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        title_box: None,
        items,
    }
}

// ── Pie layout ────────────────────────────────────────────────────────────

const PIE_COLORS: &[&str] = &[
    "#3b82f6", "#ef4444", "#22c55e", "#f59e0b", "#a855f7", "#14b8a6", "#f97316", "#8b5cf6",
];

fn layout_pie(diagram: &ChartDiagram, cw: f64, ch: f64) -> LayoutedChartDiagram {
    let cx = cw / 2.0;
    let cy = ch / 2.0;
    let r = (cw.min(ch - LEGEND_H) / 2.0 - MARGIN * 2.0).max(10.0);
    let total: f64 = diagram.slices.iter().map(|s| s.value).sum();
    let total = if total == 0.0 { 1.0 } else { total };
    let mut angle = -std::f64::consts::FRAC_PI_2; // start at 12 o'clock
    let mut items: Vec<LayoutedChartItem> = Vec::new();

    if let Some(ref t) = diagram.title {
        items.push(LayoutedChartItem::DataLabel {
            x: cw / 2.0,
            y: MARGIN + TITLE_H * 0.5,
            text: t.clone(),
            font_size: None,
            color: None,
        });
    }

    for (i, slice) in diagram.slices.iter().enumerate() {
        let delta = slice.value / total * std::f64::consts::TAU;
        let end = angle + delta;
        let color = PIE_COLORS[i % PIE_COLORS.len()].to_string();
        items.push(LayoutedChartItem::PieArc {
            cx,
            cy,
            r,
            start_angle: angle,
            end_angle: end,
            color,
            label: format!("{:.0}%", slice.value / total * 100.0),
        });
        angle = end;
    }

    let legend_entries = diagram
        .slices
        .iter()
        .enumerate()
        .map(|(i, slice)| LegendEntry {
            color: PIE_COLORS[i % PIE_COLORS.len()].to_string(),
            label: if diagram.show_data {
                format!("{} [{}]", slice.label, slice.value)
            } else {
                slice.label.clone()
            },
        })
        .collect();
    items.push(LayoutedChartItem::Legend {
        x: MARGIN,
        y: ch - MARGIN,
        entries: legend_entries,
    });

    LayoutedChartDiagram {
        width: cw,
        height: ch,
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        title_box: None,
        items,
    }
}

// ── Sankey layout ─────────────────────────────────────────────────────────

fn layout_sankey(diagram: &ChartDiagram, cw: f64, ch: f64) -> LayoutedChartDiagram {
    const NODE_W: f64 = 14.0;
    const NODE_GAP: f64 = 16.0;

    let mut incoming = HashMap::<String, f64>::new();
    let mut outgoing = HashMap::<String, f64>::new();
    let mut indegree = HashMap::<String, usize>::new();
    let mut rank = HashMap::<String, usize>::new();
    for node in &diagram.sankey_nodes {
        incoming.insert(node.id.clone(), 0.0);
        outgoing.insert(node.id.clone(), 0.0);
        indegree.insert(node.id.clone(), 0);
        rank.insert(node.id.clone(), 0);
    }
    for flow in &diagram.flows {
        *outgoing.entry(flow.source.clone()).or_default() += flow.weight;
        *incoming.entry(flow.target.clone()).or_default() += flow.weight;
        *indegree.entry(flow.target.clone()).or_default() += 1;
    }

    let mut queue = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
        .collect::<VecDeque<_>>();
    while let Some(source) = queue.pop_front() {
        let source_rank = rank[&source];
        for flow in diagram.flows.iter().filter(|flow| flow.source == source) {
            let target_rank = rank.entry(flow.target.clone()).or_default();
            *target_rank = (*target_rank).max(source_rank + 1);
            let degree = indegree.entry(flow.target.clone()).or_default();
            *degree -= 1;
            if *degree == 0 {
                queue.push_back(flow.target.clone());
            }
        }
    }

    let max_rank = rank.values().copied().max().unwrap_or(0).max(1);
    let mut columns = vec![Vec::<String>::new(); max_rank + 1];
    for node in &diagram.sankey_nodes {
        columns[rank[&node.id].min(max_rank)].push(node.id.clone());
    }
    let plot_h = ch - MARGIN * 2.0;
    let scale = columns
        .iter()
        .filter(|column| !column.is_empty())
        .map(|column| {
            let values = column
                .iter()
                .map(|id| incoming[id].max(outgoing[id]).max(1.0))
                .sum::<f64>();
            (plot_h - NODE_GAP * (column.len().saturating_sub(1) as f64)) / values
        })
        .fold(f64::INFINITY, f64::min)
        .max(0.1);

    let mut geometry = HashMap::<String, (f64, f64, f64)>::new();
    for (column_index, column) in columns.iter().enumerate() {
        let x = MARGIN + column_index as f64 / max_rank as f64 * (cw - MARGIN * 2.0 - NODE_W);
        let column_height = column
            .iter()
            .map(|id| incoming[id].max(outgoing[id]).max(1.0) * scale)
            .sum::<f64>()
            + NODE_GAP * column.len().saturating_sub(1) as f64;
        let mut y = MARGIN + (plot_h - column_height) / 2.0;
        for id in column {
            let height = incoming[id].max(outgoing[id]).max(1.0) * scale;
            geometry.insert(id.clone(), (x, y, height));
            y += height + NODE_GAP;
        }
    }

    let mut source_offsets = HashMap::<String, f64>::new();
    let mut target_offsets = HashMap::<String, f64>::new();
    let mut items: Vec<LayoutedChartItem> = Vec::new();
    for (i, flow) in diagram.flows.iter().enumerate() {
        let (source_x, source_y, _) = geometry[&flow.source];
        let (target_x, target_y, _) = geometry[&flow.target];
        let width = (flow.weight * scale).max(1.0);
        let source_offset = source_offsets.entry(flow.source.clone()).or_default();
        let target_offset = target_offsets.entry(flow.target.clone()).or_default();
        items.push(LayoutedChartItem::SankeyBand {
            from_x: source_x + NODE_W,
            from_y: source_y + *source_offset,
            to_x: target_x,
            to_y: target_y + *target_offset,
            width,
            color: SERIES_COLORS[i % SERIES_COLORS.len()].to_string(),
        });
        *source_offset += width;
        *target_offset += width;
    }
    for (i, node) in diagram.sankey_nodes.iter().enumerate() {
        let (x, y, height) = geometry[&node.id];
        items.push(LayoutedChartItem::SankeyNode {
            x,
            y,
            width: NODE_W,
            height,
            color: SERIES_COLORS[i % SERIES_COLORS.len()].to_string(),
            label: node.label.clone().unwrap_or_else(|| node.id.clone()),
        });
    }
    LayoutedChartDiagram {
        width: cw,
        height: ch,
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        title_box: None,
        items,
    }
}

// ── Quadrant layout ──────────────────────────────────────────────────────

fn layout_quadrant(diagram: &ChartDiagram, cw: f64, ch: f64) -> LayoutedChartDiagram {
    let cw = diagram.quadrant_config.chart_width.unwrap_or(cw);
    let ch = diagram.quadrant_config.chart_height.unwrap_or(ch);
    let title_padding = diagram.quadrant_config.title_padding.unwrap_or(0.0);
    let title_height = if diagram.title.is_some() {
        diagram.quadrant_config.title_font_size.unwrap_or(TITLE_H) + title_padding * 2.0
    } else {
        0.0
    };
    let y_axis_right = diagram.quadrant_config.y_axis_position.as_deref() == Some("right");
    let x_axis_top = diagram.quadrant_config.x_axis_position.as_deref() == Some("top");
    let padding = diagram.quadrant_config.quadrant_padding.unwrap_or(0.0);
    let left = MARGIN + if y_axis_right { 0.0 } else { 56.0 } + padding;
    let right = cw - MARGIN - if y_axis_right { 56.0 } else { 0.0 } - padding;
    let top = MARGIN + title_height + padding;
    let bottom = ch - MARGIN - 36.0 - padding;
    let width = (right - left).max(1.0);
    let height = (bottom - top).max(1.0);
    let half_width = width / 2.0;
    let half_height = height / 2.0;
    let default_colors = ["#dbeafe", "#dcfce7", "#fef3c7", "#fee2e2"];
    let regions = [
        (left + half_width, top),
        (left, top),
        (left, top + half_height),
        (left + half_width, top + half_height),
    ];
    let mut items = Vec::new();

    if let Some(title) = &diagram.title {
        items.push(LayoutedChartItem::DataLabel {
            x: cw / 2.0,
            y: MARGIN
                + title_padding
                + diagram.quadrant_config.title_font_size.unwrap_or(TITLE_H) / 2.0,
            text: title.clone(),
            font_size: diagram.quadrant_config.title_font_size,
            color: diagram.quadrant_config.title_fill.clone(),
        });
    }

    for (index, (x, y)) in regions.into_iter().enumerate() {
        items.push(LayoutedChartItem::QuadrantRegion {
            x,
            y,
            width: half_width,
            height: half_height,
            color: diagram.quadrant_config.quadrant_fills[index]
                .clone()
                .unwrap_or_else(|| default_colors[index].to_string()),
            label: diagram.quadrant_labels[index].clone(),
            label_font_size: diagram.quadrant_config.quadrant_label_font_size,
            label_top_padding: diagram
                .quadrant_config
                .quadrant_text_top_padding
                .unwrap_or(8.0),
            label_color: diagram.quadrant_config.quadrant_text_fills[index]
                .clone()
                .unwrap_or_else(|| "#334155".to_string()),
        });
    }
    items.push(LayoutedChartItem::QuadrantBorder {
        x: left,
        y: top,
        width,
        height,
        internal_color: diagram
            .quadrant_config
            .internal_border_stroke_fill
            .clone()
            .unwrap_or_else(|| "#64748b".to_string()),
        external_color: diagram
            .quadrant_config
            .external_border_stroke_fill
            .clone()
            .unwrap_or_else(|| "#64748b".to_string()),
        internal_width: diagram.quadrant_config.internal_border_width.unwrap_or(1.0),
        external_width: diagram.quadrant_config.external_border_width.unwrap_or(1.0),
    });

    if let Some(axis) = &diagram.x_axis {
        if let Some(label) = axis.categories.first() {
            items.push(LayoutedChartItem::DataLabel {
                x: left,
                y: if x_axis_top {
                    top - diagram.quadrant_config.x_axis_label_padding.unwrap_or(16.0)
                } else {
                    bottom + diagram.quadrant_config.x_axis_label_padding.unwrap_or(20.0)
                },
                text: label.clone(),
                font_size: diagram.quadrant_config.x_axis_label_font_size,
                color: diagram.quadrant_config.x_axis_text_fill.clone(),
            });
        }
        if let Some(label) = axis.categories.get(1) {
            items.push(LayoutedChartItem::DataLabel {
                x: right,
                y: if x_axis_top {
                    top - diagram.quadrant_config.x_axis_label_padding.unwrap_or(16.0)
                } else {
                    bottom + diagram.quadrant_config.x_axis_label_padding.unwrap_or(20.0)
                },
                text: label.clone(),
                font_size: diagram.quadrant_config.x_axis_label_font_size,
                color: diagram.quadrant_config.x_axis_text_fill.clone(),
            });
        }
    }
    if let Some(axis) = &diagram.y_axis {
        if let Some(label) = axis.categories.first() {
            items.push(LayoutedChartItem::DataLabel {
                x: if y_axis_right {
                    right + diagram.quadrant_config.y_axis_label_padding.unwrap_or(24.0)
                } else {
                    left - diagram.quadrant_config.y_axis_label_padding.unwrap_or(56.0)
                },
                y: bottom,
                text: label.clone(),
                font_size: diagram.quadrant_config.y_axis_label_font_size,
                color: diagram.quadrant_config.y_axis_text_fill.clone(),
            });
        }
        if let Some(label) = axis.categories.get(1) {
            items.push(LayoutedChartItem::DataLabel {
                x: if y_axis_right {
                    right + diagram.quadrant_config.y_axis_label_padding.unwrap_or(24.0)
                } else {
                    left - diagram.quadrant_config.y_axis_label_padding.unwrap_or(56.0)
                },
                y: top,
                text: label.clone(),
                font_size: diagram.quadrant_config.y_axis_label_font_size,
                color: diagram.quadrant_config.y_axis_text_fill.clone(),
            });
        }
    }

    for point in &diagram.quadrant_points {
        items.push(LayoutedChartItem::ScatterPoint {
            x: left + point.x * width,
            y: bottom - point.y * height,
            radius: point
                .radius
                .or(diagram.quadrant_config.point_radius)
                .unwrap_or(6.0),
            color: point
                .color
                .clone()
                .or_else(|| diagram.quadrant_config.point_fill.clone())
                .unwrap_or_else(|| "#2563eb".to_string()),
            stroke_color: point
                .stroke_color
                .clone()
                .unwrap_or_else(|| "#1e3a8a".to_string()),
            stroke_width: point.stroke_width.unwrap_or(1.5),
            label: point.label.clone(),
            label_font_size: diagram.quadrant_config.point_label_font_size,
            label_padding: diagram.quadrant_config.point_text_padding.unwrap_or(4.0),
            label_color: diagram
                .quadrant_config
                .point_text_fill
                .clone()
                .unwrap_or_else(|| "#1e293b".to_string()),
        });
    }

    LayoutedChartDiagram {
        width: cw,
        height: ch,
        accessibility_title: diagram.accessibility_title.clone(),
        accessibility_description: diagram.accessibility_description.clone(),
        title_box: None,
        items,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use diagram_ir::*;

    fn xy_diagram() -> ChartDiagram {
        ChartDiagram {
            title: Some("Test".into()),
            accessibility_title: None,
            accessibility_description: None,
            kind: ChartKind::Xy,
            show_data: false,
            x_axis: Some(Axis {
                kind: AxisKind::Categorical,
                title: None,
                categories: vec!["Jan".into(), "Feb".into(), "Mar".into()],
                min: 0.0,
                max: 0.0,
            }),
            y_axis: Some(Axis {
                kind: AxisKind::Numeric,
                title: None,
                categories: vec![],
                min: 0.0,
                max: 100.0,
            }),
            series: vec![
                ChartSeries {
                    kind: SeriesKind::Bar,
                    label: Some("A".into()),
                    data: vec![40.0, 60.0, 50.0]
                        .into_iter()
                        .map(|value| ChartDataPoint { value, label: None })
                        .collect(),
                },
                ChartSeries {
                    kind: SeriesKind::Line,
                    label: Some("B".into()),
                    data: vec![
                        ChartDataPoint {
                            value: 35.0,
                            label: Some("Start".into()),
                        },
                        ChartDataPoint {
                            value: 55.0,
                            label: None,
                        },
                        ChartDataPoint {
                            value: 48.0,
                            label: Some("End".into()),
                        },
                    ],
                },
            ],
            slices: vec![],
            sankey_nodes: vec![],
            flows: vec![],
            quadrant_labels: [None, None, None, None],
            quadrant_points: vec![],
            quadrant_config: QuadrantConfig::default(),
            xy_config: XyChartConfig::default(),
            orientation: ChartOrientation::Vertical,
        }
    }

    #[test]
    fn version_exists() {
        assert_eq!(crate::VERSION, "0.16.0");
    }

    #[test]
    fn xy_layout_produces_items() {
        let d = layout_chart_diagram(&xy_diagram(), 600.0, 400.0);
        assert!(d.width > 0.0);
        assert!(!d.items.is_empty());
    }

    #[test]
    fn xy_layout_lowers_axis_titles_to_labels() {
        let mut diagram = xy_diagram();
        diagram.x_axis.as_mut().unwrap().title = Some("Quarter".into());
        diagram.y_axis.as_mut().unwrap().title = Some("Revenue".into());
        let layout = layout_chart_diagram(&diagram, 600.0, 400.0);
        for expected in ["Quarter", "Revenue"] {
            assert!(layout.items.iter().any(|item| matches!(
                item,
                LayoutedChartItem::DataLabel { text, .. } if text == expected
            )));
        }
    }

    #[test]
    fn horizontal_xy_layout_maps_values_to_x_and_categories_to_y() {
        let mut diagram = xy_diagram();
        diagram.orientation = ChartOrientation::Horizontal;
        let layout = layout_chart_diagram(&diagram, 600.0, 400.0);
        let bars = layout
            .items
            .iter()
            .filter_map(|item| match item {
                LayoutedChartItem::Bar {
                    x,
                    y,
                    width,
                    height,
                    ..
                } => Some((*x, *y, *width, *height)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(bars.len(), 3);
        assert!(bars.iter().all(|(_, _, width, height)| width > height));
        assert!(bars[0].1 < bars[1].1 && bars[1].1 < bars[2].1);

        let points = layout
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedChartItem::LinePath { points, .. } => Some(points),
                _ => None,
            })
            .expect("horizontal line path");
        assert!(points[0].y < points[1].y && points[1].y < points[2].y);
        assert!(points[0].x < points[1].x);
        let labels = layout
            .items
            .iter()
            .filter(|item| matches!(item, LayoutedChartItem::PointLabel { .. }))
            .count();
        assert_eq!(labels, 2);
    }

    #[test]
    fn numeric_x_axis_evenly_distributes_points_in_both_orientations() {
        let mut diagram = xy_diagram();
        diagram.x_axis = Some(Axis {
            kind: AxisKind::Numeric,
            title: Some("Sample".into()),
            categories: vec![],
            min: 10.0,
            max: 20.0,
        });

        let vertical = layout_chart_diagram(&diagram, 600.0, 400.0);
        let vertical_points = vertical
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedChartItem::LinePath { points, .. } => Some(points),
                _ => None,
            })
            .unwrap();
        assert!(vertical_points[2].x - vertical_points[0].x > 300.0);
        assert!(vertical.items.iter().any(|item| {
            matches!(item, LayoutedChartItem::AxisTick { label, .. } if label == "10")
        }));

        diagram.orientation = ChartOrientation::Horizontal;
        let horizontal = layout_chart_diagram(&diagram, 600.0, 400.0);
        let horizontal_points = horizontal
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedChartItem::LinePath { points, .. } => Some(points),
                _ => None,
            })
            .unwrap();
        assert!(horizontal_points[2].y - horizontal_points[0].y > 200.0);
    }

    #[test]
    fn xy_core_config_controls_dimensions_title_and_bar_labels() {
        let mut diagram = xy_diagram();
        diagram.xy_config = XyChartConfig {
            width: Some(720.0),
            height: Some(440.0),
            title_font_size: Some(24.0),
            title_padding: Some(14.0),
            show_title: Some(false),
            show_data_label: Some(true),
            show_data_label_outside_bar: Some(true),
            data_label_color: Some("#123456".into()),
            x_axis: XyAxisConfig::default(),
            y_axis: XyAxisConfig::default(),
        };

        let layout = layout_chart_diagram(&diagram, 600.0, 400.0);
        assert_eq!((layout.width, layout.height), (720.0, 440.0));
        assert!(!layout.items.iter().any(
            |item| matches!(item, LayoutedChartItem::DataLabel { text, .. } if text == "Test")
        ));
        let bar_labels = layout
            .items
            .iter()
            .filter(|item| matches!(item, LayoutedChartItem::BarLabel { .. }))
            .count();
        assert_eq!(bar_labels, 3);
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedChartItem::BarLabel { text, color, .. }
                if text == "40" && color == "#123456"
        )));
    }

    #[test]
    fn xy_axis_config_controls_text_and_spines() {
        let mut diagram = xy_diagram();
        diagram.x_axis.as_mut().unwrap().title = Some("Hidden x title".into());
        diagram.y_axis.as_mut().unwrap().title = Some("Visible y title".into());
        diagram.xy_config.x_axis = XyAxisConfig {
            show_label: Some(false),
            show_title: Some(false),
            show_axis_line: Some(false),
            ..XyAxisConfig::default()
        };
        diagram.xy_config.y_axis = XyAxisConfig {
            label_font_size: Some(18.0),
            label_padding: Some(8.0),
            title_font_size: Some(19.0),
            title_padding: Some(9.0),
            axis_line_width: Some(5.0),
            ..XyAxisConfig::default()
        };

        let layout = layout_chart_diagram(&diagram, 600.0, 400.0);
        assert!(!layout.items.iter().any(|item| matches!(
            item,
            LayoutedChartItem::AxisTick {
                orientation: Orientation::Vertical,
                ..
            }
        )));
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedChartItem::AxisTick {
                orientation: Orientation::Horizontal,
                font_size,
                ..
            } if *font_size == 18.0
        )));
        assert!(!layout.items.iter().any(
            |item| matches!(item, LayoutedChartItem::DataLabel { text, .. } if text == "Hidden x title")
        ));
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedChartItem::DataLabel { text, font_size, .. }
                if text == "Visible y title" && *font_size == Some(19.0)
        )));
        assert_eq!(
            layout
                .items
                .iter()
                .filter(|item| matches!(item, LayoutedChartItem::AxisSpine { .. }))
                .count(),
            1
        );
        assert!(layout.items.iter().any(|item| matches!(
            item,
            LayoutedChartItem::AxisSpine { stroke_width, .. } if *stroke_width == 5.0
        )));
    }

    #[test]
    fn bar_count_matches_data_points() {
        let d = layout_chart_diagram(&xy_diagram(), 600.0, 400.0);
        let bars: Vec<_> = d
            .items
            .iter()
            .filter(|it| matches!(it, LayoutedChartItem::Bar { .. }))
            .collect();
        // 3 data points in the one bar series
        assert_eq!(bars.len(), 3);
    }

    #[test]
    fn pie_layout_produces_arcs() {
        let diagram = ChartDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            kind: ChartKind::Pie,
            show_data: true,
            x_axis: None,
            y_axis: None,
            series: vec![],
            slices: vec![
                PieSlice {
                    label: "A".into(),
                    value: 60.0,
                },
                PieSlice {
                    label: "B".into(),
                    value: 40.0,
                },
            ],
            sankey_nodes: vec![],
            flows: vec![],
            quadrant_labels: [None, None, None, None],
            quadrant_points: vec![],
            quadrant_config: QuadrantConfig::default(),
            xy_config: XyChartConfig::default(),
            orientation: ChartOrientation::Vertical,
        };
        let d = layout_chart_diagram(&diagram, 400.0, 400.0);
        let arcs: Vec<_> = d
            .items
            .iter()
            .filter(|it| matches!(it, LayoutedChartItem::PieArc { .. }))
            .collect();
        assert_eq!(arcs.len(), 2);
        assert!(d.items.iter().any(|item| matches!(
            item,
            LayoutedChartItem::PieArc { label, .. } if label == "60%"
        )));
        assert!(d.items.iter().any(|item| matches!(
            item,
            LayoutedChartItem::Legend { entries, .. }
                if entries[0].label == "A [60]" && entries[1].label == "B [40]"
        )));
    }

    #[test]
    fn sankey_layout_produces_bands() {
        let diagram = ChartDiagram {
            title: None,
            accessibility_title: None,
            accessibility_description: None,
            kind: ChartKind::Sankey,
            show_data: false,
            x_axis: None,
            y_axis: None,
            series: vec![],
            slices: vec![],
            sankey_nodes: vec![
                SankeyNode {
                    id: "a".into(),
                    label: None,
                },
                SankeyNode {
                    id: "b".into(),
                    label: None,
                },
                SankeyNode {
                    id: "c".into(),
                    label: None,
                },
            ],
            flows: vec![
                SankeyFlow {
                    source: "a".into(),
                    target: "b".into(),
                    weight: 10.0,
                },
                SankeyFlow {
                    source: "a".into(),
                    target: "c".into(),
                    weight: 5.0,
                },
            ],
            quadrant_labels: [None, None, None, None],
            quadrant_points: vec![],
            quadrant_config: QuadrantConfig::default(),
            xy_config: XyChartConfig::default(),
            orientation: ChartOrientation::Horizontal,
        };
        let d = layout_chart_diagram(&diagram, 600.0, 400.0);
        let bands: Vec<_> = d
            .items
            .iter()
            .filter(|it| matches!(it, LayoutedChartItem::SankeyBand { .. }))
            .collect();
        assert_eq!(bands.len(), 2);
        let nodes: Vec<_> = d
            .items
            .iter()
            .filter(|it| matches!(it, LayoutedChartItem::SankeyNode { .. }))
            .collect();
        assert_eq!(nodes.len(), 3);
        assert!(d.items.iter().any(|item| matches!(
            item,
            LayoutedChartItem::SankeyBand { from_x, to_x, .. } if to_x > from_x
        )));
    }

    #[test]
    fn quadrant_layout_produces_regions_and_points() {
        let diagram = ChartDiagram {
            title: Some("Portfolio".into()),
            accessibility_title: Some("Portfolio matrix".into()),
            accessibility_description: Some("Native renderer priorities".into()),
            kind: ChartKind::Quadrant,
            show_data: false,
            x_axis: Some(Axis {
                kind: AxisKind::Numeric,
                title: None,
                categories: vec!["Low".into(), "High".into()],
                min: 0.0,
                max: 1.0,
            }),
            y_axis: None,
            series: vec![],
            slices: vec![],
            sankey_nodes: vec![],
            flows: vec![],
            quadrant_labels: [Some("Invest".into()), None, None, None],
            quadrant_points: vec![QuadrantPoint {
                label: "Metal".into(),
                x: 0.75,
                y: 0.8,
                radius: Some(10.0),
                color: Some("#ff0000".into()),
                stroke_color: Some("#00ff00".into()),
                stroke_width: Some(3.0),
            }],
            quadrant_config: QuadrantConfig::default(),
            xy_config: XyChartConfig::default(),
            orientation: ChartOrientation::Vertical,
        };
        let layout = layout_chart_diagram(&diagram, 500.0, 500.0);
        assert_eq!(
            layout.accessibility_title.as_deref(),
            Some("Portfolio matrix")
        );

        assert_eq!(
            layout
                .items
                .iter()
                .filter(|item| matches!(item, LayoutedChartItem::QuadrantRegion { .. }))
                .count(),
            4
        );
        assert_eq!(
            layout
                .items
                .iter()
                .filter(|item| matches!(item, LayoutedChartItem::ScatterPoint { .. }))
                .count(),
            1
        );
        let point = layout
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedChartItem::ScatterPoint {
                    radius,
                    color,
                    stroke_width,
                    ..
                } => Some((*radius, color.as_str(), *stroke_width)),
                _ => None,
            })
            .unwrap();
        assert_eq!(point, (10.0, "#ff0000", 3.0));
    }

    #[test]
    fn quadrant_layout_applies_authored_config() {
        let mut diagram = xy_diagram();
        diagram.kind = ChartKind::Quadrant;
        diagram.series.clear();
        diagram.x_axis.as_mut().unwrap().categories = vec!["Low".into(), "High".into()];
        diagram.y_axis.as_mut().unwrap().categories = vec!["Bottom".into(), "Top".into()];
        diagram.quadrant_points.push(QuadrantPoint {
            label: "Metal".into(),
            x: 0.5,
            y: 0.5,
            radius: None,
            color: None,
            stroke_color: None,
            stroke_width: None,
        });
        diagram.quadrant_config = QuadrantConfig {
            chart_width: Some(720.0),
            chart_height: Some(540.0),
            x_axis_position: Some("top".into()),
            y_axis_position: Some("right".into()),
            point_radius: Some(11.0),
            quadrant_padding: Some(18.0),
            internal_border_width: Some(3.0),
            external_border_width: Some(5.0),
            title_font_size: Some(22.0),
            title_padding: Some(12.0),
            x_axis_label_font_size: Some(15.0),
            x_axis_label_padding: Some(21.0),
            y_axis_label_font_size: Some(16.0),
            y_axis_label_padding: Some(23.0),
            quadrant_label_font_size: Some(17.0),
            quadrant_text_top_padding: Some(19.0),
            point_label_font_size: Some(14.0),
            point_text_padding: Some(9.0),
            quadrant_fills: [Some("#111111".into()), None, None, None],
            quadrant_text_fills: [Some("#aaaaaa".into()), None, None, None],
            point_fill: Some("#123456".into()),
            point_text_fill: Some("#234567".into()),
            internal_border_stroke_fill: Some("#345678".into()),
            external_border_stroke_fill: Some("#456789".into()),
            ..QuadrantConfig::default()
        };

        let layout = layout_chart_diagram(&diagram, 400.0, 300.0);
        assert_eq!((layout.width, layout.height), (720.0, 540.0));
        let point_radius = layout.items.iter().find_map(|item| match item {
            LayoutedChartItem::ScatterPoint { radius, .. } => Some(*radius),
            _ => None,
        });
        assert_eq!(point_radius, Some(11.0));
        let point_text = layout.items.iter().find_map(|item| match item {
            LayoutedChartItem::ScatterPoint {
                label_font_size,
                label_padding,
                ..
            } => Some((*label_font_size, *label_padding)),
            _ => None,
        });
        assert_eq!(point_text, Some((Some(14.0), 9.0)));
        assert!(layout.items.iter().any(|item| matches!(item,
            LayoutedChartItem::ScatterPoint { color, label_color, .. }
                if color == "#123456" && label_color == "#234567"
        )));
        assert!(layout.items.iter().any(|item| matches!(item,
            LayoutedChartItem::QuadrantRegion { color, label_color, .. }
                if color == "#111111" && label_color == "#aaaaaa"
        )));
        let border = layout.items.iter().find_map(|item| match item {
            LayoutedChartItem::QuadrantBorder {
                x,
                internal_width,
                external_width,
                ..
            } => Some((*x, *internal_width, *external_width)),
            _ => None,
        });
        assert_eq!(border, Some((42.0, 3.0, 5.0)));
        let low = layout
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedChartItem::DataLabel { x, y, text, .. } if text == "Low" => Some((*x, *y)),
                _ => None,
            })
            .unwrap();
        let bottom = layout
            .items
            .iter()
            .find_map(|item| match item {
                LayoutedChartItem::DataLabel { x, y, text, .. } if text == "Bottom" => {
                    Some((*x, *y))
                }
                _ => None,
            })
            .unwrap();
        assert!(low.1 < 100.0, "top x-axis label should be above the plot");
        assert!(
            bottom.0 > 640.0,
            "right y-axis label should be right of the plot"
        );
    }
}
