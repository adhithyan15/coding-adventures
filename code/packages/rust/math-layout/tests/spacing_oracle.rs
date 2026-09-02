//! Every inter-atom space, checked against a real TeX.
//!
//! `fixtures/tex-spacing.txt` was produced by asking `tex` to typeset all 256
//! ordered class pairs in all four styles and reading back the glue node it
//! inserted. TeX names the parameter it used — `\glue(\medmuskip)` — so
//! nothing in the fixture is a measured width, a unit conversion, or a
//! threshold of ours deciding what counts as "thin".
//!
//! That matters more than usual here. The spacing table is exactly the kind of
//! grid where a transposed row produces output that looks *almost* right, and
//! a hand-written fixture would have been transcribed from the same book by
//! the same person as the implementation — so both would be wrong together and
//! agree perfectly.
//!
//! ## What is really being tested
//!
//! Not just the table. The fixture was generated from the arrangement
//!
//! ```text
//!     Ord  L  R  Ord
//! ```
//!
//! so a `Bin` under test is neither first nor last, and TeX applied its own
//! demotion rules before choosing any glue. Reproducing that same arrangement
//! here means the **demotion rules are under test too** — including the cells
//! the TeXbook marks impossible, which are reachable only through a demotion
//! and where the book gives no value to copy.
//!
//! Regenerate with `python3 code/scripts/extract_tex_spacing_table.py`.

use math_layout::{Atom, AtomClass, MathList, Space, Style};

const ORACLE: &str = include_str!("fixtures/tex-spacing.txt");

fn class_by_name(name: &str) -> AtomClass {
    AtomClass::ALL
        .into_iter()
        .find(|class| class.name() == name)
        .unwrap_or_else(|| panic!("unknown atom class in fixture: {name}"))
}

fn style_by_name(name: &str) -> Style {
    Style::ALL
        .into_iter()
        .find(|style| style.name() == name)
        .unwrap_or_else(|| panic!("unknown style in fixture: {name}"))
}

/// The space our implementation puts between `left` and `right`, in the same
/// padded arrangement the fixture was generated from.
fn spacing_in_context(left: AtomClass, right: AtomClass, style: Style) -> Space {
    let list = MathList::new(vec![
        Atom::symbol(AtomClass::Ord, "a"),
        Atom::symbol(left, "b"),
        Atom::symbol(right, "c"),
        Atom::symbol(AtomClass::Ord, "d"),
    ]);
    // `spacings()[i]` is the space PRECEDING atom i, so index 2 is the gap
    // between the two atoms under test.
    list.spacings(style)[2]
}

#[test]
fn every_pair_matches_a_real_tex() {
    let mut checked = 0usize;
    let mut mismatches = Vec::new();

    for line in ORACLE.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(fields.len(), 4, "malformed fixture line: {line}");

        let style = style_by_name(fields[0]);
        let left = class_by_name(fields[1]);
        let right = class_by_name(fields[2]);
        let expected = fields[3];

        let actual = spacing_in_context(left, right, style).name();
        if actual != expected {
            mismatches.push(format!(
                "  {} {} {}: tex says {expected}, we say {actual}",
                fields[0], fields[1], fields[2]
            ));
        }
        checked += 1;
    }

    assert!(
        mismatches.is_empty(),
        "{} of {checked} pairs disagree with TeX:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
    // Guards against the fixture silently emptying out and this passing by
    // checking nothing: 8 classes squared, in 4 styles.
    assert_eq!(checked, 256, "expected the full table");
}

/// The fixture must actually exercise all four spaces, or a table of zeroes
/// would satisfy the test above.
#[test]
fn the_fixture_contains_every_kind_of_space() {
    let mut seen = std::collections::HashSet::new();
    for line in ORACLE.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(space) = line.split_whitespace().nth(3) {
            seen.insert(space.to_string());
        }
    }
    for expected in ["none", "thin", "med", "thick"] {
        assert!(
            seen.contains(expected),
            "fixture never exercises {expected}"
        );
    }
}

/// The cases the TeXbook prints as a table, spelled out.
///
/// The oracle above proves agreement with TeX; these say *what* is being
/// agreed to, so a future reader can see the shape without running `tex`.
#[test]
fn the_documented_row_for_ordinary_atoms_reads_as_published() {
    use AtomClass::*;
    let expected = [
        (Ord, Space::None),
        (Op, Space::Thin),
        (Bin, Space::Med),
        (Rel, Space::Thick),
        (Open, Space::None),
        (Close, Space::None),
        (Punct, Space::None),
        (Inner, Space::Thin),
    ];
    for (right, want) in expected {
        assert_eq!(
            spacing_in_context(Ord, right, Style::Text),
            want,
            "Ord followed by {:?}",
            right
        );
    }
}
