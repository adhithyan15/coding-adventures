//! Reusable background-layer and corner geometry contracts over Layout IR.

use std::collections::HashMap;

use layout_ir::{Ext, ExtValue, LayoutNode, PositionedNode};

pub const VERSION: &str = "0.1.0";

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LengthPercent {
    pub factor: f64,
    pub offset: f64,
}

impl LengthPercent {
    pub const fn length(offset: f64) -> Self {
        Self {
            factor: 0.0,
            offset,
        }
    }
    pub const fn percent(factor: f64) -> Self {
        Self {
            factor,
            offset: 0.0,
        }
    }
    pub fn resolve(self, extent: f64) -> f64 {
        self.factor * extent + self.offset
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundBox {
    Border,
    Padding,
    Content,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundRepeat {
    Repeat,
    NoRepeat,
    Space,
    Round,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BackgroundSize {
    Auto,
    Cover,
    Contain,
    Explicit {
        width: Option<LengthPercent>,
        height: Option<LengthPercent>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackgroundColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GradientStop {
    pub color: BackgroundColor,
    pub offset: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BackgroundSource {
    Image(String),
    LinearGradient {
        angle_degrees: f64,
        stops: Vec<GradientStop>,
    },
    RadialGradient {
        stops: Vec<GradientStop>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct BackgroundLayer {
    pub source: BackgroundSource,
    pub position_x: LengthPercent,
    pub position_y: LengthPercent,
    pub size: BackgroundSize,
    pub repeat_x: BackgroundRepeat,
    pub repeat_y: BackgroundRepeat,
    pub origin: BackgroundBox,
    pub clip: BackgroundBox,
}

impl BackgroundLayer {
    pub fn new(source: BackgroundSource) -> Self {
        Self {
            source,
            position_x: LengthPercent::percent(0.0),
            position_y: LengthPercent::percent(0.0),
            size: BackgroundSize::Auto,
            repeat_x: BackgroundRepeat::Repeat,
            repeat_y: BackgroundRepeat::Repeat,
            origin: BackgroundBox::Padding,
            clip: BackgroundBox::Border,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerRadius {
    pub x: LengthPercent,
    pub y: LengthPercent,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoxEdges {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BackgroundStyle {
    pub layers: Vec<BackgroundLayer>,
    pub color_clip: BackgroundBox,
    pub corners: [CornerRadius; 4],
    pub border: BoxEdges,
    pub padding: BoxEdges,
}

impl Default for BackgroundStyle {
    fn default() -> Self {
        Self {
            layers: Vec::new(),
            color_clip: BackgroundBox::Border,
            corners: [CornerRadius::default(); 4],
            border: BoxEdges::default(),
            padding: BoxEdges::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl BackgroundStyle {
    pub fn from_layout(node: &LayoutNode) -> Self {
        Self::from_ext(&node.ext)
    }
    pub fn from_positioned(node: &PositionedNode) -> Self {
        Self::from_ext(&node.ext)
    }

    pub fn from_ext(ext: &Ext) -> Self {
        let Some(ExtValue::Map(values)) = ext.get("backgrounds") else {
            return Self::default();
        };
        let layers = match values.get("layers") {
            Some(ExtValue::List(values)) => values.iter().filter_map(layer_from_ext).collect(),
            _ => Vec::new(),
        };
        let mut corners = [CornerRadius::default(); 4];
        if let Some(ExtValue::List(values)) = values.get("corners") {
            for (slot, value) in corners.iter_mut().zip(values) {
                if let Some(radius) = corner_from_ext(value) {
                    *slot = radius;
                }
            }
        }
        Self {
            layers,
            color_clip: box_kind(values.get("colorClip"), BackgroundBox::Border),
            corners,
            border: edges_from_ext(values.get("border")).unwrap_or_default(),
            padding: edges_from_ext(values.get("padding")).unwrap_or_default(),
        }
    }

    pub fn to_ext(&self) -> ExtValue {
        ExtValue::Map(HashMap::from([
            (
                "layers".into(),
                ExtValue::List(self.layers.iter().map(layer_to_ext).collect()),
            ),
            (
                "corners".into(),
                ExtValue::List(self.corners.iter().map(corner_to_ext).collect()),
            ),
            (
                "colorClip".into(),
                ExtValue::Str(box_name(self.color_clip).into()),
            ),
            ("border".into(), edges_to_ext(self.border)),
            ("padding".into(), edges_to_ext(self.padding)),
        ]))
    }

    pub fn painting_box(&self, kind: BackgroundBox, border_box: Rect) -> Rect {
        let border = inset(border_box, self.border);
        match kind {
            BackgroundBox::Border => border_box,
            BackgroundBox::Padding => border,
            BackgroundBox::Content => inset(border, self.padding),
        }
    }

    pub fn resolved_corners(&self, rect: Rect) -> [(f64, f64); 4] {
        let mut out = self.corners.map(|corner| {
            (
                corner.x.resolve(rect.width).max(0.0),
                corner.y.resolve(rect.height).max(0.0),
            )
        });
        let top = out[0].0 + out[1].0;
        let bottom = out[3].0 + out[2].0;
        let left = out[0].1 + out[3].1;
        let right = out[1].1 + out[2].1;
        let scale = 1.0_f64
            .min(if top > 0.0 { rect.width / top } else { 1.0 })
            .min(if bottom > 0.0 {
                rect.width / bottom
            } else {
                1.0
            })
            .min(if left > 0.0 { rect.height / left } else { 1.0 })
            .min(if right > 0.0 {
                rect.height / right
            } else {
                1.0
            });
        for radius in &mut out {
            radius.0 *= scale;
            radius.1 *= scale;
        }
        out
    }

    pub fn resolved_corners_for_box(
        &self,
        kind: BackgroundBox,
        border_box: Rect,
    ) -> [(f64, f64); 4] {
        let mut corners = self.resolved_corners(border_box);
        let inset = match kind {
            BackgroundBox::Border => BoxEdges::default(),
            BackgroundBox::Padding => self.border,
            BackgroundBox::Content => BoxEdges {
                top: self.border.top + self.padding.top,
                right: self.border.right + self.padding.right,
                bottom: self.border.bottom + self.padding.bottom,
                left: self.border.left + self.padding.left,
            },
        };
        for (index, (horizontal, vertical)) in [
            (inset.left, inset.top),
            (inset.right, inset.top),
            (inset.right, inset.bottom),
            (inset.left, inset.bottom),
        ]
        .into_iter()
        .enumerate()
        {
            corners[index].0 = (corners[index].0 - horizontal).max(0.0);
            corners[index].1 = (corners[index].1 - vertical).max(0.0);
        }
        normalize_corners(corners, self.painting_box(kind, border_box))
    }
}

pub fn diagnostics(ext: &Ext) -> Vec<String> {
    let Some(ExtValue::Map(values)) = ext.get("backgrounds") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if !matches!(values.get("layers"), None | Some(ExtValue::List(_))) {
        out.push("background layers must be a list".into());
    }
    if !matches!(values.get("corners"), None | Some(ExtValue::List(_))) {
        out.push("background corners must be a list".into());
    }
    out
}

fn inset(rect: Rect, edges: BoxEdges) -> Rect {
    Rect {
        x: rect.x + edges.left,
        y: rect.y + edges.top,
        width: (rect.width - edges.left - edges.right).max(0.0),
        height: (rect.height - edges.top - edges.bottom).max(0.0),
    }
}

fn normalize_corners(mut corners: [(f64, f64); 4], rect: Rect) -> [(f64, f64); 4] {
    let top = corners[0].0 + corners[1].0;
    let bottom = corners[3].0 + corners[2].0;
    let left = corners[0].1 + corners[3].1;
    let right = corners[1].1 + corners[2].1;
    let scale = 1.0_f64
        .min(if top > 0.0 { rect.width / top } else { 1.0 })
        .min(if bottom > 0.0 {
            rect.width / bottom
        } else {
            1.0
        })
        .min(if left > 0.0 { rect.height / left } else { 1.0 })
        .min(if right > 0.0 {
            rect.height / right
        } else {
            1.0
        });
    for radius in &mut corners {
        radius.0 *= scale;
        radius.1 *= scale;
    }
    corners
}

fn lp_to_ext(value: LengthPercent) -> ExtValue {
    ExtValue::Map(HashMap::from([
        ("factor".into(), ExtValue::Float(value.factor)),
        ("offset".into(), ExtValue::Float(value.offset)),
    ]))
}
fn lp_from_ext(value: &ExtValue) -> Option<LengthPercent> {
    let ExtValue::Map(values) = value else {
        return None;
    };
    Some(LengthPercent {
        factor: number(values.get("factor"))?,
        offset: number(values.get("offset"))?,
    })
}
fn color_to_ext(value: BackgroundColor) -> ExtValue {
    ExtValue::List(
        [value.r, value.g, value.b, value.a]
            .into_iter()
            .map(|v| ExtValue::Int(i64::from(v)))
            .collect(),
    )
}
fn color_from_ext(value: &ExtValue) -> Option<BackgroundColor> {
    let ExtValue::List(values) = value else {
        return None;
    };
    let bytes = values
        .iter()
        .map(|v| number(Some(v)).map(|n| n.clamp(0.0, 255.0) as u8))
        .collect::<Option<Vec<_>>>()?;
    Some(BackgroundColor {
        r: *bytes.first()?,
        g: *bytes.get(1)?,
        b: *bytes.get(2)?,
        a: *bytes.get(3)?,
    })
}
fn stop_to_ext(stop: &GradientStop) -> ExtValue {
    let mut map = HashMap::from([("color".into(), color_to_ext(stop.color))]);
    if let Some(offset) = stop.offset {
        map.insert("offset".into(), ExtValue::Float(offset));
    }
    ExtValue::Map(map)
}
fn stop_from_ext(value: &ExtValue) -> Option<GradientStop> {
    let ExtValue::Map(values) = value else {
        return None;
    };
    Some(GradientStop {
        color: color_from_ext(values.get("color")?)?,
        offset: values.get("offset").and_then(|v| number(Some(v))),
    })
}
fn layer_to_ext(layer: &BackgroundLayer) -> ExtValue {
    let mut map = HashMap::from([
        ("positionX".into(), lp_to_ext(layer.position_x)),
        ("positionY".into(), lp_to_ext(layer.position_y)),
        (
            "repeatX".into(),
            ExtValue::Str(repeat_name(layer.repeat_x).into()),
        ),
        (
            "repeatY".into(),
            ExtValue::Str(repeat_name(layer.repeat_y).into()),
        ),
        (
            "origin".into(),
            ExtValue::Str(box_name(layer.origin).into()),
        ),
        ("clip".into(), ExtValue::Str(box_name(layer.clip).into())),
        ("size".into(), size_to_ext(layer.size)),
    ]);
    match &layer.source {
        BackgroundSource::Image(uri) => {
            map.insert("kind".into(), ExtValue::Str("image".into()));
            map.insert("uri".into(), ExtValue::Str(uri.clone()));
        }
        BackgroundSource::LinearGradient {
            angle_degrees,
            stops,
        } => {
            map.insert("kind".into(), ExtValue::Str("linear".into()));
            map.insert("angle".into(), ExtValue::Float(*angle_degrees));
            map.insert(
                "stops".into(),
                ExtValue::List(stops.iter().map(stop_to_ext).collect()),
            );
        }
        BackgroundSource::RadialGradient { stops } => {
            map.insert("kind".into(), ExtValue::Str("radial".into()));
            map.insert(
                "stops".into(),
                ExtValue::List(stops.iter().map(stop_to_ext).collect()),
            );
        }
    }
    ExtValue::Map(map)
}
fn layer_from_ext(value: &ExtValue) -> Option<BackgroundLayer> {
    let ExtValue::Map(v) = value else {
        return None;
    };
    let stops = || match v.get("stops") {
        Some(ExtValue::List(s)) => s.iter().filter_map(stop_from_ext).collect(),
        _ => Vec::new(),
    };
    let source = match string(v.get("kind"))? {
        "image" => BackgroundSource::Image(string(v.get("uri"))?.into()),
        "linear" => BackgroundSource::LinearGradient {
            angle_degrees: number(v.get("angle")).unwrap_or(180.0),
            stops: stops(),
        },
        "radial" => BackgroundSource::RadialGradient { stops: stops() },
        _ => return None,
    };
    Some(BackgroundLayer {
        source,
        position_x: v.get("positionX").and_then(lp_from_ext).unwrap_or_default(),
        position_y: v.get("positionY").and_then(lp_from_ext).unwrap_or_default(),
        size: v
            .get("size")
            .and_then(size_from_ext)
            .unwrap_or(BackgroundSize::Auto),
        repeat_x: repeat(v.get("repeatX")),
        repeat_y: repeat(v.get("repeatY")),
        origin: box_kind(v.get("origin"), BackgroundBox::Padding),
        clip: box_kind(v.get("clip"), BackgroundBox::Border),
    })
}
fn corner_to_ext(c: &CornerRadius) -> ExtValue {
    ExtValue::List(vec![lp_to_ext(c.x), lp_to_ext(c.y)])
}
fn corner_from_ext(v: &ExtValue) -> Option<CornerRadius> {
    let ExtValue::List(v) = v else { return None };
    Some(CornerRadius {
        x: lp_from_ext(v.first()?)?,
        y: lp_from_ext(v.get(1)?)?,
    })
}
fn edges_to_ext(e: BoxEdges) -> ExtValue {
    ExtValue::List(
        [e.top, e.right, e.bottom, e.left]
            .into_iter()
            .map(ExtValue::Float)
            .collect(),
    )
}
fn edges_from_ext(v: Option<&ExtValue>) -> Option<BoxEdges> {
    let ExtValue::List(v) = v? else { return None };
    Some(BoxEdges {
        top: number(v.first())?,
        right: number(v.get(1))?,
        bottom: number(v.get(2))?,
        left: number(v.get(3))?,
    })
}
fn size_to_ext(s: BackgroundSize) -> ExtValue {
    match s {
        BackgroundSize::Auto => ExtValue::Str("auto".into()),
        BackgroundSize::Cover => ExtValue::Str("cover".into()),
        BackgroundSize::Contain => ExtValue::Str("contain".into()),
        BackgroundSize::Explicit { width, height } => ExtValue::List(vec![
            width
                .map(lp_to_ext)
                .unwrap_or_else(|| ExtValue::Str("auto".into())),
            height
                .map(lp_to_ext)
                .unwrap_or_else(|| ExtValue::Str("auto".into())),
        ]),
    }
}
fn size_from_ext(v: &ExtValue) -> Option<BackgroundSize> {
    match v {
        ExtValue::Str(s) if s == "auto" => Some(BackgroundSize::Auto),
        ExtValue::Str(s) if s == "cover" => Some(BackgroundSize::Cover),
        ExtValue::Str(s) if s == "contain" => Some(BackgroundSize::Contain),
        ExtValue::List(v) => Some(BackgroundSize::Explicit {
            width: v.first().and_then(lp_from_ext),
            height: v.get(1).and_then(lp_from_ext),
        }),
        _ => None,
    }
}
fn repeat(v: Option<&ExtValue>) -> BackgroundRepeat {
    match string(v) {
        Some("no-repeat") => BackgroundRepeat::NoRepeat,
        Some("space") => BackgroundRepeat::Space,
        Some("round") => BackgroundRepeat::Round,
        _ => BackgroundRepeat::Repeat,
    }
}
fn repeat_name(v: BackgroundRepeat) -> &'static str {
    match v {
        BackgroundRepeat::Repeat => "repeat",
        BackgroundRepeat::NoRepeat => "no-repeat",
        BackgroundRepeat::Space => "space",
        BackgroundRepeat::Round => "round",
    }
}
fn box_kind(v: Option<&ExtValue>, default: BackgroundBox) -> BackgroundBox {
    match string(v) {
        Some("border-box") => BackgroundBox::Border,
        Some("content-box") => BackgroundBox::Content,
        Some("padding-box") => BackgroundBox::Padding,
        _ => default,
    }
}
fn box_name(v: BackgroundBox) -> &'static str {
    match v {
        BackgroundBox::Border => "border-box",
        BackgroundBox::Padding => "padding-box",
        BackgroundBox::Content => "content-box",
    }
}
fn number(v: Option<&ExtValue>) -> Option<f64> {
    match v? {
        ExtValue::Float(v) if v.is_finite() => Some(*v),
        ExtValue::Int(v) => Some(*v as f64),
        _ => None,
    }
}
fn string(v: Option<&ExtValue>) -> Option<&str> {
    match v? {
        ExtValue::Str(v) => Some(v),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_and_radius_normalization() {
        let style = BackgroundStyle {
            layers: vec![BackgroundLayer::new(BackgroundSource::LinearGradient {
                angle_degrees: 90.0,
                stops: vec![GradientStop {
                    color: BackgroundColor {
                        r: 1,
                        g: 2,
                        b: 3,
                        a: 255,
                    },
                    offset: Some(0.0),
                }],
            })],
            corners: [CornerRadius {
                x: LengthPercent::length(80.0),
                y: LengthPercent::length(30.0),
            }; 4],
            ..BackgroundStyle::default()
        };
        let mut node = LayoutNode::container(vec![]);
        node.ext.insert("backgrounds".into(), style.to_ext());
        assert_eq!(BackgroundStyle::from_layout(&node), style);
        assert_eq!(
            style.resolved_corners(Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 40.0
            })[0],
            (50.0, 18.75)
        );
    }
    #[test]
    fn painting_boxes_resolve_edges() {
        let style = BackgroundStyle {
            border: BoxEdges {
                top: 2.0,
                right: 2.0,
                bottom: 2.0,
                left: 2.0,
            },
            padding: BoxEdges {
                top: 3.0,
                right: 3.0,
                bottom: 3.0,
                left: 3.0,
            },
            corners: [CornerRadius {
                x: LengthPercent::length(8.0),
                y: LengthPercent::length(6.0),
            }; 4],
            ..Default::default()
        };
        assert_eq!(
            style.painting_box(
                BackgroundBox::Content,
                Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 30.0,
                    height: 20.0
                }
            ),
            Rect {
                x: 5.0,
                y: 5.0,
                width: 20.0,
                height: 10.0
            }
        );
        assert_eq!(
            style.resolved_corners_for_box(
                BackgroundBox::Content,
                Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 30.0,
                    height: 20.0,
                },
            )[0],
            (3.0, 1.0)
        );
    }
}
