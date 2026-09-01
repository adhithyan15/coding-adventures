//! Reusable intrinsic sizing and object-fit geometry for replaced boxes.

use std::collections::HashMap;

use layout_ir::{Ext, ExtValue, ImageFit, LayoutNode, PositionedNode, SizeValue};

pub const VERSION: &str = "0.1.0";
pub const DEFAULT_WIDTH: f64 = 300.0;
pub const DEFAULT_HEIGHT: f64 = 150.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct IntrinsicSize {
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub aspect_ratio: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReplacedSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ObjectFitRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub clips: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplacedDiagnostic {
    pub key: String,
    pub message: String,
}

impl IntrinsicSize {
    pub fn from_layout(node: &LayoutNode) -> Self {
        Self::from_ext(&node.ext)
    }

    pub fn from_positioned(node: &PositionedNode) -> Self {
        Self::from_ext(&node.ext)
    }

    fn from_ext(ext: &Ext) -> Self {
        let Some(ExtValue::Map(values)) = ext.get("replaced") else {
            return Self::default();
        };
        let width = positive_number(values, "intrinsicWidth");
        let height = positive_number(values, "intrinsicHeight");
        let aspect_ratio = positive_number(values, "aspectRatio")
            .or_else(|| width.zip(height).map(|(width, height)| width / height));
        Self {
            width,
            height,
            aspect_ratio,
        }
    }

    pub fn diagnostics(node: &LayoutNode) -> Vec<ReplacedDiagnostic> {
        let Some(ExtValue::Map(values)) = node.ext.get("replaced") else {
            return Vec::new();
        };
        ["intrinsicWidth", "intrinsicHeight", "aspectRatio"]
            .into_iter()
            .filter_map(|key| {
                values.get(key).and_then(|value| {
                    number_value(value)
                        .filter(|value| value.is_finite() && *value > 0.0)
                        .is_none()
                        .then(|| ReplacedDiagnostic {
                            key: key.into(),
                            message: format!("invalid replaced {key}; ignoring value"),
                        })
                })
            })
            .collect()
    }
}

pub fn replaced_ext(
    intrinsic_width: Option<f64>,
    intrinsic_height: Option<f64>,
    aspect_ratio: Option<f64>,
) -> ExtValue {
    let mut values = HashMap::new();
    if let Some(width) = valid_positive(intrinsic_width) {
        values.insert("intrinsicWidth".into(), ExtValue::Float(width));
    }
    if let Some(height) = valid_positive(intrinsic_height) {
        values.insert("intrinsicHeight".into(), ExtValue::Float(height));
    }
    if let Some(ratio) = valid_positive(aspect_ratio) {
        values.insert("aspectRatio".into(), ExtValue::Float(ratio));
    }
    ExtValue::Map(values)
}

pub fn resolve_replaced_size(
    node: &LayoutNode,
    available_width: f64,
    available_height: f64,
) -> ReplacedSize {
    let available_width = finite_non_negative(available_width);
    let available_height = finite_non_negative(available_height);
    let intrinsic = IntrinsicSize::from_layout(node);
    let ratio = intrinsic
        .aspect_ratio
        .or_else(|| {
            intrinsic
                .width
                .zip(intrinsic.height)
                .map(|(width, height)| width / height)
        })
        .or(Some(DEFAULT_WIDTH / DEFAULT_HEIGHT));
    let specified_width = resolve_dimension(node.width, available_width);
    let specified_height = resolve_dimension(node.height, available_height);

    let (mut width, mut height) = match (specified_width, specified_height) {
        (Some(width), Some(height)) => (width, height),
        (Some(width), None) => (
            width,
            ratio
                .map(|ratio| width / ratio)
                .or(intrinsic.height)
                .unwrap_or(DEFAULT_HEIGHT),
        ),
        (None, Some(height)) => (
            ratio
                .map(|ratio| height * ratio)
                .or(intrinsic.width)
                .unwrap_or(DEFAULT_WIDTH),
            height,
        ),
        (None, None) => intrinsic_dimensions(intrinsic),
    };

    width = clamp(width, node.min_width, node.max_width).min(available_width);
    if specified_height.is_none() {
        if let Some(ratio) = ratio {
            height = width / ratio;
        }
    }
    height = clamp(height, node.min_height, node.max_height);
    if specified_width.is_none() {
        if let Some(ratio) = ratio {
            width = clamp(height * ratio, node.min_width, node.max_width).min(available_width);
        }
    }
    ReplacedSize {
        width: finite_non_negative(width),
        height: finite_non_negative(height),
    }
}

pub fn intrinsic_inline_size(node: &LayoutNode) -> f64 {
    let intrinsic = IntrinsicSize::from_layout(node);
    resolve_dimension(node.width, f64::MAX)
        .or(intrinsic.width)
        .or_else(|| {
            resolve_dimension(node.height, f64::MAX)
                .zip(intrinsic.aspect_ratio)
                .map(|(height, ratio)| height * ratio)
        })
        .unwrap_or(DEFAULT_WIDTH)
}

pub fn object_fit_rect(
    fit: ImageFit,
    box_width: f64,
    box_height: f64,
    intrinsic: IntrinsicSize,
) -> ObjectFitRect {
    let box_width = finite_non_negative(box_width);
    let box_height = finite_non_negative(box_height);
    if intrinsic.width.is_none() && intrinsic.height.is_none() && intrinsic.aspect_ratio.is_none() {
        return ObjectFitRect {
            width: box_width,
            height: box_height,
            ..ObjectFitRect::default()
        };
    }
    let (source_width, source_height) = intrinsic_dimensions(intrinsic);
    if fit == ImageFit::Fill || source_width == 0.0 || source_height == 0.0 {
        return ObjectFitRect {
            width: box_width,
            height: box_height,
            ..ObjectFitRect::default()
        };
    }
    let scale = match fit {
        ImageFit::Contain => (box_width / source_width).min(box_height / source_height),
        ImageFit::Cover => (box_width / source_width).max(box_height / source_height),
        ImageFit::None => 1.0,
        ImageFit::Fill => unreachable!(),
    };
    let width = source_width * scale;
    let height = source_height * scale;
    ObjectFitRect {
        x: (box_width - width) / 2.0,
        y: (box_height - height) / 2.0,
        width,
        height,
        clips: width > box_width || height > box_height,
    }
}

fn intrinsic_dimensions(intrinsic: IntrinsicSize) -> (f64, f64) {
    match (intrinsic.width, intrinsic.height, intrinsic.aspect_ratio) {
        (Some(width), Some(height), _) => (width, height),
        (Some(width), None, Some(ratio)) => (width, width / ratio),
        (None, Some(height), Some(ratio)) => (height * ratio, height),
        (Some(width), None, None) => (width, DEFAULT_HEIGHT),
        (None, Some(height), None) => (DEFAULT_WIDTH, height),
        (None, None, Some(ratio)) => (DEFAULT_HEIGHT * ratio, DEFAULT_HEIGHT),
        (None, None, None) => (DEFAULT_WIDTH, DEFAULT_HEIGHT),
    }
}

fn resolve_dimension(value: Option<SizeValue>, available: f64) -> Option<f64> {
    match value {
        Some(SizeValue::Fixed(value)) => Some(finite_non_negative(value)),
        Some(SizeValue::Percent(fraction)) if available.is_finite() => {
            Some(available * fraction.clamp(0.0, 1.0))
        }
        _ => None,
    }
}

fn clamp(value: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    let value = finite_non_negative(value).max(valid_non_negative(min).unwrap_or(0.0));
    valid_non_negative(max).map_or(value, |max| value.min(max))
}

fn positive_number(values: &HashMap<String, ExtValue>, key: &str) -> Option<f64> {
    values
        .get(key)
        .and_then(number_value)
        .filter(|value| value.is_finite() && *value > 0.0)
}

fn number_value(value: &ExtValue) -> Option<f64> {
    match value {
        ExtValue::Float(value) => Some(*value),
        ExtValue::Int(value) => Some(*value as f64),
        _ => None,
    }
}

fn valid_positive(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

fn valid_non_negative(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value >= 0.0)
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
    use layout_ir::ImageContent;

    fn image(width: f64, height: f64) -> LayoutNode {
        LayoutNode::leaf_image(ImageContent {
            src: "fixture.gif".into(),
            fit: ImageFit::Contain,
        })
        .with_ext("replaced", replaced_ext(Some(width), Some(height), None))
    }

    #[test]
    fn intrinsic_ratio_supplies_an_auto_axis() {
        let node = image(640.0, 360.0).with_width(SizeValue::Fixed(320.0));
        assert_eq!(
            resolve_replaced_size(&node, 800.0, 600.0),
            ReplacedSize {
                width: 320.0,
                height: 180.0
            }
        );
    }

    #[test]
    fn constraints_preserve_ratio_for_auto_axes() {
        let mut node = image(640.0, 360.0);
        node.max_width = Some(160.0);
        assert_eq!(resolve_replaced_size(&node, 800.0, 600.0).height, 90.0);
    }

    #[test]
    fn fit_modes_share_centered_geometry() {
        let intrinsic = IntrinsicSize {
            width: Some(200.0),
            height: Some(100.0),
            aspect_ratio: Some(2.0),
        };
        assert_eq!(
            object_fit_rect(ImageFit::Contain, 100.0, 100.0, intrinsic).height,
            50.0
        );
        let cover = object_fit_rect(ImageFit::Cover, 100.0, 100.0, intrinsic);
        assert_eq!((cover.x, cover.width, cover.clips), (-50.0, 200.0, true));
        let fill = object_fit_rect(ImageFit::Fill, 100.0, 100.0, intrinsic);
        assert_eq!((fill.width, fill.height, fill.clips), (100.0, 100.0, false));
        let none = object_fit_rect(ImageFit::None, 100.0, 50.0, intrinsic);
        assert_eq!((none.x, none.width, none.clips), (-50.0, 200.0, true));
    }

    #[test]
    fn malformed_metadata_is_ignored_and_diagnostic() {
        let node = LayoutNode::empty().with_ext(
            "replaced",
            ExtValue::Map(HashMap::from([
                ("intrinsicWidth".into(), ExtValue::Float(f64::NAN)),
                ("intrinsicHeight".into(), ExtValue::Float(-1.0)),
                ("aspectRatio".into(), ExtValue::Str("wide".into())),
            ])),
        );
        assert_eq!(IntrinsicSize::from_layout(&node), IntrinsicSize::default());
        assert_eq!(IntrinsicSize::diagnostics(&node).len(), 3);
    }
}
