//! # The `panic` templates of `evidence.bindings` against the real panics (VER33 ADL-002b).
//!
//! Each of the eleven scalar gates `realizes=logic_gates_model.<gate>` and declares, with
//! `panic <gate> <template>` lines, the panic messages that count as the modelled
//! rejection `Err(INVALID_INPUT)`.  A template is the message with `{}` where the
//! formatted value goes.  This test makes the declaration honest in both directions:
//!
//! * **sound**: every panic a gate actually raises, for any non-bit in any argument
//!   position, matches one of that gate's declared templates, so no real rejection is
//!   undeclared; and no call panics with anything else (an `unwrap`, an overflow, an
//!   index would carry a different message and fail here);
//! * **exact**: every declared template is raised by at least one call, so nothing is
//!   declared that cannot happen (`nand_or` can only ever name `a`: `nand_not(b)`
//!   forwards to `nand_gate(b, b)`, whose first check names its argument `a`).
//!
//! The templates are read from the binding file as plain lines (`panic OP TEMPLATE`, the
//! rest of the line is the template).  The warrant-grammar test
//! `tests/logic_gates_realizes.rs` validates the same file with the real binding parser
//! and ties the templates to the source of `validate_bit`.

use logic_gates::gates::{
    and_gate, nand_and, nand_gate, nand_not, nand_or, nand_xor, nor_gate, not_gate, or_gate,
    xnor_gate, xor_gate,
};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Once;

thread_local! {
    static EXPECTING: Cell<bool> = const { Cell::new(false) };
}

fn quiet() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !EXPECTING.with(Cell::get) {
                previous(info);
            }
        }));
    });
}

/// The panic message of `f`, or `None` if it returned.
fn panic_message(f: impl FnOnce() -> u8) -> Option<String> {
    quiet();
    EXPECTING.with(|e| e.set(true));
    let r = catch_unwind(AssertUnwindSafe(f));
    EXPECTING.with(|e| e.set(false));
    match r {
        Ok(_) => None,
        Err(p) => Some(
            p.downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "<non-string panic payload>".to_string()),
        ),
    }
}

/// `panic OP TEMPLATE` lines of the package's `evidence.bindings`, by op.
fn declared() -> BTreeMap<String, Vec<String>> {
    let path = format!("{}/evidence.bindings", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).expect("evidence.bindings");
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("panic ") {
            let (op, template) = rest.split_once(' ').expect("`panic OP TEMPLATE`");
            out.entry(op.to_string())
                .or_default()
                .push(template.to_string());
        }
    }
    out
}

/// Does `message` match `template`, with each `{}` standing for a run of decimal digits
/// (the formatted `u8`)?
fn matches(template: &str, message: &str) -> bool {
    let mut rest = message;
    let mut parts = template.split("{}").peekable();
    while let Some(lit) = parts.next() {
        let Some(after) = rest.strip_prefix(lit) else {
            return false;
        };
        rest = after;
        if parts.peek().is_some() {
            let digits = rest.chars().take_while(char::is_ascii_digit).count();
            if digits == 0 {
                return false;
            }
            rest = &rest[digits..];
        }
    }
    rest.is_empty()
}

type OneInput = fn(u8) -> u8;
type TwoInput = fn(u8, u8) -> u8;

/// Values that are not bits, and the two that are.
const NON_BITS: [u8; 4] = [2, 17, 128, 255];

#[test]
fn every_real_panic_matches_a_declared_template_and_every_template_is_raised() {
    let declared = declared();
    let one: [(&str, OneInput); 2] = [("not_gate", not_gate), ("nand_not", nand_not)];
    let two: [(&str, TwoInput); 9] = [
        ("and_gate", and_gate),
        ("or_gate", or_gate),
        ("xor_gate", xor_gate),
        ("nand_gate", nand_gate),
        ("nor_gate", nor_gate),
        ("xnor_gate", xnor_gate),
        ("nand_and", nand_and),
        ("nand_or", nand_or),
        ("nand_xor", nand_xor),
    ];

    let mut observed: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    let mut record = |op: &'static str, msg: Option<String>, what: String| {
        let msg = msg.unwrap_or_else(|| panic!("{what} returned instead of panicking"));
        let templates = declared
            .get(op)
            .unwrap_or_else(|| panic!("{op}: no panic template declared"));
        let hit: Vec<&String> = templates.iter().filter(|t| matches(t, &msg)).collect();
        assert_eq!(
            hit.len(),
            1,
            "{what} panicked with `{msg}`, matched by {} declared template(s) of {op}",
            hit.len()
        );
        observed.entry(op).or_default().insert(hit[0].clone());
    };

    for bad in NON_BITS {
        for (op, f) in one {
            record(op, panic_message(|| f(bad)), format!("{op}({bad})"));
        }
        for (op, f) in two {
            for good in [0u8, 1] {
                record(
                    op,
                    panic_message(|| f(bad, good)),
                    format!("{op}({bad},{good})"),
                );
                record(
                    op,
                    panic_message(|| f(good, bad)),
                    format!("{op}({good},{bad})"),
                );
            }
            record(
                op,
                panic_message(|| f(bad, bad)),
                format!("{op}({bad},{bad})"),
            );
        }
    }

    // Exact: nothing is declared that no call can raise, and nothing is missing.
    assert_eq!(declared.len(), 11, "eleven gates declare templates");
    for (op, templates) in &declared {
        let seen = observed.get(op.as_str()).cloned().unwrap_or_default();
        let want: BTreeSet<String> = templates.iter().cloned().collect();
        assert_eq!(seen, want, "{op}: declared templates versus those raised");
    }

    // The valid bits never panic (the declared templates are not a blanket excuse).
    for a in 0..=1u8 {
        for (_, f) in one {
            assert!(panic_message(|| f(a)).is_none());
        }
        for b in 0..=1u8 {
            for (_, f) in two {
                assert!(panic_message(|| f(a, b)).is_none());
            }
        }
    }
}

#[test]
fn the_template_matcher_is_exact() {
    assert!(matches(
        "a must be 0 or 1, got {}",
        "a must be 0 or 1, got 255"
    ));
    assert!(!matches(
        "a must be 0 or 1, got {}",
        "b must be 0 or 1, got 2"
    ));
    assert!(!matches(
        "a must be 0 or 1, got {}",
        "a must be 0 or 1, got "
    ));
    assert!(!matches(
        "a must be 0 or 1, got {}",
        "a must be 0 or 1, got 2 extra"
    ));
    assert!(!matches(
        "a must be 0 or 1, got {}",
        "attempt to add with overflow"
    ));
}
