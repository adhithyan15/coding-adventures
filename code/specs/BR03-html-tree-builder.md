# BR03 — A tree builder built the way the HTML specification is written

**Status:** proposed (2026-09-25). This is BR02 phase P2.

**Builds on:** [BR02 — Venture completion roadmap](BR02-venture-completion-roadmap.md)
§2 and §4 P2. It depends on the P1 work: the scripted-case fakes are gone
(#16010), expected failures are declared, every unflagged case runs in both
scripting modes (#16017), the last upstream case is imported (#16031), and CI
re-audits against upstream (#16036).

---

## 1. The problem

`html-parser`'s tree builder (`src/lib.rs`, about 63,000 lines) produces the
expected tree for 2,654 of 2,655 corpus cases. It is not built the way the
WHATWG specification describes tree construction:

- There is **no insertion-mode state machine.** State is about thirty booleans
  and side lists (`explicit_body_end_seen`,
  `anchors_below_closed_formatting_markers`, …). The specification's 21
  insertion modes are recovered from combinations of them.
- The **stack of open elements** is a list of index paths into an owned tree,
  and there is no list of active formatting elements as the specification
  defines it.
- **Post-parse repair passes** reshape the finished tree to match particular
  corpus shapes (`repair_insanely_badly_nested_table_sequence`,
  `repair_table_cell_fostered_nobr_adoption`), and `normalize_document_shell`
  rebuilds html/head/body afterwards.
- **Fragment parsing** marks its context element with a synthetic
  `data-venture-fragment-context` attribute inside the document.

So the corpus passes, but there is no evidence of correctness on input outside
it. The one case it fails, `template.dat:124` (`</form>` in a template-context
fragment), is the kind of case a specification-shaped builder gets right by
construction.

## 2. Decision: a new crate, not an in-place rewrite

**`html-tree-builder`** is a new crate that implements WHATWG HTML §13.2.4 to
§13.2.6 directly, driven by the existing `html-lexer`. It lives beside
`html-parser` until it is at least as correct, then replaces its tree builder.

An in-place rewrite of a 63,000-line file would leave the parser broken for
the whole rewrite, and it would make "is it better?" hard to measure. A new
crate can be measured against the same corpus from its first day, and the old
builder keeps serving Venture until the new one wins.

## 3. Shape of the new crate

Each specification concept is a named type. The code follows the
specification's order and section numbers, so a reader can hold the two side
by side.

| specification | type or function |
|---|---|
| §13.2.4.1 insertion mode | `enum InsertionMode { Initial, BeforeHtml, BeforeHead, InHead, InHeadNoscript, AfterHead, InBody, Text, InTable, InTableText, InCaption, InColumnGroup, InTableBody, InRow, InCell, InTemplate, AfterBody, InFrameset, AfterFrameset, AfterAfterBody, AfterAfterFrameset }` |
| §13.2.4.1 original insertion mode, stack of template insertion modes | fields on `TreeBuilder` |
| §13.2.4.2 stack of open elements | `OpenElements` over `NodeId`s, with the scope predicates (default, list-item, button and table scope) |
| §13.2.4.3 list of active formatting elements | `ActiveFormatting` with markers, the Noah's Ark clause, reconstruction |
| §13.2.4.4 element pointers | `head_element`, `form_element` |
| §13.2.4.5 other state | `scripting`, `frameset_ok`, `foster_parenting` |
| §13.2.6 tree construction dispatcher | `fn dispatch(&mut self, token)`: HTML content versus foreign content |
| §13.2.6.4.x one rule per mode | `fn in_body(&mut self, token)`, … one function per mode |
| §13.2.6.4.7 adoption agency algorithm | `fn adoption_agency(&mut self, subject)`, written as the specification's numbered steps |
| §13.4 parsing HTML fragments | a real context element and a `template` mode stack; no synthetic attribute |

- **The tree:** an arena of nodes with parent links (`NodeId`), because
  foster parenting, the adoption agency and reparenting need parents. It
  converts to today's `dom_core::Document` at the end, so callers do not
  change. BR02 P4 later makes the arena the real DOM.
- **The tokenizer:** the tree builder sets the tokenizer's state when the
  specification says so (RCDATA, RAWTEXT, script data, PLAINTEXT, and CDATA in
  foreign content), using `HtmlLexContext` as `html-parser` does today.
- **Parse errors:** each rule reports the specification's error code where it
  names one, with the token's position, so BR02 P2's "errors compared by code
  and position" can start here.
- **The current specification, not a remembered one.** When `<select>`
  became customizable (2025) the "in select" and "in select in table" modes
  and "select scope" were removed; `select`, `option`, `optgroup`, `hr` and
  `input` are now handled by the "in body" rules, and `select` bounds the
  default scope. The html5lib corpus follows the current text, and so does
  this crate. Section numbers in the code are the current ones (§13.2.6.4.16
  is "in template").
- **Resource limits** sit on top of the specification, each reported as a
  diagnostic: tree depth 512 (Blink's figure; deeper elements become siblings,
  the stack of open elements is unchanged) and 64 active formatting elements
  after the last marker (the Noah's Ark clause only removes identical entries,
  so distinct attributes otherwise amplify input quadratically). A browser
  parsing untrusted pages needs both; no corpus case reaches either.
- **No test input is recognised anywhere** (BR02 §3). A case the builder does
  not pass is a declared expected failure with its reason.

## 4. Measuring it

- The crate runs the **same corpus** (`html5lib-tree-construction-smoke.dat`)
  through the same harness, in both scripting modes, against its **own**
  expected-failure list. The list starts as "everything" and may only shrink;
  it is the progress bar.
- **Differential testing** runs both builders over the corpus and over
  generated inputs, and reports every tree where they disagree. html5ever is
  added as a development-only oracle (never a runtime dependency) to decide
  which side is right when the two builders disagree.
- The **switch-over gate:** the new builder passes every case the old one
  passes, plus `template.dat:124`, in both scripting modes, and produces no
  diagnostics the old one does not.

## 5. Order of work

1. Crate skeleton: arena tree, `OpenElements`, `ActiveFormatting`, the
   dispatcher, and every insertion mode outside tables: `Initial` through
   `AfterHead`, `InHeadNoscript`, `InBody` with formatting elements, the
   adoption agency algorithm and select, `Text`, `InTemplate`, `AfterBody`, the
   frameset modes and the after-after modes. Harness plus expected-failure
   list. A token that reaches a mode not yet written is handled by the
   `InBody` rules and reported as a `tree-builder-mode-not-implemented`
   diagnostic.
2. Tables: `InTable` through `InCell`, foster parenting, `InTableText`.
3. Foreign content: SVG and MathML adjustments, integration points, CDATA.
4. Fragment parsing (§13.4).
5. Errors by code and position.
6. The switch: `html-parser` delegates tree construction to the new crate. Its
   repair passes, `normalize_document_shell` and the synthetic fragment
   attribute are deleted.

Work outside this crate that the corpus exposes:

- **Script-data tokenization.** `html-lexer` does not end a script at
  `</script ` followed by end of file, at `</script/ >`, or at `</script>`
  after `<!--</scrip `. `html-parser` compensates by rescanning script text in
  its tree builder (`rfind_script_end_marker`); this crate does not, so those
  31 cases stay listed until the lexer is fixed.
- **`<selectedcontent>`** mirrors the selected `<option>` through DOM
  insertion steps, not tree construction. It arrives with BR02 P4 (a real
  DOM).

Each step is its own PR and shrinks the expected-failure list.

## 6. What this does not decide

- Streaming or incremental parsing and `document.write` (BR02 P9, with the JS
  engine).
- Byte decoding (BR02 P3).
- Whether `html-parser` keeps its name once it becomes a thin wrapper over
  `html-lexer` and `html-tree-builder`.
