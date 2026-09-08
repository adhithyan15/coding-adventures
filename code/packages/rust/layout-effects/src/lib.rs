//! Reusable visual-effect and affine-transform contracts over Layout IR.

use std::collections::HashMap;

use layout_ir::{Ext, ExtValue, LayoutNode, PositionedNode};

pub const VERSION: &str = "0.1.0";
pub type Transform2D = [f64; 6];
pub const IDENTITY: Transform2D = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OriginComponent {
    pub factor: f64,
    pub offset: f64,
}

impl OriginComponent {
    pub const fn percent(factor: f64) -> Self {
        Self {
            factor,
            offset: 0.0,
        }
    }

    pub const fn length(offset: f64) -> Self {
        Self {
            factor: 0.0,
            offset,
        }
    }

    pub fn resolve(self, extent: f64) -> f64 {
        self.factor * extent + self.offset
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformOrigin {
    pub x: OriginComponent,
    pub y: OriginComponent,
}

impl Default for TransformOrigin {
    fn default() -> Self {
        Self {
            x: OriginComponent::percent(0.5),
            y: OriginComponent::percent(0.5),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EffectBlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EffectFilter {
    Blur {
        radius: f64,
    },
    DropShadow {
        dx: f64,
        dy: f64,
        blur: f64,
        color: EffectColor,
    },
    Brightness {
        amount: f64,
    },
    Contrast {
        amount: f64,
    },
    Saturate {
        amount: f64,
    },
    HueRotate {
        angle: f64,
    },
    Invert {
        amount: f64,
    },
    Opacity {
        amount: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct EffectStyle {
    pub opacity: f64,
    pub transform: Transform2D,
    pub transform_origin: TransformOrigin,
    pub filters: Vec<EffectFilter>,
    pub blend_mode: EffectBlendMode,
    pub isolation: bool,
}

impl Default for EffectStyle {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            transform: IDENTITY,
            transform_origin: TransformOrigin::default(),
            filters: Vec::new(),
            blend_mode: EffectBlendMode::Normal,
            isolation: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectDiagnostic {
    pub key: String,
    pub message: String,
}

impl EffectStyle {
    pub fn from_layout(node: &LayoutNode) -> Self {
        Self::from_ext(&node.ext)
    }

    pub fn from_positioned(node: &PositionedNode) -> Self {
        Self::from_ext(&node.ext)
    }

    pub fn from_ext(ext: &Ext) -> Self {
        let Some(ExtValue::Map(values)) = ext.get("effects") else {
            return Self::default();
        };
        Self {
            opacity: number(values, "opacity").unwrap_or(1.0).clamp(0.0, 1.0),
            transform: transform(values.get("transform")).unwrap_or(IDENTITY),
            transform_origin: TransformOrigin {
                x: origin(values.get("originX")).unwrap_or(TransformOrigin::default().x),
                y: origin(values.get("originY")).unwrap_or(TransformOrigin::default().y),
            },
            filters: filters(values.get("filters")),
            blend_mode: blend_mode(string(values, "blendMode")),
            isolation: boolean(values, "isolation").unwrap_or(false),
        }
    }

    pub fn to_ext(&self) -> ExtValue {
        let mut values = HashMap::from([
            ("opacity".into(), ExtValue::Float(self.opacity)),
            (
                "transform".into(),
                ExtValue::List(self.transform.into_iter().map(ExtValue::Float).collect()),
            ),
            ("originX".into(), origin_ext(self.transform_origin.x)),
            ("originY".into(), origin_ext(self.transform_origin.y)),
            (
                "blendMode".into(),
                ExtValue::Str(blend_mode_name(self.blend_mode).into()),
            ),
            ("isolation".into(), ExtValue::Bool(self.isolation)),
        ]);
        values.insert(
            "filters".into(),
            ExtValue::List(self.filters.iter().map(filter_ext).collect()),
        );
        ExtValue::Map(values)
    }

    pub fn is_default(&self) -> bool {
        self.opacity == 1.0
            && self.transform == IDENTITY
            && self.filters.is_empty()
            && self.blend_mode == EffectBlendMode::Normal
            && !self.isolation
    }

    pub fn needs_layer(&self) -> bool {
        self.opacity < 1.0
            || !self.filters.is_empty()
            || self.blend_mode != EffectBlendMode::Normal
            || self.isolation
    }

    /// Resolve this box-local transform against absolute logical geometry.
    pub fn resolved_transform(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        scale: f64,
    ) -> Option<Transform2D> {
        if self.transform == IDENTITY {
            return None;
        }
        let scale = scale.max(0.01);
        let origin_x = (x + self.transform_origin.x.resolve(width)) * scale;
        let origin_y = (y + self.transform_origin.y.resolve(height)) * scale;
        let mut local = self.transform;
        local[4] *= scale;
        local[5] *= scale;
        Some(multiply(
            translation(origin_x, origin_y),
            multiply(local, translation(-origin_x, -origin_y)),
        ))
    }
}

pub fn multiply(left: Transform2D, right: Transform2D) -> Transform2D {
    [
        left[0] * right[0] + left[2] * right[1],
        left[1] * right[0] + left[3] * right[1],
        left[0] * right[2] + left[2] * right[3],
        left[1] * right[2] + left[3] * right[3],
        left[0] * right[4] + left[2] * right[5] + left[4],
        left[1] * right[4] + left[3] * right[5] + left[5],
    ]
}

pub const fn translation(x: f64, y: f64) -> Transform2D {
    [1.0, 0.0, 0.0, 1.0, x, y]
}

pub fn scale(x: f64, y: f64) -> Transform2D {
    [x, 0.0, 0.0, y, 0.0, 0.0]
}

pub fn rotation(degrees: f64) -> Transform2D {
    let radians = degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    [cos, sin, -sin, cos, 0.0, 0.0]
}

pub fn transform_point(transform: Transform2D, x: f64, y: f64) -> (f64, f64) {
    (
        transform[0] * x + transform[2] * y + transform[4],
        transform[1] * x + transform[3] * y + transform[5],
    )
}

pub fn diagnostics(ext: &Ext) -> Vec<EffectDiagnostic> {
    let Some(ExtValue::Map(values)) = ext.get("effects") else {
        return Vec::new();
    };
    let mut diagnostics = Vec::new();
    if matches!(values.get("opacity"), Some(value) if number_value(value).is_none()) {
        diagnostics.push(EffectDiagnostic {
            key: "opacity".into(),
            message: "effect opacity must be a finite number".into(),
        });
    }
    if matches!(values.get("transform"), Some(value) if transform(Some(value)).is_none()) {
        diagnostics.push(EffectDiagnostic {
            key: "transform".into(),
            message: "effect transform must contain six finite numbers".into(),
        });
    }
    if matches!(values.get("filters"), Some(value) if !matches!(value, ExtValue::List(_))) {
        diagnostics.push(EffectDiagnostic {
            key: "filters".into(),
            message: "effect filters must be a list".into(),
        });
    }
    diagnostics
}

fn origin_ext(origin: OriginComponent) -> ExtValue {
    ExtValue::Map(HashMap::from([
        ("factor".into(), ExtValue::Float(origin.factor)),
        ("offset".into(), ExtValue::Float(origin.offset)),
    ]))
}

fn origin(value: Option<&ExtValue>) -> Option<OriginComponent> {
    let ExtValue::Map(values) = value? else {
        return None;
    };
    Some(OriginComponent {
        factor: number(values, "factor")?,
        offset: number(values, "offset")?,
    })
}

fn transform(value: Option<&ExtValue>) -> Option<Transform2D> {
    let ExtValue::List(values) = value? else {
        return None;
    };
    let values = values
        .iter()
        .map(number_value)
        .collect::<Option<Vec<_>>>()?;
    values.try_into().ok()
}

fn filters(value: Option<&ExtValue>) -> Vec<EffectFilter> {
    let Some(ExtValue::List(values)) = value else {
        return Vec::new();
    };
    values.iter().filter_map(parse_filter).collect()
}

fn parse_filter(value: &ExtValue) -> Option<EffectFilter> {
    let ExtValue::Map(values) = value else {
        return None;
    };
    match string(values, "kind")? {
        "blur" => Some(EffectFilter::Blur {
            radius: number(values, "radius")?.max(0.0),
        }),
        "drop-shadow" => Some(EffectFilter::DropShadow {
            dx: number(values, "dx")?,
            dy: number(values, "dy")?,
            blur: number(values, "blur")?.max(0.0),
            color: color(values.get("color"))?,
        }),
        "brightness" => Some(EffectFilter::Brightness {
            amount: number(values, "amount")?.max(0.0),
        }),
        "contrast" => Some(EffectFilter::Contrast {
            amount: number(values, "amount")?.max(0.0),
        }),
        "saturate" => Some(EffectFilter::Saturate {
            amount: number(values, "amount")?.max(0.0),
        }),
        "hue-rotate" => Some(EffectFilter::HueRotate {
            angle: number(values, "angle")?,
        }),
        "invert" => Some(EffectFilter::Invert {
            amount: number(values, "amount")?.clamp(0.0, 1.0),
        }),
        "opacity" => Some(EffectFilter::Opacity {
            amount: number(values, "amount")?.clamp(0.0, 1.0),
        }),
        _ => None,
    }
}

fn filter_ext(filter: &EffectFilter) -> ExtValue {
    let mut values = HashMap::new();
    match filter {
        EffectFilter::Blur { radius } => {
            values.insert("kind".into(), ExtValue::Str("blur".into()));
            values.insert("radius".into(), ExtValue::Float(*radius));
        }
        EffectFilter::DropShadow {
            dx,
            dy,
            blur,
            color,
        } => {
            values.insert("kind".into(), ExtValue::Str("drop-shadow".into()));
            values.insert("dx".into(), ExtValue::Float(*dx));
            values.insert("dy".into(), ExtValue::Float(*dy));
            values.insert("blur".into(), ExtValue::Float(*blur));
            values.insert("color".into(), color_ext(*color));
        }
        EffectFilter::Brightness { amount }
        | EffectFilter::Contrast { amount }
        | EffectFilter::Saturate { amount }
        | EffectFilter::Invert { amount }
        | EffectFilter::Opacity { amount } => {
            let kind = match filter {
                EffectFilter::Brightness { .. } => "brightness",
                EffectFilter::Contrast { .. } => "contrast",
                EffectFilter::Saturate { .. } => "saturate",
                EffectFilter::Invert { .. } => "invert",
                EffectFilter::Opacity { .. } => "opacity",
                _ => unreachable!(),
            };
            values.insert("kind".into(), ExtValue::Str(kind.into()));
            values.insert("amount".into(), ExtValue::Float(*amount));
        }
        EffectFilter::HueRotate { angle } => {
            values.insert("kind".into(), ExtValue::Str("hue-rotate".into()));
            values.insert("angle".into(), ExtValue::Float(*angle));
        }
    }
    ExtValue::Map(values)
}

fn color_ext(color: EffectColor) -> ExtValue {
    ExtValue::Map(HashMap::from([
        ("r".into(), ExtValue::Int(i64::from(color.r))),
        ("g".into(), ExtValue::Int(i64::from(color.g))),
        ("b".into(), ExtValue::Int(i64::from(color.b))),
        ("a".into(), ExtValue::Int(i64::from(color.a))),
    ]))
}

fn color(value: Option<&ExtValue>) -> Option<EffectColor> {
    let ExtValue::Map(values) = value? else {
        return None;
    };
    Some(EffectColor {
        r: byte(values, "r")?,
        g: byte(values, "g")?,
        b: byte(values, "b")?,
        a: byte(values, "a").unwrap_or(255),
    })
}

fn number(values: &HashMap<String, ExtValue>, key: &str) -> Option<f64> {
    number_value(values.get(key)?)
}

fn number_value(value: &ExtValue) -> Option<f64> {
    let value = match value {
        ExtValue::Float(value) => *value,
        ExtValue::Int(value) => *value as f64,
        _ => return None,
    };
    value.is_finite().then_some(value)
}

fn byte(values: &HashMap<String, ExtValue>, key: &str) -> Option<u8> {
    number(values, key).map(|value| value.round().clamp(0.0, 255.0) as u8)
}

fn string<'a>(values: &'a HashMap<String, ExtValue>, key: &str) -> Option<&'a str> {
    match values.get(key)? {
        ExtValue::Str(value) => Some(value),
        _ => None,
    }
}

fn boolean(values: &HashMap<String, ExtValue>, key: &str) -> Option<bool> {
    match values.get(key)? {
        ExtValue::Bool(value) => Some(*value),
        _ => None,
    }
}

fn blend_mode(value: Option<&str>) -> EffectBlendMode {
    match value {
        Some("multiply") => EffectBlendMode::Multiply,
        Some("screen") => EffectBlendMode::Screen,
        Some("overlay") => EffectBlendMode::Overlay,
        Some("darken") => EffectBlendMode::Darken,
        Some("lighten") => EffectBlendMode::Lighten,
        Some("color-dodge") => EffectBlendMode::ColorDodge,
        Some("color-burn") => EffectBlendMode::ColorBurn,
        Some("hard-light") => EffectBlendMode::HardLight,
        Some("soft-light") => EffectBlendMode::SoftLight,
        Some("difference") => EffectBlendMode::Difference,
        Some("exclusion") => EffectBlendMode::Exclusion,
        Some("hue") => EffectBlendMode::Hue,
        Some("saturation") => EffectBlendMode::Saturation,
        Some("color") => EffectBlendMode::Color,
        Some("luminosity") => EffectBlendMode::Luminosity,
        _ => EffectBlendMode::Normal,
    }
}

fn blend_mode_name(value: EffectBlendMode) -> &'static str {
    match value {
        EffectBlendMode::Normal => "normal",
        EffectBlendMode::Multiply => "multiply",
        EffectBlendMode::Screen => "screen",
        EffectBlendMode::Overlay => "overlay",
        EffectBlendMode::Darken => "darken",
        EffectBlendMode::Lighten => "lighten",
        EffectBlendMode::ColorDodge => "color-dodge",
        EffectBlendMode::ColorBurn => "color-burn",
        EffectBlendMode::HardLight => "hard-light",
        EffectBlendMode::SoftLight => "soft-light",
        EffectBlendMode::Difference => "difference",
        EffectBlendMode::Exclusion => "exclusion",
        EffectBlendMode::Hue => "hue",
        EffectBlendMode::Saturation => "saturation",
        EffectBlendMode::Color => "color",
        EffectBlendMode::Luminosity => "luminosity",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_round_trips_and_clamps_opacity() {
        let style = EffectStyle {
            opacity: 0.4,
            transform: multiply(translation(12.0, 4.0), rotation(90.0)),
            filters: vec![EffectFilter::Blur { radius: 2.0 }],
            blend_mode: EffectBlendMode::Multiply,
            isolation: true,
            ..EffectStyle::default()
        };
        let mut node = LayoutNode::container(Vec::new());
        node.ext.insert("effects".into(), style.to_ext());
        assert_eq!(EffectStyle::from_layout(&node), style);
    }

    #[test]
    fn transform_origin_resolves_around_positioned_box() {
        let style = EffectStyle {
            transform: scale(2.0, 2.0),
            ..EffectStyle::default()
        };
        let transform = style
            .resolved_transform(10.0, 20.0, 40.0, 20.0, 1.0)
            .unwrap();
        assert_eq!(transform_point(transform, 30.0, 30.0), (30.0, 30.0));
        assert_eq!(transform_point(transform, 10.0, 20.0), (-10.0, 10.0));
    }

    #[test]
    fn malformed_metadata_reports_diagnostics() {
        let ext = HashMap::from([(
            "effects".into(),
            ExtValue::Map(HashMap::from([
                ("opacity".into(), ExtValue::Str("opaque".into())),
                (
                    "transform".into(),
                    ExtValue::List(vec![ExtValue::Float(1.0)]),
                ),
            ])),
        )]);
        assert_eq!(diagnostics(&ext).len(), 2);
    }
}
