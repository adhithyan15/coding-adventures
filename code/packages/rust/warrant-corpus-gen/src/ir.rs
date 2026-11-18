//! # The `ir.core` model IR
//!
//! Appendix A of VER04r lists the 26 tags of `ir.core`.  This module is that table as a
//! Rust `enum`, plus its JSON form and the few structural queries every later stage
//! needs (does a tree call the subject? which operations does it name?).
//!
//! ```text
//!   lit var subject_op model_call len ok err           -- leaves and calls
//!   eq ne lt gt le ge                                  -- comparisons
//!   add sub mul neg                                    -- arithmetic (wraps, VER04r 3)
//!   and or implies not if                              -- logic (short-circuit)
//!   concat take drop is_err                            -- bytes surgery and a Result test
//! ```
//!
//! The first 21 are the tags the v1 corpus emits today; `neg concat take drop is_err` are
//! the five v2 additions (VER04r 4.1, reconciled in 15.2 item 8).
//!
//! ## `ir.fixed` (slice S10b)
//!
//! The 35 built-ins of Appendix A.1 are one variant, [`Ir::Fixed`], carrying the tag, the
//! optional static type argument (`widen<U32>(x)` is `["widen","U32",x]`) and the operands.  The
//! lowering produces them and the cost walk counts them (`1 +` the operands); **nothing
//! evaluates them yet** (slice S10c-1), and [`crate::eval`] refuses the variant, naming the level.
//!
//! ## Wire form
//!
//! The JSON is the v1 array encoding, unchanged: `["var","a"]`, `["lit",true]`,
//! `["lit",{"type":"U64","value":"7"}]`, `["subject_op","eq",[a,b]]` (plus a trailing
//! `[widths]` for a static-width call), `["if",c,t,e]`.
//!
//! ## What the tree remembers that the JSON does not
//!
//! `Arith` and `Neg` carry the operand type [`Ty`]: the wrapping rule of VER04r 3.2 needs
//! to know "fixed-width or exact?" for every node, and the lowering's typer is the only
//! place that knows.  The type is not written to JSON (the consumer re-derives it from
//! the literals and the declared types, which is what the oracle does too).
//!
//! ## Depth
//!
//! Lowering caps expression nesting at [`MAX_IR_DEPTH`] = 64 (VER04r 14), so
//! [`Ir::to_json`] and the cost walk may recurse on lowered trees.  A hand-built tree is
//! checked with [`Ir::depth`] before it is costed.

use coding_adventures_warrant_grammar::fixed::FixedTy;

use crate::json::Json;
use crate::types::Ty;

/// The IR nesting limit of VER04r section 14 ("descriptor tree depth; type depth; IR
/// depth": 16; 16; 64).
pub const MAX_IR_DEPTH: usize = 64;

/// A comparison tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cmp {
    /// `eq`
    Eq,
    /// `ne`
    Ne,
    /// `lt`
    Lt,
    /// `gt`
    Gt,
    /// `le`
    Le,
    /// `ge`
    Ge,
}

impl Cmp {
    fn tag(self) -> &'static str {
        match self {
            Cmp::Eq => "eq",
            Cmp::Ne => "ne",
            Cmp::Lt => "lt",
            Cmp::Gt => "gt",
            Cmp::Le => "le",
            Cmp::Ge => "ge",
        }
    }
}

/// A binary arithmetic tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arith {
    /// `add`
    Add,
    /// `sub`
    Sub,
    /// `mul`
    Mul,
}

impl Arith {
    fn tag(self) -> &'static str {
        match self {
            Arith::Add => "add",
            Arith::Sub => "sub",
            Arith::Mul => "mul",
        }
    }
}

/// A short-circuit Boolean tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Logic {
    /// `and`
    And,
    /// `or`
    Or,
    /// `implies`
    Implies,
}

impl Logic {
    fn tag(self) -> &'static str {
        match self {
            Logic::And => "and",
            Logic::Or => "or",
            Logic::Implies => "implies",
        }
    }
}

/// One node of an `ir.core` expression tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ir {
    /// `["lit", true]`.
    Bool(bool),
    /// `["lit", {"type": T, "value": "V"}]` for `T` in `Nat`, `Int` or a fixed-width
    /// type; `value` is the exact integer.
    Int {
        /// The literal's type (`Nat`, `Int` or `Fixed`).
        ty: Ty,
        /// The value (a negative value only for a signed type's minimum, or `Int`).
        value: i128,
    },
    /// `["lit", {"type": "Bytes", "value": "<hex>"}]`; the text is validated lowercase,
    /// even-length hex.
    Bytes(String),
    /// `["var", name]`.
    Var(String),
    /// `["subject_op", name, [args]]` with a trailing `[widths]` when `widths` is
    /// non-empty.
    Subject {
        /// The operation's Warrant name.
        op: String,
        /// The arguments, left to right.
        args: Vec<Ir>,
        /// Static widths (`eq_fixed<4>` is `[4]`).
        widths: Vec<u64>,
    },
    /// `["model_call", name, [args]]`.
    Model {
        /// The model's name.
        name: String,
        /// The arguments, left to right.
        args: Vec<Ir>,
    },
    /// `["len", e]`.
    Len(Box<Ir>),
    /// `["ok", e]`.
    Ok(Box<Ir>),
    /// `["err", "CASE"]`.
    Err(String),
    /// `eq ne lt gt le ge`.
    Cmp(Cmp, Box<Ir>, Box<Ir>),
    /// `add sub mul`, with the operand type for the wrapping rule.
    Arith(Arith, Ty, Box<Ir>, Box<Ir>),
    /// `["neg", e]`, with the operand type.
    Neg(Ty, Box<Ir>),
    /// `and or implies`.
    Logic(Logic, Box<Ir>, Box<Ir>),
    /// `["not", e]`.
    Not(Box<Ir>),
    /// `["if", c, t, e]`.
    If(Box<Ir>, Box<Ir>, Box<Ir>),
    /// `["concat", a, b]`.
    Concat(Box<Ir>, Box<Ir>),
    /// `["take", e, n]`.
    Take(Box<Ir>, Box<Ir>),
    /// `["drop", e, n]`.
    Drop(Box<Ir>, Box<Ir>),
    /// `["is_err", e]`.
    IsErr(Box<Ir>),
    /// One of the 35 `ir.fixed` built-ins of Appendix A.1 (see [`FIXED_TAGS`]): `["tag", args..]`,
    /// or `["tag", "U32", args..]` when the built-in takes a static type argument.
    Fixed {
        /// The tag, one of [`FIXED_TAGS`].
        tag: &'static str,
        /// The static type argument of `widen wrap try max_value min_value from_be from_le`.
        ty: Option<FixedTy>,
        /// The operands, left to right (the static type argument is not an operand).
        args: Vec<Ir>,
    },
}

/// Exactly the 35 names of `ir.fixed` (VER04r 4.1, Appendix A.1, [D24]), in the oracle's order.
/// `try` is spelled `try`.
pub const FIXED_TAGS: [&str; 35] = [
    "bit_and",
    "bit_or",
    "bit_xor",
    "bit_not",
    "shl",
    "shr",
    "sar",
    "rotl",
    "rotr",
    "popcount",
    "clz",
    "ctz",
    "bit",
    "widen",
    "wrap",
    "try",
    "to_int",
    "to_nat",
    "max_value",
    "min_value",
    "checked_add",
    "checked_sub",
    "checked_mul",
    "div",
    "rem",
    "div_trunc",
    "rem_trunc",
    "imin",
    "imax",
    "be_bytes",
    "le_bytes",
    "from_be",
    "from_le",
    "at",
    "unwrap_or",
];

/// The nine built-ins whose result is an `Option<T>` (VER04r 16.10 [D23]); `unwrap_or` is the
/// only consumer of an `Option`, as its first operand and directly.
pub const OPTION_PRODUCERS: [&str; 9] = [
    "try",
    "checked_add",
    "checked_sub",
    "checked_mul",
    "div",
    "rem",
    "div_trunc",
    "rem_trunc",
    "at",
];

/// The built-ins that take one static type argument, written in position 1 of their node.
pub const STATIC_TYPE_TAGS: [&str; 7] = [
    "widen",
    "wrap",
    "try",
    "max_value",
    "min_value",
    "from_be",
    "from_le",
];

/// The `ir.fixed` tag spelled `name`, as the `'static` string of [`FIXED_TAGS`].
pub fn fixed_tag(name: &str) -> Option<&'static str> {
    FIXED_TAGS.iter().find(|t| **t == name).copied()
}

/// The `requires` of lowered trees (VER04r 16.10 R9, the tag part; the spec calls it
/// `ir::levels`): `ir.core` (always present), `ir.fixed` for every [`Ir::Fixed`] node, and, for
/// every `model_call`, the callee's own `requires` as `callee` returns it (already closed over
/// the callee's callees, so the closure is never walked again).  Sorted and duplicate-free.
pub fn levels<'t>(
    trees: impl IntoIterator<Item = &'t Ir>,
    callee: &dyn Fn(&str) -> Vec<&'static str>,
) -> Vec<&'static str> {
    let mut found: std::collections::BTreeSet<&'static str> = ["ir.core"].into();
    for tree in trees {
        tree.walk(&mut |n| match n {
            Ir::Fixed { .. } => {
                found.insert("ir.fixed");
            }
            Ir::Model { name, .. } => found.extend(callee(name)),
            _ => {}
        });
    }
    found.into_iter().collect()
}

/// How many operands an `ir.fixed` tag has (the static type argument is not one).
pub fn fixed_arity(tag: &str) -> usize {
    match tag {
        "max_value" | "min_value" => 0,
        "widen" | "wrap" | "try" | "to_int" | "to_nat" | "bit_not" | "popcount" | "clz" | "ctz"
        | "be_bytes" | "le_bytes" | "from_be" | "from_le" => 1,
        _ => 2,
    }
}

/// The 26 tags of Appendix A, in the order the appendix lists them.
pub const ALL_TAGS: [&str; 26] = [
    "lit",
    "var",
    "subject_op",
    "model_call",
    "len",
    "ok",
    "err",
    "eq",
    "ne",
    "lt",
    "gt",
    "le",
    "ge",
    "add",
    "sub",
    "mul",
    "and",
    "or",
    "not",
    "implies",
    "if",
    "neg",
    "concat",
    "take",
    "drop",
    "is_err",
];

impl Ir {
    /// `["lit", true]` (an empty `admit`).
    pub fn t() -> Ir {
        Ir::Bool(true)
    }

    /// The JSON tag of this node.
    pub fn tag(&self) -> &'static str {
        match self {
            Ir::Bool(_) | Ir::Int { .. } | Ir::Bytes(_) => "lit",
            Ir::Var(_) => "var",
            Ir::Subject { .. } => "subject_op",
            Ir::Model { .. } => "model_call",
            Ir::Len(_) => "len",
            Ir::Ok(_) => "ok",
            Ir::Err(_) => "err",
            Ir::Cmp(c, ..) => c.tag(),
            Ir::Arith(a, ..) => a.tag(),
            Ir::Neg(..) => "neg",
            Ir::Logic(l, ..) => l.tag(),
            Ir::Not(_) => "not",
            Ir::If(..) => "if",
            Ir::Concat(..) => "concat",
            Ir::Take(..) => "take",
            Ir::Drop(..) => "drop",
            Ir::IsErr(_) => "is_err",
            Ir::Fixed { tag, .. } => tag,
        }
    }

    /// The operand sub-trees in evaluation order (for a call, its arguments).
    pub fn children(&self) -> Vec<&Ir> {
        match self {
            Ir::Bool(_) | Ir::Int { .. } | Ir::Bytes(_) | Ir::Var(_) | Ir::Err(_) => vec![],
            Ir::Subject { args, .. } | Ir::Model { args, .. } | Ir::Fixed { args, .. } => {
                args.iter().collect()
            }
            Ir::Len(a) | Ir::Ok(a) | Ir::Not(a) | Ir::IsErr(a) | Ir::Neg(_, a) => vec![a],
            Ir::Cmp(_, a, b)
            | Ir::Arith(_, _, a, b)
            | Ir::Logic(_, a, b)
            | Ir::Concat(a, b)
            | Ir::Take(a, b)
            | Ir::Drop(a, b) => vec![a, b],
            Ir::If(c, t, e) => vec![c, t, e],
        }
    }

    /// Visit every node, parent before children, children left to right.  Iterative,
    /// so it is safe on any tree.
    pub fn walk(&self, f: &mut impl FnMut(&Ir)) {
        let mut stack = vec![self];
        while let Some(node) = stack.pop() {
            f(node);
            let mut kids = node.children();
            kids.reverse();
            stack.extend(kids);
        }
    }

    /// Does the tree contain a `subject_op` node?
    pub fn has_subject(&self) -> bool {
        let mut found = false;
        self.walk(&mut |n| found |= matches!(n, Ir::Subject { .. }));
        found
    }

    /// The names of the operations the tree calls, in order of first occurrence
    /// (pre-order, arguments left to right).
    pub fn subject_ops(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        self.walk(&mut |n| {
            if let Ir::Subject { op, .. } = n {
                if !out.contains(op) {
                    out.push(op.clone());
                }
            }
        });
        out
    }

    /// The nesting depth (a leaf is 1).  Iterative.
    pub fn depth(&self) -> usize {
        let mut best = 0;
        let mut stack = vec![(self, 1usize)];
        while let Some((node, d)) = stack.pop() {
            best = best.max(d);
            for kid in node.children() {
                stack.push((kid, d + 1));
            }
        }
        best
    }

    /// The JSON array form of Appendix A.
    pub fn to_json(&self) -> Json {
        let tag = || Json::str(self.tag());
        match self {
            Ir::Bool(b) => Json::arr([tag(), Json::Bool(*b)]),
            Ir::Int { ty, value } => Json::arr([
                tag(),
                Json::obj([
                    ("type", Json::str(ty.name())),
                    ("value", Json::str(value.to_string())),
                ]),
            ]),
            Ir::Bytes(hex) => Json::arr([
                tag(),
                Json::obj([
                    ("type", Json::str("Bytes")),
                    ("value", Json::str(hex.as_str())),
                ]),
            ]),
            Ir::Var(name) => Json::arr([tag(), Json::str(name.as_str())]),
            Ir::Subject { op, args, widths } => {
                let mut items = vec![
                    tag(),
                    Json::str(op.as_str()),
                    Json::arr(args.iter().map(Ir::to_json)),
                ];
                if !widths.is_empty() {
                    items.push(Json::arr(widths.iter().map(|w| Json::Int(*w))));
                }
                Json::Arr(items)
            }
            Ir::Model { name, args } => Json::arr([
                tag(),
                Json::str(name.as_str()),
                Json::arr(args.iter().map(Ir::to_json)),
            ]),
            Ir::Err(case) => Json::arr([tag(), Json::str(case.as_str())]),
            Ir::Fixed { ty, args, .. } => {
                let mut items = vec![tag()];
                items.extend(ty.map(|t| Json::str(t.name())));
                items.extend(args.iter().map(Ir::to_json));
                Json::Arr(items)
            }
            other => {
                let mut items = vec![tag()];
                items.extend(other.children().into_iter().map(Ir::to_json));
                Json::Arr(items)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_warrant_grammar::fixed::FixedTy;

    fn var(n: &str) -> Box<Ir> {
        Box::new(Ir::Var(n.into()))
    }

    /// One node of every one of the 26 tags.
    fn one_of_each() -> Vec<Ir> {
        vec![
            Ir::Bool(true),
            Ir::Var("a".into()),
            Ir::Subject {
                op: "f".into(),
                args: vec![],
                widths: vec![],
            },
            Ir::Model {
                name: "m".into(),
                args: vec![],
            },
            Ir::Len(var("a")),
            Ir::Ok(var("a")),
            Ir::Err("CASE".into()),
            Ir::Cmp(Cmp::Eq, var("a"), var("b")),
            Ir::Cmp(Cmp::Ne, var("a"), var("b")),
            Ir::Cmp(Cmp::Lt, var("a"), var("b")),
            Ir::Cmp(Cmp::Gt, var("a"), var("b")),
            Ir::Cmp(Cmp::Le, var("a"), var("b")),
            Ir::Cmp(Cmp::Ge, var("a"), var("b")),
            Ir::Arith(Arith::Add, Ty::Nat, var("a"), var("b")),
            Ir::Arith(Arith::Sub, Ty::Nat, var("a"), var("b")),
            Ir::Arith(Arith::Mul, Ty::Nat, var("a"), var("b")),
            Ir::Logic(Logic::And, var("a"), var("b")),
            Ir::Logic(Logic::Or, var("a"), var("b")),
            Ir::Not(var("a")),
            Ir::Logic(Logic::Implies, var("a"), var("b")),
            Ir::If(var("a"), var("b"), var("c")),
            Ir::Neg(Ty::Int, var("a")),
            Ir::Concat(var("a"), var("b")),
            Ir::Take(var("a"), var("b")),
            Ir::Drop(var("a"), var("b")),
            Ir::IsErr(var("a")),
        ]
    }

    #[test]
    fn the_enum_has_exactly_the_26_tags_of_appendix_a() {
        let tags: Vec<&str> = one_of_each().iter().map(Ir::tag).collect();
        assert_eq!(tags, ALL_TAGS.to_vec());
        let mut dedup = tags.clone();
        dedup.sort_unstable();
        dedup.dedup();
        assert_eq!(dedup.len(), 26);
        // And the three literal forms share one tag.
        assert_eq!(
            Ir::Int {
                ty: Ty::Nat,
                value: 1
            }
            .tag(),
            "lit"
        );
        assert_eq!(Ir::Bytes(String::new()).tag(), "lit");
    }

    #[test]
    fn the_fixed_variant_has_the_35_tags_and_the_appendix_json_shape() {
        let mut sorted = FIXED_TAGS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 35);
        assert!(FIXED_TAGS.iter().all(|t| !ALL_TAGS.contains(t)));
        assert!(OPTION_PRODUCERS.iter().all(|t| FIXED_TAGS.contains(t)));
        assert!(STATIC_TYPE_TAGS.iter().all(|t| FIXED_TAGS.contains(t)));
        assert_eq!(fixed_tag("rotl"), Some("rotl"));
        assert_eq!(fixed_tag("try_nat"), None);
        let arities: usize = FIXED_TAGS.iter().map(|t| fixed_arity(t)).sum();
        // 2 nullary, 13 unary (counting the static ones), 20 binary
        assert_eq!(arities, 13 + 2 * 20);
        let j = |ir: &Ir| {
            ir.to_json()
                .render()
                .unwrap()
                .split_whitespace()
                .collect::<String>()
        };
        let widen = Ir::Fixed {
            tag: "widen",
            ty: Some(FixedTy::U32),
            args: vec![Ir::Var("a".into())],
        };
        assert_eq!(j(&widen), r#"["widen","U32",["var","a"]]"#);
        let max = Ir::Fixed {
            tag: "max_value",
            ty: Some(FixedTy::I8),
            args: vec![],
        };
        assert_eq!(j(&max), r#"["max_value","I8"]"#);
        let rot = Ir::Fixed {
            tag: "rotl",
            ty: None,
            args: vec![Ir::Var("a".into()), Ir::Var("n".into())],
        };
        assert_eq!(j(&rot), r#"["rotl",["var","a"],["var","n"]]"#);
        assert_eq!(rot.tag(), "rotl");
        assert_eq!(rot.children().len(), 2);
        assert_eq!(max.children().len(), 0);
    }

    #[test]
    fn levels_is_core_plus_fixed_plus_the_callees_own_requires() {
        let none = |_: &str| vec!["ir.core"];
        let fixed_callee = |n: &str| {
            if n == "g" {
                vec!["ir.core", "ir.fixed"]
            } else {
                vec!["ir.core"]
            }
        };
        let a = Ir::Var("a".into());
        assert_eq!(levels([&a], &none), ["ir.core"]);
        let call = |n: &str| Ir::Model {
            name: n.into(),
            args: vec![Ir::Var("a".into())],
        };
        assert_eq!(levels([&call("f")], &fixed_callee), ["ir.core"]);
        assert_eq!(levels([&call("g")], &fixed_callee), ["ir.core", "ir.fixed"]);
        let direct = Ir::Fixed {
            tag: "bit_not",
            ty: None,
            args: vec![Ir::Var("a".into())],
        };
        assert_eq!(levels([&direct, &a], &none), ["ir.core", "ir.fixed"]);
        assert_eq!(levels(std::iter::empty(), &none), ["ir.core"]);
    }

    #[test]
    fn json_forms_match_the_v1_array_encoding() {
        let j = |ir: &Ir| {
            ir.to_json()
                .render()
                .unwrap()
                .split_whitespace()
                .collect::<String>()
        };
        assert_eq!(j(&Ir::Bool(true)), r#"["lit",true]"#);
        assert_eq!(
            j(&Ir::Int {
                ty: Ty::Fixed(FixedTy::U64),
                value: 7
            }),
            r#"["lit",{"type":"U64","value":"7"}]"#
        );
        assert_eq!(
            j(&Ir::Int {
                ty: Ty::Int,
                value: -3
            }),
            r#"["lit",{"type":"Int","value":"-3"}]"#
        );
        assert_eq!(
            j(&Ir::Bytes("00ff".into())),
            r#"["lit",{"type":"Bytes","value":"00ff"}]"#
        );
        assert_eq!(j(&Ir::Var("a".into())), r#"["var","a"]"#);
        let call = Ir::Subject {
            op: "eq_fixed".into(),
            args: vec![Ir::Var("a".into()), Ir::Var("b".into())],
            widths: vec![4],
        };
        assert_eq!(
            j(&call),
            r#"["subject_op","eq_fixed",[["var","a"],["var","b"]],[4]]"#
        );
        let plain = Ir::Subject {
            op: "eq".into(),
            args: vec![Ir::Var("a".into())],
            widths: vec![],
        };
        assert_eq!(j(&plain), r#"["subject_op","eq",[["var","a"]]]"#);
        let m = Ir::Model {
            name: "same".into(),
            args: vec![Ir::Var("a".into())],
        };
        assert_eq!(j(&m), r#"["model_call","same",[["var","a"]]]"#);
        assert_eq!(j(&Ir::Err("X".into())), r#"["err","X"]"#);
        assert_eq!(
            j(&Ir::If(var("c"), var("t"), var("e"))),
            r#"["if",["var","c"],["var","t"],["var","e"]]"#
        );
        assert_eq!(j(&Ir::Neg(Ty::Int, var("a"))), r#"["neg",["var","a"]]"#);
        assert_eq!(
            j(&Ir::Take(var("a"), var("n"))),
            r#"["take",["var","a"],["var","n"]]"#
        );
    }

    #[test]
    fn the_arith_type_annotation_is_not_written() {
        let a = Ir::Arith(Arith::Add, Ty::Fixed(FixedTy::U8), var("a"), var("b"));
        let b = Ir::Arith(Arith::Add, Ty::Nat, var("a"), var("b"));
        assert_eq!(a.to_json(), b.to_json());
        assert_ne!(a, b);
    }

    #[test]
    fn walk_is_preorder_left_to_right_and_finds_subjects() {
        let tree = Ir::Logic(
            Logic::And,
            Box::new(Ir::Subject {
                op: "f".into(),
                args: vec![Ir::Subject {
                    op: "g".into(),
                    args: vec![],
                    widths: vec![],
                }],
                widths: vec![],
            }),
            Box::new(Ir::Subject {
                op: "f".into(),
                args: vec![],
                widths: vec![],
            }),
        );
        let mut tags = vec![];
        tree.walk(&mut |n| tags.push(n.tag()));
        assert_eq!(tags, ["and", "subject_op", "subject_op", "subject_op"]);
        assert!(tree.has_subject());
        assert_eq!(tree.subject_ops(), ["f", "g"]);
        assert!(!Ir::Model {
            name: "m".into(),
            args: vec![Ir::Var("a".into())]
        }
        .has_subject());
    }

    #[test]
    fn depth_counts_levels_iteratively() {
        assert_eq!(Ir::Bool(true).depth(), 1);
        let mut ir = Ir::Var("a".into());
        for _ in 0..200 {
            ir = Ir::Not(Box::new(ir));
        }
        assert_eq!(ir.depth(), 201);
        let shallow = Ir::If(var("c"), Box::new(Ir::Not(var("t"))), var("e"));
        assert_eq!(shallow.depth(), 3);
    }

    #[test]
    fn children_are_in_evaluation_order() {
        let n = Ir::If(var("c"), var("t"), var("e"));
        let names: Vec<String> = n
            .children()
            .iter()
            .map(|k| match k {
                Ir::Var(v) => v.clone(),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(names, ["c", "t", "e"]);
        assert!(Ir::Bool(false).children().is_empty());
        assert_eq!(Ir::t(), Ir::Bool(true));
    }
}
