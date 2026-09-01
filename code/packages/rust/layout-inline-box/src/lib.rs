//! Fragmented inline box edge policy over the shared Layout IR.

use std::collections::HashMap;

use layout_ir::{Edges, Ext, ExtValue, LayoutNode, PositionedNode};

pub const VERSION: &str = "0.1.0";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoxDecorationBreak {
    #[default]
    Slice,
    Clone,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InlineBoxStyle {
    pub margin: Edges,
    pub padding: Edges,
    pub border: Edges,
    pub decoration_break: BoxDecorationBreak,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FragmentEdges {
    pub margin: Edges,
    pub padding: Edges,
    pub border: Edges,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineBoxDiagnostic {
    pub key: String,
    pub message: String,
}

impl InlineBoxStyle {
    pub fn from_layout(node: &LayoutNode) -> Self {
        let decoration_break = match node.ext.get("inlineBox") {
            Some(ExtValue::Map(values)) => match string(values, "boxDecorationBreak") {
                Some("clone") => BoxDecorationBreak::Clone,
                _ => BoxDecorationBreak::Slice,
            },
            _ => BoxDecorationBreak::Slice,
        };
        Self {
            margin: sanitize_edges(node.margin.unwrap_or_default()),
            padding: sanitize_edges(node.padding.unwrap_or_default()),
            border: border_edges(&node.ext),
            decoration_break,
        }
    }

    pub fn fragment_edges(self, first: bool, last: bool) -> FragmentEdges {
        let clone = self.decoration_break == BoxDecorationBreak::Clone;
        FragmentEdges {
            margin: Edges {
                top: self.margin.top,
                right: if clone || last {
                    self.margin.right
                } else {
                    0.0
                },
                bottom: self.margin.bottom,
                left: if clone || first {
                    self.margin.left
                } else {
                    0.0
                },
            },
            padding: Edges {
                top: self.padding.top,
                right: if clone || last {
                    self.padding.right
                } else {
                    0.0
                },
                bottom: self.padding.bottom,
                left: if clone || first {
                    self.padding.left
                } else {
                    0.0
                },
            },
            border: Edges {
                top: self.border.top,
                right: if clone || last {
                    self.border.right
                } else {
                    0.0
                },
                bottom: self.border.bottom,
                left: if clone || first {
                    self.border.left
                } else {
                    0.0
                },
            },
        }
    }

    pub fn start_reservation(self, first: bool) -> f64 {
        let edges = self.fragment_edges(first, false);
        edges.margin.left + edges.border.left + edges.padding.left
    }

    pub fn end_reservation(self, last: bool) -> f64 {
        let edges = self.fragment_edges(false, last);
        edges.padding.right + edges.border.right + edges.margin.right
    }

    pub fn diagnostics(node: &LayoutNode) -> Vec<InlineBoxDiagnostic> {
        let Some(ExtValue::Map(values)) = node.ext.get("inlineBox") else {
            return Vec::new();
        };
        match string(values, "boxDecorationBreak") {
            Some(value) if !matches!(value, "slice" | "clone") => vec![InlineBoxDiagnostic {
                key: "boxDecorationBreak".into(),
                message: format!("unsupported box-decoration-break `{value}`; using slice"),
            }],
            _ => Vec::new(),
        }
    }
}

pub fn inline_box_ext(decoration_break: BoxDecorationBreak) -> ExtValue {
    ExtValue::Map(HashMap::from([(
        "boxDecorationBreak".into(),
        ExtValue::Str(
            match decoration_break {
                BoxDecorationBreak::Slice => "slice",
                BoxDecorationBreak::Clone => "clone",
            }
            .into(),
        ),
    )]))
}

/// Expand a tight content fragment to its decorated border box.
///
/// Margins remain outside the returned hit/paint geometry, but are supplied in
/// `FragmentEdges` so inline layout can reserve them in the line advance.
pub fn decorate_fragment(node: &mut PositionedNode, edges: FragmentEdges) {
    let left = edges.border.left + edges.padding.left;
    let right = edges.padding.right + edges.border.right;
    let top = edges.border.top + edges.padding.top;
    let bottom = edges.padding.bottom + edges.border.bottom;
    node.x -= left;
    node.y -= top;
    node.width += left + right;
    node.height += top + bottom;
    for child in &mut node.children {
        child.x += left;
        child.y += top;
    }
    suppress_fragment_borders(&mut node.ext, edges.border);
    node.ext.insert(
        "inlineFragment".into(),
        ExtValue::Map(HashMap::from([
            ("marginLeft".into(), ExtValue::Float(edges.margin.left)),
            ("marginRight".into(), ExtValue::Float(edges.margin.right)),
            ("paddingLeft".into(), ExtValue::Float(edges.padding.left)),
            ("paddingRight".into(), ExtValue::Float(edges.padding.right)),
        ])),
    );
}

fn suppress_fragment_borders(ext: &mut Ext, border: Edges) {
    let Some(ExtValue::Map(paint)) = ext.get_mut("paint") else {
        return;
    };
    for (key, width) in [
        ("borderTopWidth", border.top),
        ("borderRightWidth", border.right),
        ("borderBottomWidth", border.bottom),
        ("borderLeftWidth", border.left),
    ] {
        if width == 0.0 {
            paint.remove(key);
        } else {
            paint.insert(key.into(), ExtValue::Float(width));
        }
    }
}

fn border_edges(ext: &Ext) -> Edges {
    let Some(ExtValue::Map(paint)) = ext.get("paint") else {
        return Edges::default();
    };
    Edges {
        top: number(paint, "borderTopWidth"),
        right: number(paint, "borderRightWidth"),
        bottom: number(paint, "borderBottomWidth"),
        left: number(paint, "borderLeftWidth"),
    }
}

fn number(values: &HashMap<String, ExtValue>, key: &str) -> f64 {
    match values.get(key) {
        Some(ExtValue::Float(value)) => finite_non_negative(*value),
        Some(ExtValue::Int(value)) => finite_non_negative(*value as f64),
        _ => 0.0,
    }
}

fn string<'a>(values: &'a HashMap<String, ExtValue>, key: &str) -> Option<&'a str> {
    let ExtValue::Str(value) = values.get(key)? else {
        return None;
    };
    Some(value)
}

fn sanitize_edges(edges: Edges) -> Edges {
    Edges {
        top: finite_non_negative(edges.top),
        right: finite_non_negative(edges.right),
        bottom: finite_non_negative(edges.bottom),
        left: finite_non_negative(edges.left),
    }
}

fn finite_non_negative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(mode: BoxDecorationBreak) -> InlineBoxStyle {
        InlineBoxStyle {
            margin: Edges {
                top: 1.0,
                right: 2.0,
                bottom: 3.0,
                left: 4.0,
            },
            padding: Edges {
                top: 5.0,
                right: 6.0,
                bottom: 7.0,
                left: 8.0,
            },
            border: Edges {
                top: 1.0,
                right: 2.0,
                bottom: 3.0,
                left: 4.0,
            },
            decoration_break: mode,
        }
    }

    #[test]
    fn slice_continues_only_outer_inline_edges() {
        let style = style(BoxDecorationBreak::Slice);
        let middle = style.fragment_edges(false, false);
        assert_eq!(
            (middle.margin.left, middle.padding.left, middle.border.left),
            (0.0, 0.0, 0.0)
        );
        assert_eq!((middle.padding.top, middle.border.bottom), (5.0, 3.0));
        assert_eq!(style.start_reservation(true), 16.0);
        assert_eq!(style.end_reservation(true), 10.0);
    }

    #[test]
    fn clone_repeats_every_fragment_edge() {
        let style = style(BoxDecorationBreak::Clone);
        let middle = style.fragment_edges(false, false);
        assert_eq!(
            (
                middle.margin.left,
                middle.padding.right,
                middle.border.right
            ),
            (4.0, 6.0, 2.0)
        );
        assert_eq!(style.start_reservation(false), 16.0);
        assert_eq!(style.end_reservation(false), 10.0);
    }

    #[test]
    fn decorated_geometry_excludes_margin_but_shifts_children() {
        let mut node = PositionedNode {
            x: 20.0,
            y: 10.0,
            width: 30.0,
            height: 12.0,
            id: Some("link".into()),
            content: None,
            children: vec![PositionedNode {
                x: 0.0,
                y: 0.0,
                width: 30.0,
                height: 12.0,
                id: None,
                content: None,
                children: Vec::new(),
                ext: Ext::new(),
            }],
            ext: Ext::new(),
        };
        let edges = style(BoxDecorationBreak::Slice).fragment_edges(true, true);
        decorate_fragment(&mut node, edges);
        assert_eq!(
            (node.x, node.y, node.width, node.height),
            (8.0, 4.0, 50.0, 28.0)
        );
        assert_eq!((node.children[0].x, node.children[0].y), (12.0, 6.0));
    }
}
