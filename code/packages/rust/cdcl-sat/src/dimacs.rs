//! # DIMACS CNF: the lingua franca of SAT solvers
//!
//! Every SAT competition, benchmark library and solver since 1993 reads the
//! same plain-text format, named after the DIMACS centre that set the first
//! challenge:
//!
//! ```text
//!   c  This is a comment.
//!   c  (x1 ∨ ¬x3) ∧ (x2 ∨ x3 ∨ ¬x1)
//!   p cnf 3 2
//!   1 -3 0
//!   2 3 -1 0
//! ```
//!
//! - `p cnf <variables> <clauses>` is the header.
//! - Each clause is a list of non-zero integers ended by `0`. A positive
//!   integer `k` is variable `k`; a negative one is its negation.
//! - A clause may span several lines, and a line may hold several clauses.
//!
//! ## Strictness (VER09 §5)
//!
//! A proof is only as good as the formula it is about. A reader that
//! silently "fixes" a malformed file proves a property of a formula nobody
//! wrote. So this reader rejects anything doubtful, with the line number:
//!
//! | Input | Result |
//! |---|---|
//! | clause before the header | error |
//! | second header | error |
//! | literal `|k|` above the declared variable count | error |
//! | last clause missing its `0` | error |
//! | clause count differs from the header | error |
//! | header variable count above the caller's limit | error |
//! | a line holding only `%` | end of input (SATLIB files end this way) |
//!
//! The variable limit exists because the header is attacker-controlled when
//! the file is: `p cnf 4000000000 0` must not make anyone allocate 4 billion
//! entries.

use std::fmt;

use crate::lit::Lit;

/// The default cap on a header's variable count: 2^24 (about 16 million).
pub const DEFAULT_MAX_VARS: usize = 1 << 24;

/// A formula in conjunctive normal form: an AND of clauses, each an OR of
/// literals over variables `0..num_vars`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cnf {
    /// Number of variables. Every literal's variable index is below this.
    pub num_vars: usize,
    /// The clauses, in order.
    pub clauses: Vec<Vec<Lit>>,
}

impl Cnf {
    /// An empty formula (trivially satisfiable) over `num_vars` variables.
    pub fn new(num_vars: usize) -> Cnf {
        Cnf { num_vars, clauses: Vec::new() }
    }

    /// Append a clause, growing `num_vars` if the clause mentions a new
    /// variable.
    pub fn add_clause(&mut self, clause: &[Lit]) {
        if let Some(max) = clause.iter().map(|l| l.var().index()).max() {
            self.num_vars = self.num_vars.max(max + 1);
        }
        self.clauses.push(clause.to_vec());
    }

    /// Does `assignment` (one value per variable) make every clause true?
    ///
    /// This is the check every SAT answer is re-verified with: it shares no
    /// code with the solver.
    pub fn is_satisfied_by(&self, assignment: &[bool]) -> bool {
        self.clauses.iter().all(|c| {
            c.iter().any(|l| {
                assignment
                    .get(l.var().index())
                    .is_some_and(|&value| value != l.is_negated())
            })
        })
    }
}

/// What went wrong while reading DIMACS text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DimacsErrorKind {
    /// A clause appeared before the `p cnf` header.
    MissingHeader,
    /// A second `p` line.
    DuplicateHeader,
    /// The `p` line is not `p cnf <vars> <clauses>`.
    BadHeader(String),
    /// The header's variable count is above the caller's limit.
    TooManyVars {
        /// Declared count.
        declared: usize,
        /// The caller's limit.
        limit: usize,
    },
    /// A token is not an integer.
    BadLiteral(String),
    /// A literal's variable is above the declared variable count.
    VarOutOfRange {
        /// The literal as written.
        literal: i64,
        /// The declared variable count.
        num_vars: usize,
    },
    /// The input ended inside a clause (no terminating `0`).
    UnterminatedClause,
    /// The number of clauses differs from the header.
    ClauseCountMismatch {
        /// Header count.
        declared: usize,
        /// Clauses actually read.
        found: usize,
    },
}

/// A DIMACS reading error, with the 1-based line it was found on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimacsError {
    /// 1-based line number (the last line, for end-of-input errors).
    pub line: usize,
    /// What went wrong.
    pub kind: DimacsErrorKind,
}

impl fmt::Display for DimacsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DIMACS line {}: ", self.line)?;
        match &self.kind {
            DimacsErrorKind::MissingHeader => write!(f, "clause before the `p cnf` header"),
            DimacsErrorKind::DuplicateHeader => write!(f, "second `p` header"),
            DimacsErrorKind::BadHeader(why) => write!(f, "malformed header: {why}"),
            DimacsErrorKind::TooManyVars { declared, limit } => {
                write!(f, "header declares {declared} variables; the limit is {limit}")
            }
            DimacsErrorKind::BadLiteral(tok) => write!(f, "`{tok}` is not an integer literal"),
            DimacsErrorKind::VarOutOfRange { literal, num_vars } => {
                write!(f, "literal {literal} is outside the declared {num_vars} variables")
            }
            DimacsErrorKind::UnterminatedClause => write!(f, "last clause has no terminating 0"),
            DimacsErrorKind::ClauseCountMismatch { declared, found } => {
                write!(f, "header declares {declared} clauses but {found} were found")
            }
        }
    }
}

impl std::error::Error for DimacsError {}

/// Read DIMACS CNF text with the default variable limit.
pub fn parse_dimacs(text: &str) -> Result<Cnf, DimacsError> {
    parse_dimacs_with_limit(text, DEFAULT_MAX_VARS)
}

/// Read DIMACS CNF text, rejecting headers that declare more than
/// `max_vars` variables.
pub fn parse_dimacs_with_limit(text: &str, max_vars: usize) -> Result<Cnf, DimacsError> {
    let err = |line: usize, kind: DimacsErrorKind| DimacsError { line, kind };

    // (variables, clauses) from the header, once seen.
    let mut header: Option<(usize, usize)> = None;
    let mut clauses: Vec<Vec<Lit>> = Vec::new();
    let mut current: Vec<Lit> = Vec::new();
    let mut last_line = 0;

    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        last_line = line_no;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('c') {
            continue;
        }
        if line == "%" {
            break;
        }
        if line.starts_with('p') {
            if header.is_some() {
                return Err(err(line_no, DimacsErrorKind::DuplicateHeader));
            }
            header = Some(parse_header(line, max_vars).map_err(|k| err(line_no, k))?);
            continue;
        }
        let (num_vars, _) = header.ok_or_else(|| err(line_no, DimacsErrorKind::MissingHeader))?;
        for token in line.split_whitespace() {
            let value: i64 = token
                .parse()
                .map_err(|_| err(line_no, DimacsErrorKind::BadLiteral(token.to_owned())))?;
            if value == 0 {
                clauses.push(std::mem::take(&mut current));
                continue;
            }
            if value.unsigned_abs() > num_vars as u64 {
                return Err(err(line_no, DimacsErrorKind::VarOutOfRange { literal: value, num_vars }));
            }
            // In range and non-zero, so the conversion cannot fail.
            current.push(Lit::from_dimacs(value).expect("checked non-zero and in range"));
        }
    }

    let (num_vars, declared) =
        header.ok_or_else(|| err(last_line.max(1), DimacsErrorKind::MissingHeader))?;
    if !current.is_empty() {
        return Err(err(last_line, DimacsErrorKind::UnterminatedClause));
    }
    if clauses.len() != declared {
        return Err(err(
            last_line.max(1),
            DimacsErrorKind::ClauseCountMismatch { declared, found: clauses.len() },
        ));
    }
    Ok(Cnf { num_vars, clauses })
}

/// Parse `p cnf <vars> <clauses>`.
fn parse_header(line: &str, max_vars: usize) -> Result<(usize, usize), DimacsErrorKind> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() != 4 || fields[0] != "p" || fields[1] != "cnf" {
        return Err(DimacsErrorKind::BadHeader(format!("expected `p cnf <vars> <clauses>`, got `{line}`")));
    }
    let number = |s: &str, what: &str| {
        s.parse::<usize>()
            .map_err(|_| DimacsErrorKind::BadHeader(format!("{what} `{s}` is not a non-negative integer")))
    };
    let vars = number(fields[2], "variable count")?;
    let clauses = number(fields[3], "clause count")?;
    if vars > max_vars {
        return Err(DimacsErrorKind::TooManyVars { declared: vars, limit: max_vars });
    }
    Ok((vars, clauses))
}

/// Write a formula as DIMACS CNF text: a header, then one clause per line.
pub fn write_dimacs(cnf: &Cnf) -> String {
    let mut out = format!("p cnf {} {}\n", cnf.num_vars, cnf.clauses.len());
    for clause in &cnf.clauses {
        for lit in clause {
            out.push_str(&lit.to_string());
            out.push(' ');
        }
        out.push_str("0\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(d: i64) -> Lit {
        Lit::from_dimacs(d).unwrap()
    }

    #[test]
    fn reads_the_module_example() {
        let text = "c comment\nc another\np cnf 3 2\n1 -3 0\n2 3 -1 0\n";
        let cnf = parse_dimacs(text).unwrap();
        assert_eq!(cnf.num_vars, 3);
        assert_eq!(cnf.clauses, vec![vec![lit(1), lit(-3)], vec![lit(2), lit(3), lit(-1)]]);
    }

    #[test]
    fn clauses_may_span_and_share_lines() {
        let cnf = parse_dimacs("p cnf 3 3\n1 2\n -3 0 2 0\n\n  0\n").unwrap();
        assert_eq!(cnf.clauses, vec![vec![lit(1), lit(2), lit(-3)], vec![lit(2)], vec![]]);
    }

    #[test]
    fn percent_line_ends_input() {
        let cnf = parse_dimacs("p cnf 2 1\n1 2 0\n%\n0\ngarbage\n").unwrap();
        assert_eq!(cnf.clauses.len(), 1);
    }

    #[test]
    fn empty_formula() {
        let cnf = parse_dimacs("p cnf 0 0\n").unwrap();
        assert_eq!(cnf, Cnf::new(0));
    }

    #[test]
    fn every_error_kind() {
        let kind = |t: &str| parse_dimacs(t).unwrap_err().kind;
        assert_eq!(kind("1 2 0\n"), DimacsErrorKind::MissingHeader);
        assert_eq!(kind(""), DimacsErrorKind::MissingHeader);
        assert_eq!(kind("p cnf 1 0\np cnf 1 0\n"), DimacsErrorKind::DuplicateHeader);
        assert!(matches!(kind("p cnf 1\n"), DimacsErrorKind::BadHeader(_)));
        assert!(matches!(kind("p dnf 1 1\n"), DimacsErrorKind::BadHeader(_)));
        assert!(matches!(kind("p cnf x 1\n"), DimacsErrorKind::BadHeader(_)));
        assert!(matches!(kind("p cnf 1 -1\n"), DimacsErrorKind::BadHeader(_)));
        assert_eq!(kind("p cnf 1 1\n1 x 0\n"), DimacsErrorKind::BadLiteral("x".into()));
        assert_eq!(
            kind("p cnf 2 1\n1 -3 0\n"),
            DimacsErrorKind::VarOutOfRange { literal: -3, num_vars: 2 }
        );
        assert_eq!(
            kind("p cnf 2 1\n-9223372036854775808 0\n"),
            DimacsErrorKind::VarOutOfRange { literal: i64::MIN, num_vars: 2 }
        );
        assert_eq!(kind("p cnf 2 1\n1 2\n"), DimacsErrorKind::UnterminatedClause);
        assert_eq!(
            kind("p cnf 2 2\n1 2 0\n"),
            DimacsErrorKind::ClauseCountMismatch { declared: 2, found: 1 }
        );
    }

    #[test]
    fn variable_limit_guards_allocation() {
        let e = parse_dimacs_with_limit("p cnf 11 0\n", 10).unwrap_err();
        assert_eq!(e.kind, DimacsErrorKind::TooManyVars { declared: 11, limit: 10 });
        assert_eq!(e.line, 1);
        assert!(parse_dimacs("p cnf 4000000000 0\n").is_err());
        assert!(parse_dimacs_with_limit("p cnf 10 0\n", 10).is_ok());
    }

    #[test]
    fn errors_report_their_line() {
        let e = parse_dimacs("c x\np cnf 2 1\n\n1 7 0\n").unwrap_err();
        assert_eq!(e.line, 4);
        let shown = e.to_string();
        assert!(shown.starts_with("DIMACS line 4: "), "{shown}");
    }

    #[test]
    fn every_error_displays() {
        let samples = [
            "1 0\n",
            "p cnf 1 0\np cnf 1 0\n",
            "p cnf\n",
            "p cnf 1 1\nq 0\n",
            "p cnf 1 1\n2 0\n",
            "p cnf 1 1\n1\n",
            "p cnf 1 2\n1 0\n",
        ];
        for s in samples {
            let msg = parse_dimacs(s).unwrap_err().to_string();
            assert!(msg.contains("DIMACS line"), "{msg}");
        }
        let msg = parse_dimacs_with_limit("p cnf 5 0\n", 1).unwrap_err().to_string();
        assert!(msg.contains("limit is 1"), "{msg}");
    }

    #[test]
    fn write_then_read_round_trips() {
        let mut cnf = Cnf::new(0);
        cnf.add_clause(&[lit(1), lit(-4)]);
        cnf.add_clause(&[]);
        cnf.add_clause(&[lit(2)]);
        assert_eq!(cnf.num_vars, 4);
        let text = write_dimacs(&cnf);
        assert_eq!(text, "p cnf 4 3\n1 -4 0\n0\n2 0\n");
        assert_eq!(parse_dimacs(&text).unwrap(), cnf);
    }

    #[test]
    fn satisfaction_check() {
        let cnf = parse_dimacs("p cnf 2 2\n1 2 0\n-1 0\n").unwrap();
        assert!(cnf.is_satisfied_by(&[false, true]));
        assert!(!cnf.is_satisfied_by(&[true, true]));
        assert!(!cnf.is_satisfied_by(&[false])); // missing variable counts as unsatisfied
        let empty_clause = parse_dimacs("p cnf 1 1\n0\n").unwrap();
        assert!(!empty_clause.is_satisfied_by(&[true]));
    }
}
