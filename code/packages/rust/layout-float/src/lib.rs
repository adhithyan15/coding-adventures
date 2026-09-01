//! Reusable CSS float and exclusion geometry over `layout-ir`.

use std::collections::HashMap;

use layout_ir::{Ext, ExtValue, LayoutNode, PositionedNode};

pub const VERSION: &str = "0.1.0";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FloatSide {
    #[default]
    None,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Clear {
    #[default]
    None,
    Left,
    Right,
    Both,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FloatStyle {
    pub side: FloatSide,
    pub clear: Clear,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FloatDiagnostic {
    pub key: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exclusion {
    pub side: FloatSide,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Exclusion {
    pub fn right(self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(self) -> f64 {
        self.y + self.height
    }

    fn overlaps(self, y: f64, height: f64) -> bool {
        let bottom = y + height.max(f64::EPSILON);
        self.y < bottom && self.bottom() > y
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AvailableBand {
    pub x: f64,
    pub width: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatPlacement {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExclusionSpace {
    width: f64,
    exclusions: Vec<Exclusion>,
}

impl FloatStyle {
    pub fn from_layout(node: &LayoutNode) -> Self {
        Self::from_ext(&node.ext)
    }

    pub fn from_positioned(node: &PositionedNode) -> Self {
        Self::from_ext(&node.ext)
    }

    pub fn from_ext(ext: &Ext) -> Self {
        let Some(ExtValue::Map(values)) = ext.get("float") else {
            return Self::default();
        };
        Self {
            side: match string(values, "side") {
                Some("left") => FloatSide::Left,
                Some("right") => FloatSide::Right,
                _ => FloatSide::None,
            },
            clear: match string(values, "clear") {
                Some("left") => Clear::Left,
                Some("right") => Clear::Right,
                Some("both") => Clear::Both,
                _ => Clear::None,
            },
        }
    }

    pub fn to_ext(self) -> ExtValue {
        ExtValue::Map(HashMap::from([
            (
                "side".into(),
                ExtValue::Str(
                    match self.side {
                        FloatSide::None => "none",
                        FloatSide::Left => "left",
                        FloatSide::Right => "right",
                    }
                    .into(),
                ),
            ),
            (
                "clear".into(),
                ExtValue::Str(
                    match self.clear {
                        Clear::None => "none",
                        Clear::Left => "left",
                        Clear::Right => "right",
                        Clear::Both => "both",
                    }
                    .into(),
                ),
            ),
        ]))
    }

    pub fn is_floating(self) -> bool {
        self.side != FloatSide::None
    }

    pub fn diagnostics(node: &LayoutNode) -> Vec<FloatDiagnostic> {
        let Some(ExtValue::Map(values)) = node.ext.get("float") else {
            return Vec::new();
        };
        let mut diagnostics = Vec::new();
        if let Some(value) = string(values, "side") {
            if !matches!(value, "none" | "left" | "right") {
                diagnostics.push(FloatDiagnostic {
                    key: "side".into(),
                    message: format!("unsupported float side `{value}`; using none"),
                });
            }
        }
        if let Some(value) = string(values, "clear") {
            if !matches!(value, "none" | "left" | "right" | "both") {
                diagnostics.push(FloatDiagnostic {
                    key: "clear".into(),
                    message: format!("unsupported clear value `{value}`; using none"),
                });
            }
        }
        diagnostics
    }
}

pub fn float_ext(style: FloatStyle) -> ExtValue {
    style.to_ext()
}

impl ExclusionSpace {
    pub fn new(width: f64) -> Self {
        Self {
            width: finite_non_negative(width),
            exclusions: Vec::new(),
        }
    }

    pub fn exclusions(&self) -> &[Exclusion] {
        &self.exclusions
    }

    pub fn available_band(&self, y: f64, height: f64) -> AvailableBand {
        let y = finite_non_negative(y);
        let height = finite_non_negative(height);
        let mut left = 0.0;
        let mut right = self.width;
        for exclusion in self
            .exclusions
            .iter()
            .copied()
            .filter(|exclusion| exclusion.overlaps(y, height))
        {
            match exclusion.side {
                FloatSide::Left => left = f64::max(left, exclusion.right()),
                FloatSide::Right => right = f64::min(right, exclusion.x),
                FloatSide::None => {}
            }
        }
        AvailableBand {
            x: left.min(self.width),
            width: (right - left).max(0.0),
        }
    }

    pub fn clearance_y(&self, clear: Clear, y: f64) -> f64 {
        let mut result = finite_non_negative(y);
        for exclusion in &self.exclusions {
            let matches = match clear {
                Clear::None => false,
                Clear::Left => exclusion.side == FloatSide::Left,
                Clear::Right => exclusion.side == FloatSide::Right,
                Clear::Both => exclusion.side != FloatSide::None,
            };
            if matches && exclusion.bottom() > result {
                result = exclusion.bottom();
            }
        }
        result
    }

    pub fn next_y_with_width(&self, y: f64, height: f64, required_width: f64) -> f64 {
        let mut candidate = finite_non_negative(y);
        let required_width = finite_non_negative(required_width).min(self.width);
        loop {
            if self.available_band(candidate, height).width + f64::EPSILON >= required_width {
                return candidate;
            }
            let next = self
                .exclusions
                .iter()
                .copied()
                .filter(|exclusion| exclusion.overlaps(candidate, height))
                .map(Exclusion::bottom)
                .filter(|bottom| *bottom > candidate)
                .fold(f64::INFINITY, f64::min);
            if !next.is_finite() {
                return candidate;
            }
            candidate = next;
        }
    }

    pub fn place(&mut self, side: FloatSide, y: f64, width: f64, height: f64) -> FloatPlacement {
        let width = finite_non_negative(width).min(self.width);
        let height = finite_non_negative(height);
        let y = self.next_y_with_width(y, height, width);
        let band = self.available_band(y, height);
        let x = match side {
            FloatSide::Right => band.x + (band.width - width).max(0.0),
            FloatSide::None | FloatSide::Left => band.x,
        };
        if side != FloatSide::None {
            self.exclusions.push(Exclusion {
                side,
                x,
                y,
                width,
                height,
            });
        }
        FloatPlacement {
            x,
            y,
            width,
            height,
        }
    }

    pub fn bottom(&self) -> f64 {
        self.exclusions
            .iter()
            .copied()
            .map(Exclusion::bottom)
            .fold(0.0, f64::max)
    }
}

pub fn shrink_to_fit(preferred_min: f64, preferred: f64, available: f64) -> f64 {
    let available = finite_non_negative(available);
    let preferred_min = finite_non_negative(preferred_min).min(available);
    let preferred = finite_non_negative(preferred).max(preferred_min);
    preferred_min.max(available.min(preferred))
}

fn string<'a>(values: &'a HashMap<String, ExtValue>, key: &str) -> Option<&'a str> {
    let ExtValue::Str(value) = values.get(key)? else {
        return None;
    };
    Some(value)
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

    #[test]
    fn opposing_floats_share_a_band_then_stack_when_space_runs_out() {
        let mut space = ExclusionSpace::new(300.0);
        assert_eq!(space.place(FloatSide::Left, 0.0, 100.0, 80.0).x, 0.0);
        assert_eq!(space.place(FloatSide::Right, 0.0, 120.0, 60.0).x, 180.0);
        let third = space.place(FloatSide::Left, 0.0, 100.0, 20.0);
        assert_eq!((third.x, third.y), (100.0, 60.0));
    }

    #[test]
    fn clear_selects_matching_float_sides() {
        let mut space = ExclusionSpace::new(300.0);
        space.place(FloatSide::Left, 0.0, 90.0, 70.0);
        space.place(FloatSide::Right, 0.0, 90.0, 40.0);
        assert_eq!(space.clearance_y(Clear::Left, 10.0), 70.0);
        assert_eq!(space.clearance_y(Clear::Right, 10.0), 40.0);
        assert_eq!(space.clearance_y(Clear::Both, 10.0), 70.0);
    }

    #[test]
    fn available_band_tracks_active_exclusions() {
        let mut space = ExclusionSpace::new(320.0);
        space.place(FloatSide::Left, 0.0, 80.0, 50.0);
        space.place(FloatSide::Right, 0.0, 60.0, 100.0);
        assert_eq!(
            space.available_band(10.0, 20.0),
            AvailableBand {
                x: 80.0,
                width: 180.0
            }
        );
        assert_eq!(
            space.available_band(60.0, 20.0),
            AvailableBand {
                x: 0.0,
                width: 260.0
            }
        );
        assert_eq!(
            space.available_band(110.0, 20.0),
            AvailableBand {
                x: 0.0,
                width: 320.0
            }
        );
    }

    #[test]
    fn shrink_to_fit_respects_intrinsic_bounds() {
        assert_eq!(shrink_to_fit(80.0, 220.0, 140.0), 140.0);
        assert_eq!(shrink_to_fit(80.0, 220.0, 300.0), 220.0);
        assert_eq!(shrink_to_fit(180.0, 220.0, 120.0), 120.0);
    }

    #[test]
    fn malformed_values_are_diagnostic_and_default_safe() {
        let node = LayoutNode::empty().with_ext(
            "float",
            ExtValue::Map(HashMap::from([
                ("side".into(), ExtValue::Str("center".into())),
                ("clear".into(), ExtValue::Str("inline".into())),
            ])),
        );
        assert_eq!(FloatStyle::from_layout(&node), FloatStyle::default());
        assert_eq!(FloatStyle::diagnostics(&node).len(), 2);
    }
}
