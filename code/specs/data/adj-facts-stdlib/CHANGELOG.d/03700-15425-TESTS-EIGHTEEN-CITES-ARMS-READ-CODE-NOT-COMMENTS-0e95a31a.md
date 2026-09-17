- **#15425: eighteen e2e tests check "no `cites`" against the tokens of the code, not the raw
  text.** Each asserted `!<recv>.contains("cites ")` or `!<recv>.contains("cites \"")` over text that
  includes `%` comments — 15 over `shipped_table()`, which slices from `adj.find("table …")` to end of
  file, and 3 over the whole file (`elementgroups`, `heredityterm`, `insulinglucagontrigger`). All 18
  now assert `!common::has_code_word(&<text>, "cites")` over the WHOLE file — the 15 through a new
  `whole_adj` read of the same path `shipped_table()` reads. Test-only — no `.adj` change.

  ### The helper

  `tests/common/mod.rs` is new: `adj_code_lines`, `code_idents` and `has_code_word`. They follow
  ADJ's own lexer (`adj-lang/src/_lexer_grammar.rs`) token for token rather than splitting strings:

  - `LINE_COMMENT = %[^\n]*` is skipped wherever a token may start, so a comment is dropped whether it
    fills a line or follows code;
  - `STRING = "([^"\\]|\\.)*"` honours escapes and may span lines, so string contents are blanked and
    "inside a string" is carried across line breaks;
  - on each code line, a whole `NUMBER` is consumed first — sign, decimal point, exponent — then a `$`
    `VAR` (not an identifier), then an `IDENT` `[a-z_][a-z0-9_]*`. There is no word boundary between
    tokens, so `2cites` and `1.e5cites` each yield the identifier `cites`.

  The scan covers the whole file, not the table onward. Two table-scoped versions each let a real
  corroboration through (weaker versions 5 and 6 below); the whole-file scan also catches a `cites`
  above the table (measured below).

  Where the scan differs from the lexer it differs only on the safe side: a character the lexer
  rejects is skipped here. `cites` is not a reserved word, so an atom spelled `cites` also trips the
  check.

  Both comment kinds are real: across the 723 `.adj` files under `adj-facts-stdlib`, 1,612 lines
  carry a `%` comment AFTER code, and 11 carry a `%` only inside a string.

  `tests/adj_code_helper.rs` pins seventeen cases. Twelve deliberately broken helpers each fail at
  least one: comments not dropped, strings not blanked, escapes not honoured, string state not carried
  across lines, the last line dropped, a substring search instead of tokens, numbers not consumed, the
  decimal point not consumed, the exponent not consumed, `VAR` not recognised, uppercase accepted
  inside an identifier, and the scan starting at the first line that holds `table`. The last-line one
  is also rejected by `-D warnings` before any test runs; with warnings allowed it fails two.

  ### Before the fix, at `369fea1677`

  A `%` comment containing the arm's needle:

      15 table-to-EOF arms   comment inside the table     KILL, only at the arm   15 of 15
                             same comment in the header   SURVIVE                 15 of 15
       3 whole-file arms     comment in the header        KILL, at the arm         3 of 3

  The 18 test files are unchanged between `369fea1677` and the base this commit sits on, and so are
  the `.adj` files between `2b45881d27` (where the census above was taken) and that base.

  ### After the fix

      mutant                                                         result
      the same comment, on its own line                              SURVIVE, 18 of 18
      a trailing `% …cites…` comment on the `columns` line             SURVIVE, 18 of 18
      a comment quoting `"table <name>"` just above the declaration    SURVIVE, 18 of 18
      the same comment, plus a real `cites` at the end of the file   KILL at the arm, 18 of 18
      the declaration split across lines, or after another
        statement on its line, a `relate … cites` below the table,
        and a line that looks like the declaration after it          KILL at the arm, 18 of 18 each
      the same statements with no `cites`                            SURVIVE, 18 of 18
      a `relate … cites` just above the declaration                  KILL at the arm, 18 of 18
      the same `relate` with no `cites`                              SURVIVE, 18 of 18
      valid table-level `cites "x" locator "…"`                      KILL at the arm, 18 of 18
      the same, spaced with a tab / no space / two spaces /
        its string on the next line                                  KILL at the arm, 18 of 18 each
      a `cites` glued to a formula's number: `2cites`,
        `1.e5cites`, `1.E5cites`, `12.e05cites`, `-1.e5cites`        KILL at the arm, 18 of 18 each
      a `cites` after a string that closes on the next line          file fails 18 of 18; arm fires 16
      bare 8-space `cites "x"` in no row block                       KILL at the arm, 16 of 18
      valid 8-space row-level `cites "x" locator "…"`                KILL at the arm, 2 of 2
      valid `cites` APPENDED to the trust line, a row opener,
        or a row's source line (54 mutants)                          file passes 0 of 54; arm fires 26
      one-line row block carrying an inline `cites`                  file fails 18 of 18; arm fires 3

  Where the arm does not fire, the file fails first on a guard earlier in the same test, printed for
  each rather than inferred: a verbatim row-shape pin, a row-source count, the verbatim envelope pin
  (for the line-spanning string in `elementgroups` and `heredityterm`, whose mutant opens the
  envelope's own locator), or the CLI rejecting a bare 8-space `cites "x"` (the 2 of 18, which is why
  those two needed the valid row-level form). The comment quoting `"table <name>"`, placed at the top
  of the file instead of just above the declaration, fails 4 of 18 on such a guard; the arm fires on
  it in none.

  ### Six weaker versions, each caught by a security review, none pushed

  1. **Line-start**, `l.trim_start().starts_with(<needle>)`: on the 54 appended placements its arm
     fired on **0**, and **5** whole test files passed — `elementgroups` on the trust line and a row
     opener, `heredityterm` on all three. My own trade-off check had tried only one-line rows.
  2. **Per-line code with the old needle**, `adj_code(l).contains(<needle>)`: a fixed needle misses
     `cites` spaced with a tab, no space, two spaces, or its string on the next line, which let
     `elementgroups` and `heredityterm` pass, as the original substring arms also would; and reading
     each line alone inverts string state after a string that spans lines.
  3. **Whole-word match**, splitting on non-word characters: `2cites` is one word to it. With
     `formula … = v * 2cites "x" locator "…"` appended, **all 18** test files passed; written
     `2 cites`, the arm fired in all 18.
  4. **The same, stripping a leading number from each word**: the split on `.` still ran first, so
     `1.e5cites` became `1` and `e5cites`. Review 4 measured all 18 files passing on `1.e5cites`,
     `1.E5cites`, `12.e05cites` and `-1.e5cites`, and the original substring arms catching them.
  5. **The token scan over the existing slice**: review 5 compared the scan with ADJ's real lexer on
     164 named inputs and 2.4M fuzzed ones and found no case where the scan misses a `cites` the lexer
     sees — but the table arms still scanned the `adj.find` slice. With a header quoting the table
     name and a corroboration at the end of the file, **15 of 18** test files passed. The substring
     arms, which track no string state, were not fooled. Review 5 also measured that a formula, a
     new-tuple `relate` or another table, each with a `cites` and placed ABOVE the table, passed
     those 15.
  6. **The token scan from the first line that looks like the declaration** — the first code line
     whose first two identifiers are `table <name>`. The lexer ignores line breaks, so review 6 split
     the real declaration across lines, or put another statement before it on its line, then added a
     `relate … cites` below the table and a lookalike line after that: **15 of 18** test files passed.

  The version above scans the whole file and closes all six. Twenty-two other tests on `main` still
  use form 1; #15430 tracks them, and they were not mutated here.

  ### Noticed, not changed

  `facts_heredityterm_e2e.rs` asserts the same `columns` line twice.

  Gates: after `cargo clean -p adj-lang-cli`, `cargo clippy --all-targets -- -D warnings` → exit 0,
  with one `Checking adj-lang-cli` line. The 18 test files under `RUSTFLAGS="-Dwarnings"` → 107 passed,
  0 failed; `adj_code_helper` → 17 passed, 0 failed. Every mutant was written to the tracked `.adj`
  inside a try/finally and restored byte-identical by SHA-256.
