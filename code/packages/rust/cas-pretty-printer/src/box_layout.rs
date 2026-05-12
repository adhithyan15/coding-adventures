//! Two-dimensional text box layout for CAS pretty printing.
//!
//! A [`Box`] is a rectangular region of text with a baseline row. Horizontal
//! composition aligns baselines, which lets fractions, powers, roots, and
//! plain atoms sit together in one expression.

use symbolic_ir::{IRApply, IRNode};

use crate::dialect::Dialect;
use crate::walker::format_node;

/// A rectangular text region with a mathematical baseline row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Box {
    /// Text rows in top-to-bottom order.
    pub lines: Vec<String>,
    /// Zero-based row index used for horizontal baseline alignment.
    pub baseline: usize,
}

impl Box {
    /// Construct a box from text rows and a baseline.
    pub fn new(lines: Vec<String>, baseline: usize) -> Self {
        Self { lines, baseline }
    }

    /// Width of the widest row.
    pub fn width(&self) -> usize {
        self.lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0)
    }

    /// Number of rows.
    pub fn height(&self) -> usize {
        self.lines.len()
    }

    /// Render rows as a single newline-delimited string.
    pub fn render(&self) -> String {
        self.lines.join("\n")
    }

    /// Return a copy with every row padded to `target` character cells.
    pub fn pad_width(&self, target: usize, align: Align) -> Self {
        if target <= self.width() {
            return self.clone();
        }

        let lines = self
            .lines
            .iter()
            .map(|line| {
                let pad = target.saturating_sub(line.chars().count());
                match align {
                    Align::Center => {
                        let left = pad / 2;
                        let right = pad - left;
                        format!("{}{}{}", " ".repeat(left), line, " ".repeat(right))
                    }
                    Align::Left => format!("{}{}", line, " ".repeat(pad)),
                    Align::Right => format!("{}{}", " ".repeat(pad), line),
                }
            })
            .collect();

        Self {
            lines,
            baseline: self.baseline,
        }
    }
}

/// Horizontal alignment used by [`Box::pad_width`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Center,
    Left,
    Right,
}

/// Create a single-line atom box.
pub fn atom(text: impl Into<String>) -> Box {
    Box::new(vec![text.into()], 0)
}

/// Align boxes on their baselines and concatenate them horizontally.
pub fn hbox(boxes: &[Box], sep: &str) -> Box {
    if boxes.is_empty() {
        return atom("");
    }

    let baseline = boxes.iter().map(|b| b.baseline).max().unwrap_or(0);
    let max_below = boxes
        .iter()
        .map(|b| b.height().saturating_sub(b.baseline + 1))
        .max()
        .unwrap_or(0);
    let total_height = baseline + 1 + max_below;

    let padded: Vec<Vec<String>> = boxes
        .iter()
        .map(|b| {
            let above = baseline - b.baseline;
            let below = total_height - b.height() - above;
            let empty = " ".repeat(b.width());
            let mut rows = Vec::with_capacity(total_height);
            rows.extend(std::iter::repeat(empty.clone()).take(above));
            rows.extend(b.lines.iter().cloned());
            rows.extend(std::iter::repeat(empty).take(below));
            rows
        })
        .collect();

    let lines = (0..total_height)
        .map(|row| {
            padded
                .iter()
                .map(|box_rows| box_rows[row].as_str())
                .collect::<Vec<_>>()
                .join(sep)
        })
        .collect();

    Box::new(lines, baseline)
}

/// Stack boxes vertically, centered to the widest box.
pub fn vbox(boxes: &[Box]) -> Box {
    if boxes.is_empty() {
        return atom("");
    }

    let width = boxes.iter().map(Box::width).max().unwrap_or(0);
    let lines = boxes
        .iter()
        .flat_map(|b| b.pad_width(width, Align::Center).lines)
        .collect::<Vec<_>>();
    let baseline = lines.len() / 2;

    Box::new(lines, baseline)
}

/// Format `node` as a multi-line 2D string.
pub fn pretty_2d(node: &IRNode, dialect: &dyn Dialect) -> String {
    build_box(node, dialect).render()
}

fn build_box(node: &IRNode, dialect: &dyn Dialect) -> Box {
    match node {
        IRNode::Integer(v) => atom(dialect.format_integer(*v)),
        IRNode::Rational(n, d) => atom(dialect.format_rational(*n, *d)),
        IRNode::Float(v) => atom(dialect.format_float(*v)),
        IRNode::Str(s) => atom(dialect.format_string(s)),
        IRNode::Symbol(name) => atom(dialect.format_symbol(name)),
        IRNode::Apply(apply) => build_apply_box(apply, dialect),
    }
}

fn build_apply_box(node: &IRApply, dialect: &dyn Dialect) -> Box {
    if let Some(sugared) = dialect.try_sugar(node) {
        return build_box(&sugared, dialect);
    }

    let head_name = match &node.head {
        IRNode::Symbol(s) => Some(s.as_str()),
        _ => None,
    };

    match (head_name, node.args.as_slice()) {
        (Some("Neg"), [inner]) => neg_box(build_box(inner, dialect)),
        (Some("Div"), [num, den]) => div_box(build_box(num, dialect), build_box(den, dialect)),
        (Some("Pow"), [base, exp]) => pow_box(build_box(base, dialect), build_box(exp, dialect)),
        (Some("Sqrt"), [arg]) => sqrt_box(build_box(arg, dialect)),
        (Some("Add"), [_, _, ..]) => infix_box(&node.args, " + ", dialect),
        (Some("Sub"), [left, right]) => hbox(
            &[
                build_box(left, dialect),
                atom(" - "),
                build_box(right, dialect),
            ],
            "",
        ),
        (Some("Mul"), [_, _, ..]) => infix_box(&node.args, "*", dialect),
        (Some("List"), args) => list_box(args, dialect),
        _ => atom(format_node(
            &IRNode::Apply(std::boxed::Box::new(node.clone())),
            dialect,
            0,
        )),
    }
}

fn neg_box(inner: Box) -> Box {
    let lines = inner
        .lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            if i == inner.baseline {
                format!("-{line}")
            } else {
                format!(" {line}")
            }
        })
        .collect();

    Box::new(lines, inner.baseline)
}

fn div_box(num: Box, den: Box) -> Box {
    let width = num.width().max(den.width()) + 2;
    let num = num.pad_width(width, Align::Center);
    let den = den.pad_width(width, Align::Center);
    let mut lines = num.lines;
    let baseline = lines.len();
    lines.push("─".repeat(width));
    lines.extend(den.lines);

    Box::new(lines, baseline)
}

fn pow_box(base: Box, exp: Box) -> Box {
    let base_blank = " ".repeat(base.width());
    let exp_blank = " ".repeat(exp.width());
    let mut lines = Vec::with_capacity(base.height() + exp.height());

    lines.extend(
        exp.lines
            .iter()
            .map(|line| format!("{}{}", base_blank, pad_right(line, exp.width()))),
    );
    lines.extend(
        base.lines
            .iter()
            .map(|line| format!("{}{}", pad_right(line, base.width()), exp_blank)),
    );

    Box::new(lines, exp.height() + base.baseline)
}

fn sqrt_box(arg: Box) -> Box {
    let arg_width = arg.width();
    let inner_width = arg_width + 2;
    let mut lines = Vec::with_capacity(arg.height() + 1);

    lines.push(format!("  ┌{}┐", "─".repeat(inner_width)));
    lines.extend(arg.lines.iter().enumerate().map(|(i, line)| {
        let prefix = if i == arg.baseline {
            "√ │"
        } else {
            "  │"
        };
        format!("{prefix} {} │", pad_right(line, arg_width))
    }));

    Box::new(lines, 1 + arg.baseline)
}

fn infix_box(args: &[IRNode], sep: &str, dialect: &dyn Dialect) -> Box {
    let mut parts = Vec::with_capacity(args.len().saturating_mul(2).saturating_sub(1));
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            parts.push(atom(sep));
        }
        parts.push(build_box(arg, dialect));
    }
    hbox(&parts, "")
}

fn list_box(args: &[IRNode], dialect: &dyn Dialect) -> Box {
    let (open, close) = dialect.list_brackets();
    if args.is_empty() {
        return atom(format!("{open}{close}"));
    }

    let mut parts = Vec::with_capacity(args.len().saturating_mul(2).saturating_sub(1));
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            parts.push(atom(", "));
        }
        parts.push(build_box(arg, dialect));
    }

    hbox(&[atom(open), hbox(&parts, ""), atom(close)], "")
}

fn pad_right(line: &str, width: usize) -> String {
    let pad = width.saturating_sub(line.chars().count());
    format!("{line}{}", " ".repeat(pad))
}
