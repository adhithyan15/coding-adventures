//! # `contract.vos`: the ceilings and configuration of a contract package
//!
//! A contract package owns three kinds of text (VOS02 R1): the Warrant contract block,
//! one obligations block per tier, and this file, the part that is not Warrant:
//!
//! ```text
//!   vos-contract 1
//!   name vos-sched-contract
//!   iface sched
//!   tier any
//!   tier canonical
//!   tier policy
//!   tier representation
//!   tier safety
//!   config N 8
//!   ceiling dispatch depth 2
//!   ceiling dispatch heap 0
//!   ceiling dispatch steps 160
//!   end
//! ```
//!
//! ## Reading it
//!
//! ```text
//!   bytes
//!     │ caps: 64 KiB, 1,024-byte lines, ASCII               (checked before lexing)
//!     ▼
//!   GrammarLexer      code/grammars/vos_contract/vos_contract.tokens
//!     ▼
//!   GrammarParser     code/grammars/vos_contract/vos_contract.grammar
//!     ▼
//!   rows (words only)
//!     │ post-parse checks the grammar cannot make:
//!     │   version is 1 · word classes · DEC fits u64 · sorted, unique · row caps
//!     ▼
//!   Contract ──► emit() ──► must equal the input byte for byte (ONE spelling)
//! ```
//!
//! The last step is the VOS02 section 6.1 discipline.  The grammar tolerates a doubled
//! space and a blank line (the lexer skips blanks, and an empty line is a newline
//! token); the re-emit comparison does not.  Two byte strings can therefore never
//! describe one contract, and the contract's digest is the digest of its bytes.

use lexer::grammar_lexer::GrammarLexer;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode, GrammarParser};

use crate::caps::{
    CONTRACT_MAX_BYTES, CONTRACT_MAX_LINE, MAX_CEILING_ROWS, MAX_CONFIG_ROWS, MAX_TIER_ROWS,
    NAME_MAX,
};
use crate::digest::sha256_hex;
use crate::grammar::{contract_grammar, contract_tokens, token_kind};

/// The five tiers of obligations a contract can define, in the order the file (and the
/// generated document) lists them: sorted by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// The most-general implementation's soundness and coverage (VOS02 section 3.3).
    Any,
    /// The reachable states are the image of `build` (VOS02 R5).
    Canonical,
    /// The fixed policy (for a scheduler, fixed priority).
    Policy,
    /// The reachable states are the image of an implementation-supplied `rep` (VOS02 section 2.7):
    /// the closure argument of an implementation whose state has history.
    Representation,
    /// Behaviour any implementation of the interface must have.
    Safety,
}

impl Tier {
    /// Every tier, in sorted order.
    pub const ALL: [Tier; 5] = [
        Tier::Any,
        Tier::Canonical,
        Tier::Policy,
        Tier::Representation,
        Tier::Safety,
    ];

    /// The spelling in `contract.vos`, in `vos.toml` and in a marker.
    pub fn name(self) -> &'static str {
        match self {
            Tier::Any => "any",
            Tier::Canonical => "canonical",
            Tier::Policy => "policy",
            Tier::Representation => "representation",
            Tier::Safety => "safety",
        }
    }

    /// The tier spelled `s`, or `None`.
    pub fn parse(s: &str) -> Option<Tier> {
        Tier::ALL.into_iter().find(|t| t.name() == s)
    }
}

/// A ceiling metric.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Metric {
    /// Call depth.
    Depth,
    /// Heap bytes.
    Heap,
    /// Instruction steps.
    Steps,
}

impl Metric {
    /// The spelling in the file.
    pub fn name(self) -> &'static str {
        match self {
            Metric::Depth => "depth",
            Metric::Heap => "heap",
            Metric::Steps => "steps",
        }
    }

    fn parse(s: &str) -> Option<Metric> {
        [Metric::Depth, Metric::Heap, Metric::Steps]
            .into_iter()
            .find(|m| m.name() == s)
    }
}

/// One `config KEY DEC` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// The configuration name (`N`).
    pub key: String,
    /// Its value.
    pub value: u64,
}

/// One `ceiling OPNAME METRIC DEC` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ceiling {
    /// The interface operation.
    pub op: String,
    /// What is bounded.
    pub metric: Metric,
    /// The bound.
    pub value: u64,
}

/// A parsed, canonical `contract.vos`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contract {
    /// The contract package (`vos-sched-contract`).
    pub package: String,
    /// The interface name (`sched`).
    pub iface: String,
    /// The tiers it defines, sorted and unique.
    pub tiers: Vec<Tier>,
    /// The configuration, sorted by key.
    pub config: Vec<Config>,
    /// The ceilings, sorted by `(op, metric)`.
    pub ceilings: Vec<Ceiling>,
}

/// What went wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractErrorKind {
    /// The file is over its byte cap.
    TooLarge,
    /// A line is over its cap.
    LineTooLong,
    /// A byte outside ASCII.
    NotAscii,
    /// The lexer refused a character (a tab, a CR, a symbol, ...).
    Lexical,
    /// The token stream does not match the grammar.
    Syntax,
    /// The version word is not 1.
    Version,
    /// A name is not of the class its row requires.
    BadName,
    /// A number does not fit `u64`.
    BadNumber,
    /// A `tier` row is not one of `any`, `canonical`, `policy`, `representation`, `safety`.
    BadTier,
    /// A `ceiling` metric is not `depth`, `heap` or `steps`.
    BadMetric,
    /// A row is out of sorted order.
    Unsorted,
    /// A row repeats an earlier one.
    Duplicate,
    /// More rows than the cap.
    TooManyRows,
    /// Parsed fine, but the text is not the canonical spelling of what it says.
    NotCanonical,
}

/// A `contract.vos` error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractError {
    /// What went wrong.
    pub kind: ContractErrorKind,
    /// 1-based line, or 1 when no line is meaningful.
    pub line: u32,
    /// Human-readable reason.
    pub message: String,
}

impl ContractError {
    /// True for a size cap (VOS02 section 6.8): the tool could not even look at the
    /// file, which is not the same as the file being wrong.
    pub fn is_cap(&self) -> bool {
        matches!(
            self.kind,
            ContractErrorKind::TooLarge | ContractErrorKind::LineTooLong
        )
    }
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "contract.vos:{}: {}", self.line, self.message)
    }
}

impl std::error::Error for ContractError {}

fn err<T>(
    kind: ContractErrorKind,
    line: usize,
    message: impl Into<String>,
) -> Result<T, ContractError> {
    Err(ContractError {
        kind,
        line: u32::try_from(line).unwrap_or(u32::MAX),
        message: message.into(),
    })
}

// ---------------------------------------------------------------------------
// Word classes (VOS02 section 6.2): a post-token check, because the class depends on
// the row the word is in.
// ---------------------------------------------------------------------------

fn class_ok(s: &str, first: fn(u8) -> bool, rest: fn(u8) -> bool) -> bool {
    let b = s.as_bytes();
    !b.is_empty() && b.len() <= NAME_MAX && first(b[0]) && b[1..].iter().all(|&c| rest(c))
}

/// `IDENT = [a-z][a-z0-9-]{0,31}`: package, interface, profile and slot names.
pub fn is_ident(s: &str) -> bool {
    class_ok(
        s,
        |c| c.is_ascii_lowercase(),
        |c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-',
    )
}

/// `OPNAME = [a-z][a-z0-9_]{0,31}`: an interface operation (VOS-D051).
pub fn is_opname(s: &str) -> bool {
    class_ok(
        s,
        |c| c.is_ascii_lowercase(),
        |c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_',
    )
}

/// `KEY = [A-Za-z][A-Za-z0-9_]{0,31}`: a configuration name.
pub fn is_key(s: &str) -> bool {
    class_ok(
        s,
        |c| c.is_ascii_alphabetic(),
        |c| c.is_ascii_alphanumeric() || c == b'_',
    )
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// One row: the words of a rule node (newline tokens dropped), with their lines.
struct Row {
    rule: String,
    words: Vec<(String, usize)>,
}

fn rows_of(root: &GrammarASTNode) -> Vec<Row> {
    let mut rows = Vec::new();
    for child in &root.children {
        if let ASTNodeOrToken::Node(node) = child {
            let mut words = Vec::new();
            for c in &node.children {
                if let ASTNodeOrToken::Token(t) = c {
                    if token_kind(t) != "NEWLINE" {
                        words.push((t.value.clone(), t.line));
                    }
                }
            }
            rows.push(Row {
                rule: node.rule_name.clone(),
                words,
            });
        }
    }
    rows
}

fn number(word: &(String, usize)) -> Result<u64, ContractError> {
    word.0.parse::<u64>().or_else(|_| {
        err(
            ContractErrorKind::BadNumber,
            word.1,
            format!("`{}` does not fit in a u64", word.0),
        )
    })
}

/// Parse a `contract.vos`.  Total: it returns a [`Contract`] or an error, and the
/// `Contract` it returns re-emits to exactly the bytes it came from.
pub fn parse_contract(bytes: &[u8]) -> Result<Contract, ContractError> {
    if bytes.len() > CONTRACT_MAX_BYTES {
        return err(
            ContractErrorKind::TooLarge,
            1,
            format!("over the {CONTRACT_MAX_BYTES}-byte cap"),
        );
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return err(ContractErrorKind::NotAscii, 1, "not UTF-8, so not ASCII");
    };
    if !text.is_ascii() {
        return err(ContractErrorKind::NotAscii, 1, "contract.vos is ASCII");
    }
    for (n, line) in text.split('\n').enumerate() {
        if line.len() > CONTRACT_MAX_LINE {
            return err(
                ContractErrorKind::LineTooLong,
                n + 1,
                format!("a line is over {CONTRACT_MAX_LINE} bytes"),
            );
        }
    }

    // The grammar pair does the lexing and the parsing.
    let tokens = match GrammarLexer::new(text, contract_tokens()).tokenize() {
        Ok(t) => t,
        Err(e) => return err(ContractErrorKind::Lexical, e.line, e.message),
    };
    let root = match GrammarParser::new(tokens, contract_grammar().clone()).parse() {
        Ok(r) => r,
        Err(e) => return err(ContractErrorKind::Syntax, e.token.line, e.message),
    };
    let rows = rows_of(&root);

    // Post-parse checks, row by row.
    let mut package = String::new();
    let mut iface = String::new();
    let mut tiers: Vec<Tier> = Vec::new();
    let mut config: Vec<Config> = Vec::new();
    let mut ceilings: Vec<Ceiling> = Vec::new();
    for row in &rows {
        let w = &row.words;
        match row.rule.as_str() {
            "header" => {
                if w[1].0 != "1" {
                    return err(ContractErrorKind::Version, w[1].1, "the version is 1");
                }
            }
            "name_row" => {
                if !is_ident(&w[1].0) {
                    return err(
                        ContractErrorKind::BadName,
                        w[1].1,
                        "a package name is [a-z][a-z0-9-]{0,31}",
                    );
                }
                package = w[1].0.clone();
            }
            "iface_row" => {
                if !is_ident(&w[1].0) {
                    return err(
                        ContractErrorKind::BadName,
                        w[1].1,
                        "an interface name is [a-z][a-z0-9-]{0,31}",
                    );
                }
                iface = w[1].0.clone();
            }
            "tier_row" => {
                let Some(t) = Tier::parse(&w[1].0) else {
                    return err(
                        ContractErrorKind::BadTier,
                        w[1].1,
                        "a tier is any, canonical, policy, representation or safety",
                    );
                };
                if tiers.len() >= MAX_TIER_ROWS {
                    return err(ContractErrorKind::TooManyRows, w[1].1, "more than 5 tiers");
                }
                if let Some(&last) = tiers.last() {
                    if t == last {
                        return err(ContractErrorKind::Duplicate, w[1].1, "a repeated tier");
                    }
                    if t < last {
                        return err(
                            ContractErrorKind::Unsorted,
                            w[1].1,
                            "tiers are sorted: any, canonical, policy, representation, safety",
                        );
                    }
                }
                tiers.push(t);
            }
            "config_row" => {
                if !is_key(&w[1].0) {
                    return err(
                        ContractErrorKind::BadName,
                        w[1].1,
                        "a config key is [A-Za-z][A-Za-z0-9_]{0,31}",
                    );
                }
                if config.len() >= MAX_CONFIG_ROWS {
                    return err(
                        ContractErrorKind::TooManyRows,
                        w[1].1,
                        "more than 64 config rows",
                    );
                }
                let value = number(&w[2])?;
                if let Some(last) = config.last() {
                    if w[1].0 == last.key {
                        return err(
                            ContractErrorKind::Duplicate,
                            w[1].1,
                            "a repeated config key",
                        );
                    }
                    if w[1].0 < last.key {
                        return err(
                            ContractErrorKind::Unsorted,
                            w[1].1,
                            "config rows are sorted by key",
                        );
                    }
                }
                config.push(Config {
                    key: w[1].0.clone(),
                    value,
                });
            }
            "ceiling_row" => {
                if !is_opname(&w[1].0) {
                    return err(
                        ContractErrorKind::BadName,
                        w[1].1,
                        "an operation name is [a-z][a-z0-9_]{0,31}",
                    );
                }
                let Some(metric) = Metric::parse(&w[2].0) else {
                    return err(
                        ContractErrorKind::BadMetric,
                        w[2].1,
                        "a metric is depth, heap or steps",
                    );
                };
                if ceilings.len() >= MAX_CEILING_ROWS {
                    return err(
                        ContractErrorKind::TooManyRows,
                        w[1].1,
                        "more than 1,024 ceiling rows",
                    );
                }
                let value = number(&w[3])?;
                if let Some(last) = ceilings.last() {
                    let key = (w[1].0.as_str(), metric);
                    let prev = (last.op.as_str(), last.metric);
                    if key == prev {
                        return err(ContractErrorKind::Duplicate, w[1].1, "a repeated ceiling");
                    }
                    if key < prev {
                        return err(
                            ContractErrorKind::Unsorted,
                            w[1].1,
                            "ceiling rows are sorted by (operation, metric)",
                        );
                    }
                }
                ceilings.push(Ceiling {
                    op: w[1].0.clone(),
                    metric,
                    value,
                });
            }
            _ => {} // end_row: nothing to check
        }
    }

    let contract = Contract {
        package,
        iface,
        tiers,
        config,
        ceilings,
    };

    // ONE spelling: the canonical text must be the input.
    let canon = contract.emit();
    if canon != text {
        let line = first_difference(&canon, text);
        return err(
            ContractErrorKind::NotCanonical,
            line,
            "not the canonical spelling (one space between words, no blank line, rows in order, one final LF)",
        );
    }
    Ok(contract)
}

/// The 1-based line of the first difference between two texts.
fn first_difference(a: &str, b: &str) -> usize {
    let mut la = a.split('\n');
    let mut lb = b.split('\n');
    let mut n = 1;
    loop {
        match (la.next(), lb.next()) {
            (Some(x), Some(y)) if x == y => n += 1,
            _ => return n,
        }
    }
}

impl Contract {
    /// The canonical text: the single spelling of this contract.
    pub fn emit(&self) -> String {
        let mut s = String::from("vos-contract 1\n");
        s.push_str(&format!("name {}\n", self.package));
        s.push_str(&format!("iface {}\n", self.iface));
        for t in &self.tiers {
            s.push_str(&format!("tier {}\n", t.name()));
        }
        for c in &self.config {
            s.push_str(&format!("config {} {}\n", c.key, c.value));
        }
        for c in &self.ceilings {
            s.push_str(&format!(
                "ceiling {} {} {}\n",
                c.op,
                c.metric.name(),
                c.value
            ));
        }
        s.push_str("end\n");
        s
    }

    /// SHA-256 of the canonical text (the `ceilings-sha256` and `contract-sha256` of
    /// the lock, VOS02 section 6.3, are built from it).
    pub fn digest(&self) -> String {
        sha256_hex(self.emit().as_bytes())
    }

    /// True when the contract defines `tier`.
    pub fn has_tier(&self, tier: Tier) -> bool {
        self.tiers.contains(&tier)
    }

    /// The value of a configuration key.
    pub fn config_value(&self, key: &str) -> Option<u64> {
        self.config.iter().find(|c| c.key == key).map(|c| c.value)
    }
}
