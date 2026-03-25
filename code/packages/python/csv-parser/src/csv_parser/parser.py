"""
parser.py — Hand-rolled CSV state machine parser.

═══════════════════════════════════════════════════════════════════════════════
BACKGROUND: WHY A STATE MACHINE?
═══════════════════════════════════════════════════════════════════════════════

Most text formats (JSON, SQL, HTML) can be tokenized by a regular expression or a
simple context-free grammar. CSV cannot, because the meaning of a comma depends on
the *context* — specifically, whether you are inside a quoted field.

Consider:
    Widget,9.99,"A small, round widget"
                        ^
                        This comma is NOT a field separator — it is part of the value.

A regex-based tokenizer would see all three commas equally. Only by tracking state
(am I inside quotes right now?) can we correctly parse the third field.

This is a classic use case for a **finite automaton** (state machine). We define a
small set of states, and for each state we decide what to do with the current character.

═══════════════════════════════════════════════════════════════════════════════
THE GRAMMAR (EBNF, for reference)
═══════════════════════════════════════════════════════════════════════════════

    file        = [header] { record }
    header      = record
    record      = field { COMMA field } (NEWLINE | EOF)
    field       = quoted | unquoted
    quoted      = '"' { any char; '"' becomes '""' for literal quote } '"'
    unquoted    = { any char except COMMA, '"', NEWLINE, EOF }

    COMMA       = ',' (default; configurable)
    NEWLINE     = '\r\n' | '\n' | '\r'
    EOF         = end of input

═══════════════════════════════════════════════════════════════════════════════
STATE MACHINE DIAGRAM
═══════════════════════════════════════════════════════════════════════════════

                    ┌─────────────────────────────────────┐
                    │           FIELD_START               │◄──────┐
                    │  decide: quoted or unquoted?        │       │
                    └──────────────┬──────────────────────┘       │
                                   │                              │
              ┌────────────────────┼───────────────┐             │
              │                   │               │             │
           '"' char           other char    COMMA/NEWLINE        │
              │                   │          (empty field)       │
              ▼                   ▼               │             │
    ┌─────────────────┐  ┌──────────────────┐    │             │
    │ IN_QUOTED_FIELD │  │IN_UNQUOTED_FIELD │    └─────────────┘
    │                 │  │                  │
    │  ┌──────────────┤  │ ┌────────────────┤
    │  │ any non-'"'  │  │ │ non-special    │
    │  │ → append     │  │ │ → append       │
    │  └──────────────┤  │ └────────────────┤
    │                 │  │                  │
    │  '"' → move to  │  │ COMMA → end field│
    └──────┬──────────┘  │ NEWLINE → end row│
           │             │ EOF → end row    │
           ▼             └──────────────────┘
    ┌──────────────────────────┐
    │   IN_QUOTED_MAYBE_END    │
    │   (just saw '"' inside   │
    │    a quoted field)       │
    └──────┬───────────────────┘
           │
    ┌──────┴─────────────────────────────┐
    │                                    │
    ▼                                    ▼
   '"'                            COMMA/NEWLINE/EOF
  (escaped quote: ""               (field ends cleanly)
   → append single '"',
   → back to IN_QUOTED_FIELD)

═══════════════════════════════════════════════════════════════════════════════
ESCAPE LOGIC TRUTH TABLE (IN_QUOTED_MAYBE_END state)
═══════════════════════════════════════════════════════════════════════════════

  Next char after '"'  │  Meaning
  ─────────────────────┼─────────────────────────────────────────
  '"'                  │  Escape sequence "" → emit single '"', stay in quoted field
  ','  (delimiter)     │  End of quoted field, delimiter follows
  '\n' or '\r'         │  End of quoted field, record ends
  EOF                  │  End of quoted field, file ends
  anything else        │  Technically malformed CSV; we treat as end of quote and
                       │  emit the unexpected char as the start of the next field
                       │  (lenient parsing — many real files are technically wrong)

═══════════════════════════════════════════════════════════════════════════════
NEWLINE NORMALIZATION
═══════════════════════════════════════════════════════════════════════════════

RFC 4180 specifies '\r\n' as the record terminator. In practice, files use '\n'
(Unix), '\r\n' (Windows), and occasionally bare '\r' (old macOS). We treat all
three as record terminators:

  - '\n' alone: record ends
  - '\r\n': '\r' triggers "end of record" and the following '\n' is consumed
  - '\r' alone: record ends

Inside a quoted field, newlines are *preserved* as-is (not normalized). If the
source file uses '\r\n' inside a quoted field, the field value will contain '\r\n'.
This matches the most common real-world expectation.
"""

from enum import Enum, auto

from csv_parser.errors import UnclosedQuoteError


class ParseState(Enum):
    """The four states of the CSV parser automaton.

    Each state represents a distinct "mode" the parser can be in as it reads
    characters one at a time from left to right through the source string.

    Think of each state as answering the question: "What am I currently doing?"
      FIELD_START        → I'm about to start a new field; let me look at the first char.
      IN_UNQUOTED_FIELD  → I'm reading a plain field (no quotes); stop at comma/newline/EOF.
      IN_QUOTED_FIELD    → I'm inside "..." and only a '"' can end or escape.
      IN_QUOTED_MAYBE_END→ I just saw '"' inside a quoted field; what comes next decides.
    """

    FIELD_START = auto()
    IN_UNQUOTED_FIELD = auto()
    IN_QUOTED_FIELD = auto()
    IN_QUOTED_MAYBE_END = auto()


def parse_csv(source: str, delimiter: str = ",") -> list[dict[str, str]]:
    """Parse CSV text into a list of row maps.

    Each row in the output is a dictionary mapping header column names to field
    values. All values are strings — no type coercion is performed.

    The first row of the CSV is always treated as the header. If the source has
    only one row (the header), an empty list is returned. If the source is empty
    (zero characters), an empty list is returned.

    Args:
        source:    The full CSV text as a string (UTF-8 recommended).
        delimiter: A single character used as the field separator.
                   Default: ',' (standard CSV).
                   Common alternatives: '\t' (TSV), ';' (European CSV), '|' (pipe).

    Returns:
        A list of dicts. Each dict maps column name → field value (both strings).
        The list does not include the header row itself.

    Raises:
        UnclosedQuoteError: If a quoted field is opened with '"' but never closed.

    Examples:
        >>> parse_csv("name,age\\nAlice,30\\nBob,25")
        [{'name': 'Alice', 'age': '30'}, {'name': 'Bob', 'age': '25'}]

        >>> parse_csv("a,b,c\\n1,,3")
        [{'a': '1', 'b': '', 'c': '3'}]

        >>> parse_csv('id,val\\n1,"say ""hi"""')
        [{'id': '1', 'val': 'say "hi"'}]

        >>> parse_csv("name\\tage\\nAlice\\t30", delimiter="\\t")
        [{'name': 'Alice', 'age': '30'}]
    """
    # ── Step 1: tokenise the source into a list of rows (list of list of str) ──────
    # This separates the low-level character scanning from the higher-level logic of
    # matching data rows to header columns. It also makes the code easier to test in
    # isolation.
    raw_rows = _scan(source, delimiter)

    # ── Step 2: if there are no rows at all, return an empty list ────────────────
    # This handles the empty-file case.
    if not raw_rows:
        return []

    # ── Step 3: pull the first row out as the header ─────────────────────────────
    header = raw_rows[0]
    data_rows = raw_rows[1:]

    # If there are no data rows (header-only file), return an empty list.
    # The caller gets [] rather than a list containing the header itself.
    if not data_rows:
        return []

    # ── Step 4: convert each data row into a dict keyed by header columns ────────
    result: list[dict[str, str]] = []
    for row in data_rows:
        result.append(_zip_row(header, row))

    return result


# ──────────────────────────────────────────────────────────────────────────────
# INTERNAL: _scan
# Converts the raw source string into a list of rows, where each row is a list
# of field value strings. This is the core state machine.
# ──────────────────────────────────────────────────────────────────────────────

def _scan(source: str, delimiter: str) -> list[list[str]]:
    """Drive the state machine over `source` and return a list of raw rows.

    Each row is a list of strings. No header processing is done here; this
    function is purely about character-by-character parsing.

    The sentinel approach: we append a virtual '\n' to the source so that the
    final record (which may not have a trailing newline) is always flushed. This
    simplifies the state machine — we never need special EOF handling inside the
    scan loop; the '\n' sentinel triggers normal end-of-record logic.

    Exception: inside a quoted field, EOF (the sentinel) raising an error is
    handled by checking the final state after the loop.
    """
    # Append a sentinel newline so the last record is always flushed cleanly.
    # We use a flag to detect if we're processing the sentinel vs. a real newline.
    sentinel = "\n"
    chars = source + sentinel

    state = ParseState.FIELD_START

    # current_field accumulates characters for the field being parsed
    current_field: list[str] = []

    # current_row accumulates fields for the row being parsed
    current_row: list[str] = []

    # all_rows collects completed rows
    all_rows: list[list[str]] = []

    # We track position for potential error messages (future use)
    i = 0
    n = len(chars)

    while i < n:
        ch = chars[i]

        # ── FIELD_START ────────────────────────────────────────────────────────
        # We are at the very beginning of a new field. The first character tells us
        # which type of field this is.
        if state == ParseState.FIELD_START:
            if ch == '"':
                # Opening quote: switch to quoted-field mode.
                # We do NOT include the '"' itself in the field value.
                state = ParseState.IN_QUOTED_FIELD
                i += 1

            elif ch == delimiter:
                # A delimiter immediately after FIELD_START means an empty unquoted field.
                # Example: a,,b → the middle field is ''.
                current_row.append("")
                # Stay in FIELD_START for the next field.
                i += 1

            elif ch in ("\n", "\r"):
                # A newline immediately after FIELD_START.
                # This can happen when:
                #   1. We just processed a delimiter at the end of a row (trailing comma).
                #   2. We're looking at a blank line (or the sentinel newline we added).
                # Either way, flush the current (empty) field and complete the row.
                # _finish_row skips the [""] artefact rows produced by the sentinel.
                current_row.append("")
                _finish_row(current_row, all_rows)
                current_row = []
                # Handle '\r\n' — consume the '\n' after '\r' as one record terminator
                if ch == "\r" and i + 1 < n and chars[i + 1] == "\n":
                    i += 2
                else:
                    i += 1

            else:
                # Any other character starts an unquoted field.
                state = ParseState.IN_UNQUOTED_FIELD
                current_field.append(ch)
                i += 1

        # ── IN_UNQUOTED_FIELD ──────────────────────────────────────────────────
        # We are consuming characters of a plain (unquoted) field.
        # The field ends at a delimiter, a newline, or EOF (sentinel).
        elif state == ParseState.IN_UNQUOTED_FIELD:
            if ch == delimiter:
                # End of field; another field follows on the same row.
                current_row.append("".join(current_field))
                current_field = []
                state = ParseState.FIELD_START
                i += 1

            elif ch in ("\n", "\r"):
                # End of field AND end of row.
                current_row.append("".join(current_field))
                current_field = []
                _finish_row(current_row, all_rows)
                current_row = []
                state = ParseState.FIELD_START
                # Handle '\r\n' as a single newline
                if ch == "\r" and i + 1 < n and chars[i + 1] == "\n":
                    i += 2
                else:
                    i += 1

            else:
                # Regular character: accumulate it.
                current_field.append(ch)
                i += 1

        # ── IN_QUOTED_FIELD ────────────────────────────────────────────────────
        # We are inside a quoted field. Any character except '"' is taken literally,
        # including commas and newlines. Only '"' is special: it either ends the
        # field or begins an escape sequence ("").
        elif state == ParseState.IN_QUOTED_FIELD:
            if ch == '"':
                # Could be end-of-field OR the start of a "" escape sequence.
                # Move to the "maybe end" state and peek at the next character.
                state = ParseState.IN_QUOTED_MAYBE_END
                i += 1

            else:
                # All other characters (including delimiter and newline) are literal
                # inside a quoted field. This is the core of RFC 4180 quoted fields.
                current_field.append(ch)
                i += 1

        # ── IN_QUOTED_MAYBE_END ────────────────────────────────────────────────
        # We just saw '"' while inside a quoted field. The NEXT character tells us:
        #
        #   '"' again   → escape sequence ("") → emit one '"', stay in quoted field
        #   delimiter   → end of quoted field, delimiter follows
        #   '\n'/'\r'   → end of quoted field, end of row
        #   sentinel    → end of quoted field, end of file
        #   anything else → treat as end of quote (lenient), re-process char
        elif state == ParseState.IN_QUOTED_MAYBE_END:
            if ch == '"':
                # "" escape sequence → one literal '"' in the output
                current_field.append('"')
                state = ParseState.IN_QUOTED_FIELD
                i += 1

            elif ch == delimiter:
                # Field ends cleanly; a sibling field follows.
                current_row.append("".join(current_field))
                current_field = []
                state = ParseState.FIELD_START
                i += 1

            elif ch in ("\n", "\r"):
                # Field ends cleanly; record ends.
                current_row.append("".join(current_field))
                current_field = []
                _finish_row(current_row, all_rows)
                current_row = []
                state = ParseState.FIELD_START
                if ch == "\r" and i + 1 < n and chars[i + 1] == "\n":
                    i += 2
                else:
                    i += 1

            else:
                # Unexpected character after closing '"'. RFC 4180 says this is
                # undefined. We take the lenient approach: treat the '"' as the end
                # of the quoted field and re-process the current character without
                # advancing i (so it gets handled in the next loop iteration under
                # FIELD_START/unquoted logic).
                #
                # Example: "value"extra → field value is "value", then "extra" is
                # concatenated (both end up in the same field because FIELD_START
                # only starts a new field when it sees a delimiter).
                #
                # Actually, more precisely: we flush the current quoted portion,
                # switch to unquoted, and continue accumulating.
                state = ParseState.IN_UNQUOTED_FIELD
                # Do NOT advance i — the current char is handled in the next iteration.

    # ── Post-loop: check for unclosed quoted field ─────────────────────────────
    # If we exited the loop while still inside a quoted field (or in the
    # "maybe end" state that can only be reached from inside a quoted field
    # when the very last character was '"' before our sentinel), that means
    # the quoted field was never closed.
    #
    # Wait — our sentinel '\n' should have triggered the end-of-field transition
    # for IN_QUOTED_MAYBE_END. So the only way to end up here in IN_QUOTED_FIELD
    # is if the source ended without ever closing the quote.
    if state == ParseState.IN_QUOTED_FIELD:
        raise UnclosedQuoteError(
            "Unclosed quoted field at end of input. "
            "A field was opened with '\"' but the matching closing '\"' was never found."
        )

    # Flush any remaining content (edge case: source with no trailing newline
    # but only if the sentinel didn't already flush it).
    # Because we added a sentinel '\n', the loop should always have flushed the
    # last record. But as a safety net:
    if current_row or current_field:
        if current_field:
            current_row.append("".join(current_field))
        if current_row:
            _finish_row(current_row, all_rows)

    return all_rows


def _finish_row(row: list[str], all_rows: list[list[str]]) -> None:
    """Append a completed row to all_rows, skipping blank rows.

    A "blank row" here means a row containing exactly one field that is the
    empty string. This happens when the source has a trailing newline — the
    sentinel '\n' we append to the source causes FIELD_START to see a '\n',
    which would produce an empty field and an empty row. We skip those.

    We do NOT skip rows that have more than one empty field (e.g., ",,"), because
    those are genuinely empty data rows that the user put there intentionally.
    """
    # A row of exactly [""] is an artefact of trailing newlines; skip it.
    if row == [""]:
        return
    all_rows.append(row)


def _zip_row(header: list[str], row: list[str]) -> dict[str, str]:
    """Combine a header row and a data row into a dict.

    Implements ragged-row handling:
      - If the data row is shorter than the header, missing fields are filled
        with the empty string "".
      - If the data row is longer than the header, extra fields are discarded.

    This matches the spec's requirement that the header defines the authoritative
    column count, and ragged rows should not raise errors.

    Args:
        header: List of column name strings from the first CSV row.
        row:    List of field value strings from a data row.

    Returns:
        A dict mapping each header column to its corresponding field value.

    Examples:
        header = ["a", "b", "c"], row = ["1", "2"]
        → {"a": "1", "b": "2", "c": ""}  (padded)

        header = ["a", "b"], row = ["1", "2", "3"]
        → {"a": "1", "b": "2"}  (truncated)
    """
    result: dict[str, str] = {}
    for col_index, col_name in enumerate(header):
        if col_index < len(row):
            result[col_name] = row[col_index]
        else:
            # Pad missing fields with empty string
            result[col_name] = ""
    return result
