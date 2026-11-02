//! # Bounded diagnostics (#18474 security review)
//!
//! A type checker reports one diagnostic per problem, and a hostile program
//! can contain a great many problems.  Worse, some diagnostics describe a
//! *type*, not a spot in the source.  The exhaustiveness check listed every
//! unmatched variant of a union, so each zero-arm `(match x)` on a 60,000-
//! variant union formatted about 600 KB, and 50,000 such matches asked for
//! about 33 GB of messages.  Each match costs a few bytes of source; its
//! message cost the size of the union.
//!
//! Three limits bound what a checker built on this protocol can accumulate:
//!
//! | Limit | Value | Why |
//! |-------|-------|-----|
//! | [`MAX_DIAGNOSTICS`] | 256 | no one reads past the first screens of errors; GCC's `-fmax-errors` and many IDEs stop much earlier |
//! | [`MAX_DIAGNOSTIC_MESSAGE_BYTES`] | 4 KiB | a message names at most a few identifiers (each capped at 256 bytes by the Twig parser) plus [`MAX_LISTED_NAMES`] list items |
//! | [`MAX_DIAGNOSTIC_BYTES`] | 1 MiB | all messages together: 256 × 4 KiB, enforced on its own so no message length can break it |
//!
//! When a diagnostic does not fit, it is dropped and a single
//! [`DIAGNOSTICS_TRUNCATED`] note is appended (once), so the result still
//! says that more errors exist.  [`diagnostics_closed`] (the note is
//! there) lets a checker skip the *work* of building a diagnostic nobody
//! will keep.
//!
//! [`name_list`] formats a list of names for a message: the first
//! [`MAX_LISTED_NAMES`], then "… and K more".

use crate::TypeErrorDiagnostic;

/// Diagnostics a checker keeps per program, before the truncation note.
pub const MAX_DIAGNOSTICS: usize = 256;

/// Bytes in one diagnostic message; a longer one is cut at a character
/// boundary and ends in `…`.
pub const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 4096;

/// Bytes of all diagnostic messages a checker keeps per program.
pub const MAX_DIAGNOSTIC_BYTES: usize = 1 << 20;

/// Names a message lists before summarising the rest.
pub const MAX_LISTED_NAMES: usize = 8;

/// The note appended, once, when a diagnostic is dropped.
pub const DIAGNOSTICS_TRUNCATED: &str =
    "too many type errors: later diagnostics were not reported";

/// Is there room for no more diagnostics?  True once the truncation note
/// is present or the count limit is reached.  A checker asks this before
/// doing work whose only product is a diagnostic.
pub fn diagnostics_full(errors: &[TypeErrorDiagnostic]) -> bool {
    errors.len() >= MAX_DIAGNOSTICS
        || errors.last().is_some_and(|d| d.message == DIAGNOSTICS_TRUNCATED)
}

/// Has a diagnostic already been dropped (the [`DIAGNOSTICS_TRUNCATED`]
/// note is present)?  From then on nothing more is kept, so a checker can
/// skip building diagnostics.  (While merely [`diagnostics_full`], the next
/// diagnostic must still be built and pushed, so the note records it.)
pub fn diagnostics_closed(errors: &[TypeErrorDiagnostic]) -> bool {
    errors.last().is_some_and(|d| d.message == DIAGNOSTICS_TRUNCATED)
}

/// Push `diagnostic` within the limits above.  Returns whether it was kept.
///
/// Its message is first cut to [`MAX_DIAGNOSTIC_MESSAGE_BYTES`].  If it
/// still does not fit (count or total bytes), it is dropped and the
/// [`DIAGNOSTICS_TRUNCATED`] note is appended if it is not there yet, at
/// the dropped diagnostic's position.  The total is re-summed on each push,
/// at most [`MAX_DIAGNOSTICS`] messages.
pub fn push_bounded(errors: &mut Vec<TypeErrorDiagnostic>, mut diagnostic: TypeErrorDiagnostic) -> bool {
    truncate_message(&mut diagnostic.message);
    let bytes: usize = errors.iter().map(|d| d.message.len()).sum();
    if diagnostics_full(errors) || bytes + diagnostic.message.len() > MAX_DIAGNOSTIC_BYTES {
        if errors.last().is_none_or(|d| d.message != DIAGNOSTICS_TRUNCATED) {
            errors.push(TypeErrorDiagnostic {
                message: DIAGNOSTICS_TRUNCATED.to_string(),
                line: diagnostic.line,
                column: diagnostic.column,
            });
        }
        return false;
    }
    errors.push(diagnostic);
    true
}

fn truncate_message(message: &mut String) {
    const ELLIPSIS: &str = "…";
    if message.len() <= MAX_DIAGNOSTIC_MESSAGE_BYTES {
        return;
    }
    let mut end = MAX_DIAGNOSTIC_MESSAGE_BYTES - ELLIPSIS.len();
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message.truncate(end);
    message.push_str(ELLIPSIS);
}

/// Format `names` (all of them, `total` in number) for a message: each in
/// backticks, comma-separated, at most [`MAX_LISTED_NAMES`], then
/// `, … and K more`.  Only the listed names are read, so the cost is
/// `MAX_LISTED_NAMES` names however long the list.
///
/// ```
/// use type_checker_protocol::bounded::name_list;
/// assert_eq!(name_list(["A", "B"], 2), "`A`, `B`");
/// let many = (0..10).map(|i| format!("V{i}"));
/// assert_eq!(
///     name_list(many, 10),
///     "`V0`, `V1`, `V2`, `V3`, `V4`, `V5`, `V6`, `V7`, … and 2 more"
/// );
/// ```
pub fn name_list<S: AsRef<str>>(names: impl IntoIterator<Item = S>, total: usize) -> String {
    name_list_with(names, total, "`", "`")
}

/// [`name_list`] with other quotes, e.g. `'` … `'`.
pub fn name_list_with<S: AsRef<str>>(
    names: impl IntoIterator<Item = S>,
    total: usize,
    open: &str,
    close: &str,
) -> String {
    let listed: Vec<String> = names
        .into_iter()
        .take(MAX_LISTED_NAMES)
        .map(|name| format!("{open}{}{close}", name.as_ref()))
        .collect();
    let mut out = listed.join(", ");
    if total > listed.len() {
        out.push_str(&format!(", … and {} more", total - listed.len()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(message: &str) -> TypeErrorDiagnostic {
        TypeErrorDiagnostic { message: message.to_string(), line: 1, column: 1 }
    }

    #[test]
    fn exactly_the_limit_is_kept_and_one_more_becomes_a_single_note() {
        let mut errors = Vec::new();
        for i in 0..MAX_DIAGNOSTICS {
            assert!(push_bounded(&mut errors, diag(&format!("e{i}"))));
        }
        assert_eq!(errors.len(), MAX_DIAGNOSTICS);
        assert!(diagnostics_full(&errors));
        assert!(!diagnostics_closed(&errors), "full, but nothing dropped yet");
        assert!(!push_bounded(&mut errors, diag("one more")));
        assert!(diagnostics_closed(&errors));
        assert!(!push_bounded(&mut errors, diag("and another")));
        assert_eq!(errors.len(), MAX_DIAGNOSTICS + 1, "one note, not one per drop");
        assert_eq!(errors.last().unwrap().message, DIAGNOSTICS_TRUNCATED);
    }

    #[test]
    fn a_long_message_is_cut_at_a_character_boundary() {
        let mut errors = Vec::new();
        push_bounded(&mut errors, diag(&"é".repeat(MAX_DIAGNOSTIC_MESSAGE_BYTES)));
        let message = &errors[0].message;
        assert!(message.len() <= MAX_DIAGNOSTIC_MESSAGE_BYTES, "{}", message.len());
        assert!(message.ends_with('…'));
        let exact = "a".repeat(MAX_DIAGNOSTIC_MESSAGE_BYTES);
        push_bounded(&mut errors, diag(&exact));
        assert_eq!(errors[1].message, exact, "a message of exactly the limit is kept whole");
    }

    #[test]
    fn the_byte_limit_holds_on_its_own() {
        let mut errors = Vec::new();
        let big = "a".repeat(MAX_DIAGNOSTIC_MESSAGE_BYTES);
        let fit = MAX_DIAGNOSTIC_BYTES / MAX_DIAGNOSTIC_MESSAGE_BYTES;
        for _ in 0..fit {
            assert!(push_bounded(&mut errors, diag(&big)));
        }
        assert!(!push_bounded(&mut errors, diag("x")), "one byte over the total");
        assert_eq!(errors.last().unwrap().message, DIAGNOSTICS_TRUNCATED);
    }

    #[test]
    fn a_name_list_at_the_limit_is_whole_and_one_more_is_summarised() {
        let names: Vec<String> = (0..=MAX_LISTED_NAMES).map(|i| format!("N{i}")).collect();
        let at = name_list(&names[..MAX_LISTED_NAMES], MAX_LISTED_NAMES);
        assert!(!at.contains("more"), "{at}");
        let over = name_list(&names, names.len());
        assert!(over.ends_with(", … and 1 more"), "{over}");
        assert_eq!(name_list_with(["a"], 1, "'", "'"), "'a'");
    }
}
