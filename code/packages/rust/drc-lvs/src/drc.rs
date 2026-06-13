//! Geometric design-rule checking.
//!
//! Operates on axis-aligned rectangles on named layers. v0.1.0 uses
//! O(n²) pairwise checks; sufficient for the 4-bit-adder smoke test.
//! An R-tree index for scale comes in v0.2.0.

use std::collections::HashMap;

/// One rectangle of geometry on a named layer.
#[derive(Debug, Clone, PartialEq)]
pub struct DrcRect {
    /// GDS layer name (e.g., "met1", "poly").
    pub layer: String,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl DrcRect {
    pub fn width(&self)  -> f64 { self.x2 - self.x1 }
    pub fn height(&self) -> f64 { self.y2 - self.y1 }
    pub fn area(&self)   -> f64 { self.width() * self.height() }
}

/// One DRC rule violation.
#[derive(Debug, Clone, PartialEq)]
pub struct Violation {
    /// Rule name (e.g., "met1.W").
    pub rule: String,
    /// "error" or "warning".
    pub severity: String,
    pub layer: String,
    pub location_x: f64,
    pub location_y: f64,
    pub description: String,
}

/// The kind of check this rule performs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleKind {
    MinWidth,
    MinSpacing,
    MinArea,
}

/// One design rule.
#[derive(Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub layer: String,
    pub kind: RuleKind,
    /// Rule threshold in µm or µm².
    pub value: f64,
    /// "error" or "warning".
    pub severity: String,
}

impl Rule {
    pub fn min_width(name: &str, layer: &str, value: f64) -> Self {
        Self { name: name.into(), layer: layer.into(), kind: RuleKind::MinWidth, value, severity: "error".into() }
    }
    pub fn min_spacing(name: &str, layer: &str, value: f64) -> Self {
        Self { name: name.into(), layer: layer.into(), kind: RuleKind::MinSpacing, value, severity: "error".into() }
    }
    pub fn min_area(name: &str, layer: &str, value: f64) -> Self {
        Self { name: name.into(), layer: layer.into(), kind: RuleKind::MinArea, value, severity: "error".into() }
    }
}

/// DRC results.
#[derive(Debug, Default)]
pub struct DrcReport {
    pub violations: Vec<Violation>,
    pub rules_checked: usize,
}

impl DrcReport {
    /// Returns `true` if there are no error-severity violations.
    pub fn clean(&self) -> bool {
        !self.violations.iter().any(|v| v.severity == "error")
    }
}

/// Run all rules against the given rectangles.
pub fn run_drc(rects: &[DrcRect], rules: &[Rule]) -> DrcReport {
    let mut report = DrcReport { rules_checked: rules.len(), ..Default::default() };

    let mut by_layer: HashMap<&str, Vec<&DrcRect>> = HashMap::new();
    for r in rects {
        by_layer.entry(r.layer.as_str()).or_default().push(r);
    }

    for rule in rules {
        let layer_rects: Vec<&DrcRect> = by_layer.get(rule.layer.as_str()).cloned().unwrap_or_default();
        match rule.kind {
            RuleKind::MinWidth   => check_min_width(rule, layer_rects, &mut report),
            RuleKind::MinSpacing => check_min_spacing(rule, layer_rects, &mut report),
            RuleKind::MinArea    => check_min_area(rule, layer_rects, &mut report),
        }
    }

    report
}

// ---------------------------------------------------------------------------
// Rule checks
// ---------------------------------------------------------------------------

fn check_min_width(rule: &Rule, rects: Vec<&DrcRect>, report: &mut DrcReport) {
    for r in rects {
        if r.width() < rule.value || r.height() < rule.value {
            report.violations.push(Violation {
                rule: rule.name.clone(),
                severity: rule.severity.clone(),
                layer: rule.layer.clone(),
                location_x: r.x1,
                location_y: r.y1,
                description: format!(
                    "min_width {:.4} violated: {:.4}×{:.4}",
                    rule.value, r.width(), r.height()
                ),
            });
        }
    }
}

fn check_min_spacing(rule: &Rule, rects: Vec<&DrcRect>, report: &mut DrcReport) {
    for i in 0..rects.len() {
        for j in i+1..rects.len() {
            let spacing = rect_spacing(rects[i], rects[j]);
            if (0.0..rule.value).contains(&spacing) {
                report.violations.push(Violation {
                    rule: rule.name.clone(),
                    severity: rule.severity.clone(),
                    layer: rule.layer.clone(),
                    location_x: (rects[i].x1 + rects[j].x1) / 2.0,
                    location_y: (rects[i].y1 + rects[j].y1) / 2.0,
                    description: format!(
                        "min_spacing {:.4} violated: {spacing:.4}",
                        rule.value
                    ),
                });
            }
        }
    }
}

fn check_min_area(rule: &Rule, rects: Vec<&DrcRect>, report: &mut DrcReport) {
    for r in rects {
        if r.area() < rule.value {
            report.violations.push(Violation {
                rule: rule.name.clone(),
                severity: rule.severity.clone(),
                layer: rule.layer.clone(),
                location_x: r.x1,
                location_y: r.y1,
                description: format!(
                    "min_area {:.4} violated: {:.4}",
                    rule.value, r.area()
                ),
            });
        }
    }
}

/// Minimum Euclidean distance between two non-overlapping rectangles.
/// Returns 0.0 if touching; -1.0 if overlapping.
fn rect_spacing(a: &DrcRect, b: &DrcRect) -> f64 {
    // Overlap check.
    if !(a.x2 <= b.x1 || b.x2 <= a.x1 || a.y2 <= b.y1 || b.y2 <= a.y1) {
        return -1.0;
    }
    let dx = (b.x1 - a.x2).max(a.x1 - b.x2).max(0.0);
    let dy = (b.y1 - a.y2).max(a.y1 - b.y2).max(0.0);
    if dx == 0.0 && dy == 0.0 { return 0.0; }
    (dx * dx + dy * dy).sqrt()
}
