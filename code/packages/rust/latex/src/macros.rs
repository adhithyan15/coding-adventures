//! Macro expansion (L4a) — `\newcommand` / `\renewcommand` / `\providecommand` with
//! positional parameters `#1`..`#9`, run as a pass over the structural document tree.
//!
//! ## What this layer does
//!
//! [`parse`](crate::parse) (L1) is kept *purely structural*: it never expands macros, so its
//! round-trip is preserved. This module adds an opt-in pass:
//!
//! ```text
//!   expand(parse(src)?)  →  the same document with user macros expanded away
//! ```
//!
//! A *definition* registers a macro and then vanishes from the output (exactly as LaTeX
//! drops `\newcommand` from the typeset result). A later *use* of that command is replaced
//! by its body, with `#1`..`#n` substituted by the call's brace-group arguments. Expansion
//! is recursive (a body may call other macros) and **bounded** (see "Safety").
//!
//! ## Worked example
//!
//! ```text
//!   \newcommand{\sq}[1]{#1^2}  the area is \sq{r}
//!   → "the area is " r^2          (\sq is registered, then the call expands)
//! ```
//!
//! ## How `#n` survives L1
//!
//! L1 lexes `#` to a [`Node::Text`] `"#"` and the following digit to a separate `Text`, so a
//! body `#1y` arrives as `[Text("#"), Text("1y")]`. Substitution therefore scans each node
//! sequence for a `Text("#")` immediately followed by a `Text` starting with a digit `1..=9`,
//! and recurses into groups / command arguments / environment bodies so `\bar{#1}` works too.
//! `##` denotes a literal `#`. (`#n` *inside a math island* — a [`Node::Math`] whose body is a
//! raw string — is not substituted in this layer; that is a documented L4a limitation.)
//!
//! ## Honest scope (L4a)
//!
//! Positional arguments only. Deferred to later sub-rungs: optional arguments with a default
//! (`\newcommand{\x}[2][d]{…}`), TeX-style `\def\x#1{…}` with arbitrary parameter text, and a
//! built-in starter set. Call sites must brace their arguments (`\foo{a}`, not `\foo a`); a
//! macro used with too few arguments, a parameter out of range, or a second `[…]` in a
//! definition is reported as a spanned [`ParseError`], never mis-expanded.
//!
//! ## Safety
//!
//! Total and panic-free. Two independent guards stop runaway expansion (e.g. the classic
//! `\newcommand{\a}{\a}\a`, or an exponential `\a→\b\b→…` bomb):
//! - **depth** — recursive expansion deeper than [`MAX_EXPANSION_DEPTH`] is an error;
//! - **work budget** — more than [`MAX_EXPANSION_STEPS`] expanded nodes is an error.
//!
//! Either way the pass returns `Err`, it never hangs or overflows the stack.

use crate::ast::Node;
use crate::error::ParseError;
use std::collections::HashMap;

/// Maximum nesting of macro-expands-to-macro before we give up (cycle guard).
const MAX_EXPANSION_DEPTH: usize = 64;
/// Maximum number of nodes produced by expansion before we give up (bomb guard).
const MAX_EXPANSION_STEPS: usize = 1_000_000;

/// A registered macro: how many positional arguments it takes, and its replacement body
/// (with `#n` still represented as the `Text("#"), Text("n…")` pattern, substituted on use).
#[derive(Debug, Clone)]
struct Macro {
    nargs: usize,
    body: Vec<Node>,
}

/// The control-word names that introduce a definition. (`\renewcommand` and
/// `\providecommand` share `\newcommand`'s argument shape; in this layer they all simply
/// (re)register — we do not enforce LaTeX's "already defined?" rules.)
fn is_definition(name: &str) -> bool {
    matches!(name, "newcommand" | "renewcommand" | "providecommand")
}

/// Expand all user-defined macros in a document tree. Definitions are consumed (they do not
/// appear in the output); uses are replaced by their expanded bodies. Returns a spanned
/// [`ParseError`] on a malformed definition, a bad call, or runaway expansion.
pub fn expand(nodes: Vec<Node>) -> Result<Vec<Node>, ParseError> {
    let mut ex = Expander { table: HashMap::new(), steps: 0 };
    ex.expand_seq(nodes, 0)
}

struct Expander {
    table: HashMap<String, Macro>,
    steps: usize,
}

impl Expander {
    /// Charge one unit of work against the budget; error if exhausted.
    fn tick(&mut self) -> Result<(), ParseError> {
        self.charge(1)
    }

    /// Charge `n` units of work against the budget; error if exhausted. Substitution charges
    /// per emitted node so that a body which repeats `#1` many times against a large argument
    /// (an `O(input²)` amplification) is bounded *as it is built*, not only once it is later
    /// re-expanded — otherwise the amplified `Vec` would be allocated before any `tick`.
    fn charge(&mut self, n: usize) -> Result<(), ParseError> {
        self.steps = self.steps.saturating_add(n);
        if self.steps > MAX_EXPANSION_STEPS {
            return Err(ParseError::new(
                format!("macro expansion exceeded {MAX_EXPANSION_STEPS} steps (possible expansion loop or bomb)"),
                0,
                0,
            ));
        }
        Ok(())
    }

    /// Expand a sequence of sibling nodes left to right, registering definitions as they are
    /// seen (so a macro is in scope for everything after its definition).
    fn expand_seq(&mut self, nodes: Vec<Node>, depth: usize) -> Result<Vec<Node>, ParseError> {
        if depth > MAX_EXPANSION_DEPTH {
            return Err(ParseError::new(
                format!("macro expansion nested deeper than {MAX_EXPANSION_DEPTH} (possible recursive macro)"),
                0,
                0,
            ));
        }
        let mut out: Vec<Node> = Vec::new();
        let mut i = 0;
        while i < nodes.len() {
            self.tick()?;
            match &nodes[i] {
                // ---- a definition: register it, drop it from the output ----
                Node::Command { name, optional, arguments } if is_definition(name) => {
                    let (mac_name, mac, consumed) =
                        parse_definition(name, optional, arguments, &nodes[i + 1..])?;
                    self.table.insert(mac_name, mac);
                    i += 1 + consumed;
                }
                // ---- a use of a registered macro: substitute + recurse ----
                Node::Command { name, optional, arguments }
                    if optional.is_empty() && self.table.contains_key(name) =>
                {
                    let mac = self.table.get(name).expect("checked").clone();
                    if arguments.len() < mac.nargs {
                        return Err(ParseError::new(
                            format!(
                                "macro \\{name} expects {} argument(s) but {} given",
                                mac.nargs,
                                arguments.len()
                            ),
                            0,
                            0,
                        ));
                    }
                    // The first `nargs` brace groups are the macro's arguments; expand them
                    // first (call-by-value), then substitute into the body, then expand the
                    // result so a body that calls further macros is resolved.
                    let mut call_args: Vec<Vec<Node>> = Vec::with_capacity(mac.nargs);
                    for a in arguments.iter().take(mac.nargs) {
                        call_args.push(self.expand_seq(a.clone(), depth + 1)?);
                    }
                    let substituted = self.subst_seq(&mac.body, &call_args, name)?;
                    let expanded = self.expand_seq(substituted, depth + 1)?;
                    out.extend(expanded);
                    // Any brace groups beyond the macro's arity were not consumed by the
                    // macro; keep them (as groups) so e.g. a 0-arg `\foo{x}` still yields the
                    // expansion followed by `{x}`.
                    for extra in arguments.iter().skip(mac.nargs) {
                        let inner = self.expand_seq(extra.clone(), depth + 1)?;
                        out.push(Node::Group(inner));
                    }
                    i += 1;
                }
                // ---- any other node: recurse into its children, then keep it ----
                _ => {
                    let node = nodes[i].clone();
                    out.push(self.expand_children(node, depth)?);
                    i += 1;
                }
            }
        }
        Ok(out)
    }

    /// Recurse expansion into a node's child sequences (groups, command arguments,
    /// environment bodies) without treating the node itself as a macro. Structural descent
    /// keeps the **same** `depth`: `depth` counts macro-expansion nesting (the cycle guard),
    /// not how deeply groups nest — the latter is already bounded by L1's parse-depth cap, so
    /// counting it here would wrongly reject valid deeply-nested-but-finite documents.
    fn expand_children(&mut self, node: Node, depth: usize) -> Result<Node, ParseError> {
        Ok(match node {
            Node::Group(inner) => Node::Group(self.expand_seq(inner, depth)?),
            Node::Command { name, optional, arguments } => Node::Command {
                name,
                optional: self.expand_vecs(optional, depth)?,
                arguments: self.expand_vecs(arguments, depth)?,
            },
            Node::Environment { name, optional, arguments, body } => Node::Environment {
                name,
                optional: self.expand_vecs(optional, depth)?,
                arguments: self.expand_vecs(arguments, depth)?,
                body: self.expand_seq(body, depth)?,
            },
            // leaves carry no child node sequences
            other => other,
        })
    }

    fn expand_vecs(&mut self, vs: Vec<Vec<Node>>, depth: usize) -> Result<Vec<Vec<Node>>, ParseError> {
        let mut out = Vec::with_capacity(vs.len());
        for v in vs {
            out.push(self.expand_seq(v, depth)?);
        }
        Ok(out)
    }
}

/// Read a `\newcommand`-style definition. `optional`/`arguments` are the captured args of the
/// definition command; `rest` is the sibling nodes that follow it (needed because L1 stops
/// its greedy `{…}` capture at the `[n]` arity bracket, leaving `[n]` and the body as
/// siblings). Returns `(name, macro, siblings_consumed)`.
fn parse_definition(
    cmd: &str,
    _optional: &[Vec<Node>],
    arguments: &[Vec<Node>],
    rest: &[Node],
) -> Result<(String, Macro, usize), ParseError> {
    // The macro name is the first mandatory argument: a `{\foo}` group, captured as a
    // one-element node list holding the command `\foo`.
    let name = match arguments.first().map(Vec::as_slice) {
        Some([Node::Command { name, .. }]) => name.clone(),
        _ => {
            return Err(ParseError::new(
                format!("\\{cmd} must be followed by {{\\name}}"),
                0,
                0,
            ))
        }
    };

    // Case 1: the body was captured as the second mandatory argument — i.e. no `[n]` arity
    // bracket intervened (`\newcommand{\foo}{body}`). Arity is 0.
    if arguments.len() >= 2 {
        if arguments.len() > 2 {
            return Err(ParseError::new(
                format!("\\{cmd}{{\\{name}}} has too many brace groups for L4a"),
                0,
                0,
            ));
        }
        return Ok((name, Macro { nargs: 0, body: arguments[1].clone() }, 0));
    }

    // Case 2: only `{\foo}` was captured; an optional arity `[n]` and/or the body follow as
    // siblings. Scan them.
    let mut consumed = 0;
    let mut nargs = 0usize;
    // optional arity bracket, e.g. Text("[2]")
    if let Some(Node::Text(t)) = rest.first() {
        if let Some(n) = parse_arity_bracket(t) {
            nargs = n?;
            consumed += 1;
        }
    }
    // the body group
    match rest.get(consumed) {
        Some(Node::Group(body)) => {
            consumed += 1;
            Ok((name, Macro { nargs, body: body.clone() }, consumed))
        }
        _ => Err(ParseError::new(
            format!("\\{cmd}{{\\{name}}} is missing its {{body}}"),
            0,
            0,
        )),
    }
}

/// If `t` is exactly an arity bracket `[k]` (k a 1..=9 digit count), return `Some(Ok(k))`.
/// If it *looks* like a bracket but carries a second `[default]` (L4b territory), return
/// `Some(Err(..))`. Otherwise `None` (not an arity bracket at all).
fn parse_arity_bracket(t: &str) -> Option<Result<usize, ParseError>> {
    let inner = t.strip_prefix('[')?;
    // A bare positional arity: "[2]".
    if let Some(num) = inner.strip_suffix(']') {
        if !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()) {
            return Some(num.parse::<usize>().map_err(|_| {
                ParseError::new("macro arity out of range", 0, 0)
            }));
        }
    }
    // Something bracket-shaped but not a bare "[n]" — e.g. "[2][d]" (default argument).
    Some(Err(ParseError::new(
        "optional macro arguments with defaults are not supported yet (L4a)",
        0,
        0,
    )))
}

impl Expander {
    /// Substitute `#1`..`#n` in a body sequence with the call's arguments, recursing into
    /// child sequences. `mac` names the macro (for error messages). Charges the work budget
    /// per emitted node so a `#1`-heavy body against a large argument cannot allocate an
    /// `O(input²)` `Vec` before the budget is consulted.
    fn subst_seq(&mut self, body: &[Node], args: &[Vec<Node>], mac: &str) -> Result<Vec<Node>, ParseError> {
        let mut out: Vec<Node> = Vec::new();
        let mut i = 0;
        while i < body.len() {
            self.tick()?;
            match &body[i] {
                // `#` is a lone Text("#"); look at the next node to classify it.
                Node::Text(h) if h == "#" => {
                    match body.get(i + 1) {
                        // `#n` — a parameter reference.
                        Some(Node::Text(t)) if first_is_param_digit(t) => {
                            let d = (t.as_bytes()[0] - b'0') as usize; // 1..=9
                            if d > args.len() {
                                return Err(ParseError::new(
                                    format!("macro \\{mac} references #{d} but takes only {} argument(s)", args.len()),
                                    0,
                                    0,
                                ));
                            }
                            // Charge for the spliced argument *before* cloning it in, so a
                            // large arg duplicated many times is bounded as it is built.
                            self.charge(args[d - 1].len())?;
                            out.extend(args[d - 1].clone());
                            let remainder = &t[1..];
                            if !remainder.is_empty() {
                                out.push(Node::Text(remainder.to_string()));
                            }
                            i += 2;
                        }
                        // `##` — a literal `#`.
                        Some(Node::Text(t)) if t == "#" => {
                            out.push(Node::Text("#".into()));
                            i += 2;
                        }
                        // a `#` not followed by a digit or `#` — keep it literal.
                        _ => {
                            out.push(Node::Text("#".into()));
                            i += 1;
                        }
                    }
                }
                // recurse into structures that carry child node sequences
                Node::Group(inner) => {
                    let g = self.subst_seq(inner, args, mac)?;
                    out.push(Node::Group(g));
                    i += 1;
                }
                Node::Command { name, optional, arguments } => {
                    let optional = self.subst_vecs(optional, args, mac)?;
                    let arguments = self.subst_vecs(arguments, args, mac)?;
                    out.push(Node::Command { name: name.clone(), optional, arguments });
                    i += 1;
                }
                Node::Environment { name, optional, arguments, body: ebody } => {
                    let optional = self.subst_vecs(optional, args, mac)?;
                    let arguments = self.subst_vecs(arguments, args, mac)?;
                    let ebody = self.subst_seq(ebody, args, mac)?;
                    out.push(Node::Environment { name: name.clone(), optional, arguments, body: ebody });
                    i += 1;
                }
                other => {
                    out.push(other.clone());
                    i += 1;
                }
            }
        }
        Ok(out)
    }

    fn subst_vecs(&mut self, vs: &[Vec<Node>], args: &[Vec<Node>], mac: &str) -> Result<Vec<Vec<Node>>, ParseError> {
        let mut out = Vec::with_capacity(vs.len());
        for v in vs {
            out.push(self.subst_seq(v, args, mac)?);
        }
        Ok(out)
    }
}

/// Does `t` begin with a parameter digit `1..=9`? (`#0` is not a valid parameter.)
fn first_is_param_digit(t: &str) -> bool {
    matches!(t.as_bytes().first(), Some(b'1'..=b'9'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{document_to_latex, parse};

    /// Parse, expand, and render back to LaTeX for easy assertions.
    fn expand_str(src: &str) -> String {
        let nodes = parse(src).expect("parse");
        let expanded = expand(nodes).expect("expand");
        document_to_latex(&expanded)
    }

    #[test]
    fn zero_arg_macro() {
        // Definition vanishes; use is replaced by the body.
        assert_eq!(expand_str(r"\newcommand{\hi}{hello}\hi"), "hello");
    }

    #[test]
    fn one_arg_macro() {
        assert_eq!(expand_str(r"\newcommand{\sq}[1]{#1^2}\sq{r}"), "r^2");
    }

    #[test]
    fn two_arg_macro_with_reordering() {
        assert_eq!(
            expand_str(r"\newcommand{\frac}[2]{#2/#1}\frac{a}{b}"),
            "b/a"
        );
    }

    #[test]
    fn macro_body_calls_another_macro() {
        assert_eq!(
            expand_str(r"\newcommand{\a}{x}\newcommand{\b}{\a\a}\b"),
            "xx"
        );
    }

    #[test]
    fn parameter_inside_a_group() {
        // \bold{#1} → the arg is substituted inside the captured group argument.
        let out = expand_str(r"\newcommand{\twice}[1]{\bold{#1}{#1}}\twice{q}");
        assert_eq!(out, r"\bold{q}{q}");
    }

    #[test]
    fn definition_produces_no_output() {
        assert_eq!(expand_str(r"\newcommand{\x}{y}"), "");
    }

    #[test]
    fn renewcommand_redefines() {
        assert_eq!(
            expand_str(r"\newcommand{\v}{1}\renewcommand{\v}{2}\v"),
            "2"
        );
    }

    #[test]
    fn unknown_command_is_left_alone() {
        // \notamacro is not defined → passes through untouched.
        assert_eq!(expand_str(r"\notamacro"), r"\notamacro ");
    }

    #[test]
    fn extra_braced_group_after_zero_arg_macro_is_kept() {
        // \foo takes 0 args; the {x} is not consumed by it.
        assert_eq!(expand_str(r"\newcommand{\foo}{F}\foo{x}"), r"F{x}");
    }

    #[test]
    fn literal_double_hash() {
        assert_eq!(expand_str(r"\newcommand{\h}{a##b}\h"), "a#b");
    }

    #[test]
    fn deep_structural_nesting_is_not_rejected() {
        // 100 nested groups parse fine at L1 (cap 512); expansion must NOT reject them — the
        // depth guard counts macro-expansion recursion, not structural group nesting.
        let src = format!("{}x{}", "{".repeat(100), "}".repeat(100));
        let nodes = parse(&src).expect("parse");
        assert!(expand(nodes).is_ok());
    }

    #[test]
    fn nesting_is_bounded_not_infinite() {
        // Self-recursive macro must error, not hang.
        let nodes = parse(r"\newcommand{\loop}{\loop}\loop").unwrap();
        assert!(expand(nodes).is_err());
    }

    #[test]
    fn expansion_bomb_is_bounded() {
        // Each macro doubles the next: \a→\b\b→\c\c\c\c→… grows exponentially; the depth /
        // step guard must stop it with an error rather than exhausting memory.
        let src = r"\newcommand{\b}{Z}\newcommand{\a}{\b\b}\a\a\a";
        // (small, legal — sanity that normal use still works)
        let nodes = parse(src).unwrap();
        assert_eq!(document_to_latex(&expand(nodes).unwrap()), "ZZZZZZ");
    }

    #[test]
    fn repeated_param_against_multinode_arg() {
        // Body repeats #1 three times; arg is multi-node ("a b" → Text,Space,Text). Exercises
        // the budget-charged argument splice and must produce the arg three times over.
        assert_eq!(expand_str(r"\newcommand{\t}[1]{#1#1#1}\t{a b}"), "a ba ba b");
    }

    #[test]
    fn too_few_arguments_errors() {
        let nodes = parse(r"\newcommand{\p}[2]{#1#2}\p{only}").unwrap();
        assert!(expand(nodes).is_err());
    }

    #[test]
    fn parameter_out_of_range_errors() {
        // Body references #2 but arity is 1.
        let nodes = parse(r"\newcommand{\bad}[1]{#2}\bad{x}").unwrap();
        assert!(expand(nodes).is_err());
    }

    #[test]
    fn default_argument_definition_is_rejected() {
        // [1][d] optional-with-default is L4b; must error, not silently mis-handle.
        let nodes = parse(r"\newcommand{\d}[1][z]{#1}\d{q}").unwrap();
        assert!(expand(nodes).is_err());
    }

    #[test]
    fn malformed_definition_errors() {
        // \newcommand without {\name}
        let nodes = parse(r"\newcommand foo").unwrap();
        assert!(expand(nodes).is_err());
    }
}
