//! Dual-fill edge loop reconstruction and stroke run assembly.

#![deny(unsafe_code)]

use crate::parser::ParsedShape;
use crate::records::ShapeEdge;

/// A reconstructed directed contour consisting of connected edges.
#[derive(Debug, Clone, PartialEq)]
pub struct DirectedContour {
    /// Ordered edge segments forming this contour.
    pub segments: Vec<ShapeEdge>,
    /// Whether the contour forms a closed loop (`last.end == first.start`).
    pub is_closed: bool,
}

impl DirectedContour {
    /// Number of edges in this contour.
    #[inline]
    pub fn contour_edge_count(&self) -> usize {
        self.segments.len()
    }
}

/// A collection of closed contours sharing a common fill style.
#[derive(Debug, Clone, PartialEq)]
pub struct FillGroup {
    /// 1-based index into the shape's fill style array.
    pub style_index: u32,
    /// Directed contours for this fill (exterior boundaries and interior island holes).
    pub contours: Vec<DirectedContour>,
}

impl FillGroup {
    /// Number of contours in this fill group.
    #[inline]
    pub fn fill_contour_count(&self) -> usize {
        self.contours.len()
    }
}

/// A contiguous sequence of stroke edges sharing a line style.
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeRun {
    /// 1-based index into the shape's line style array.
    pub line_style_index: u32,
    /// Ordered stroke edges.
    pub edges: Vec<ShapeEdge>,
    /// Whether the stroke loop closes back on itself.
    pub is_closed: bool,
}

impl StrokeRun {
    /// Number of edges in this stroke run.
    #[inline]
    pub fn stroke_edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// Reconstructs closed directed contours for a specific fill style index.
///
/// Uses dual-fill topology tags:
/// - `FillStyle1` (right side): edge traverses forward `start -> end`.
/// - `FillStyle0` (left side): edge traverses reversed `end -> start`.
pub fn reconstruct_fill_contours(
    edges: &[ShapeEdge],
    fill_style_index: u32,
) -> Vec<DirectedContour> {
    if fill_style_index == 0 {
        return Vec::new();
    }

    let mut segments = Vec::new();
    for edge in edges {
        if edge.fill_style1 == fill_style_index {
            segments.push(*edge);
        }
        if edge.fill_style0 == fill_style_index {
            segments.push(edge.reverse_edge_direction());
        }
    }

    if segments.is_empty() {
        return Vec::new();
    }

    let mut unused = vec![true; segments.len()];
    let mut contours = Vec::new();

    while let Some(start_idx) = unused.iter().position(|&u| u) {
        unused[start_idx] = false;
        let mut contour_segs = vec![segments[start_idx]];
        let loop_start = segments[start_idx].start;
        let mut tip = segments[start_idx].end;

        loop {
            if tip == loop_start {
                contours.push(DirectedContour {
                    segments: contour_segs,
                    is_closed: true,
                });
                break;
            }

            let next_match = unused
                .iter()
                .enumerate()
                .position(|(i, &u)| u && segments[i].start == tip);

            match next_match {
                Some(idx) => {
                    unused[idx] = false;
                    tip = segments[idx].end;
                    contour_segs.push(segments[idx]);
                }
                None => {
                    contours.push(DirectedContour {
                        segments: contour_segs,
                        is_closed: false,
                    });
                    break;
                }
            }
        }
    }

    contours
}

/// Reconstructs stroke runs for a specific line style index.
pub fn reconstruct_stroke_runs(edges: &[ShapeEdge], line_style_index: u32) -> Vec<StrokeRun> {
    if line_style_index == 0 {
        return Vec::new();
    }

    let line_edges: Vec<ShapeEdge> = edges
        .iter()
        .filter(|e| e.line_style == line_style_index)
        .copied()
        .collect();

    if line_edges.is_empty() {
        return Vec::new();
    }

    let mut unused = vec![true; line_edges.len()];
    let mut runs = Vec::new();

    while let Some(start_idx) = unused.iter().position(|&u| u) {
        unused[start_idx] = false;
        let mut run_edges = vec![line_edges[start_idx]];
        let mut tip = line_edges[start_idx].end;
        let start_pos = line_edges[start_idx].start;

        loop {
            if tip == start_pos {
                runs.push(StrokeRun {
                    line_style_index,
                    edges: run_edges,
                    is_closed: true,
                });
                break;
            }

            let next_match = unused
                .iter()
                .enumerate()
                .position(|(i, &u)| u && line_edges[i].start == tip);

            match next_match {
                Some(idx) => {
                    unused[idx] = false;
                    tip = line_edges[idx].end;
                    run_edges.push(line_edges[idx]);
                }
                None => {
                    runs.push(StrokeRun {
                        line_style_index,
                        edges: run_edges,
                        is_closed: false,
                    });
                    break;
                }
            }
        }
    }

    runs
}

/// Reconstructs all fill groups present in a parsed shape.
pub fn reconstruct_all_fills(shape: &ParsedShape) -> Vec<FillGroup> {
    let mut groups = Vec::new();
    for style_idx in 1..=shape.fill_styles.len() {
        let contours = reconstruct_fill_contours(&shape.edges, style_idx as u32);
        if !contours.is_empty() {
            groups.push(FillGroup {
                style_index: style_idx as u32,
                contours,
            });
        }
    }
    groups
}

/// Reconstructs all stroke runs present in a parsed shape.
pub fn reconstruct_all_strokes(shape: &ParsedShape) -> Vec<StrokeRun> {
    let mut runs = Vec::new();
    for line_idx in 1..=shape.line_styles.len() {
        let line_runs = reconstruct_stroke_runs(&shape.edges, line_idx as u32);
        runs.extend(line_runs);
    }
    runs
}
