# BR02 — Venture: completion roadmap

**Status:** proposed roadmap (2026-09-25). Owner direction: drive Venture to
completion piece by piece, bring the HTML parser up to the full WHATWG
specification, and build our own JavaScript engine on the LANG VM pipeline
(tracked in the V8C series; see §6).

**Builds on:** [BR01 — Venture](BR01-venture-browser.md), whose acceptance
backlog is complete. BR01 says itself that this "does not imply complete
browser conformance". This roadmap is the next phase: from a Mosaic-1.0-class
browser to a standards-following one.

---

## 1. Where Venture stands (audit, 2026-09-25)

Venture's pipeline is: HTML text → `html-lexer` → `html-parser` → render
tree → `html-to-layout` (CSS cascade) → `layout-*` → `layout-to-paint` → a
paint VM (Metal on macOS, Direct2D on Windows, Cairo for Qt, Flutter and
Compose), shown in each host's `HostSurface`.

| subsystem | crates | state | largest gaps |
|---|---|---|---|
| HTML tokenizer | `html-lexer` | html5lib tokenizer suite: 6,806 upstream cases, 0 missing, exact tokens and error codes | error positions only spot-checked |
| HTML tree builder | `html-parser` | WPT tree-construction: 2,654 local cases pass (1 upstream case missing, `template.dat:124`) | see §2: not structured on the spec, partly test-shaped |
| Byte decoding | none | missing | no BOM / HTTP charset / `<meta>` prescan / windows-1252 fallback; callers use `from_utf8_lossy` |
| DOM | `dom-core` (177 lines) | minimal owned tree | no node identity, parent links, mutation, events, template fragments, serialization |
| CSS syntax | `css-lexer`, `css-parser` | grammar-driven | not CSS Syntax Level 3 tokenization and error recovery |
| Cascade | in `html-to-layout` | partial | no sibling combinators, `:not/:is/:where/:has`, `:hover/:focus`, `calc()`, viewport units, `@font-face`, `@supports`, `@layer`, `unset/revert` |
| Layout | `layout-*` (15 crates) | bounded profile | no bidi, no hyphenation, limited writing modes, no WPT CSS reftests |
| Text | `text-native` | CoreText, DirectWrite | **no Linux backend**: widths are a 0.5em-per-character estimate on the Cairo hosts |
| Images | `image-codec-gif`, `-jpeg` wired | partial | PNG decode, WebP, ICO not wired; no SVG |
| Networking | `http1-client` | HTTP/1.0 GET/POST, redirects | **no HTTPS** (`tls-platform` exists, unused), no HTTP/1.1 keep-alive/chunked, no compression, cookies, cache, CORS, sniffing |
| URLs | `url-parser` | RFC 1738 | not the WHATWG URL Standard (no IDNA, special schemes, percent-encode sets) |
| Forms, navigation | `browser-form-*`, `browser-navigation` | advanced | single window, no tabs |
| Accessibility | projected roles and names | partial | page content is not exposed to NSAccessibility / UIA / AT-SPI |
| JavaScript | none in Venture | missing | V8C01–V8C03 are designs only |

BR01's "In Scope / Out of Scope" list is out of date (forms, find-in-page,
printing and macOS are built). It is corrected in the same PR as this roadmap's
first implementation step.

## 2. The HTML parser's conformance claim, stated honestly

The tree builder's pass count is real in the narrow sense that every local case
produces the expected tree. It is not yet evidence that the parser implements
the specification:

1. **Scripted test cases are answered by string matching in production code.**
   `apply_scripted_tree_construction_side_effects` (`html-parser/src/lib.rs`)
   runs whenever scripting is enabled (the default) and recognises the exact
   script text of the scripted WPT cases (`document.write("2")`,
   `document.getElementById("A").id = "B"`, …), rewriting the tree to the
   expected result. There is no script engine. This runs on real pages.
2. **Post-parse repair passes target specific corpus shapes**
   (`repair_insanely_badly_nested_table_sequence`,
   `repair_table_cell_fostered_nobr_adoption`), and `normalize_document_shell`
   rebuilds the html/head/body shell after parsing.
3. **The tree builder has no insertion-mode state machine.** State is about
   thirty booleans and side lists; fragment parsing marks the context with a
   synthetic `data-venture-fragment-context` attribute. Behaviour on inputs
   outside the corpus is unmeasured.
4. **Venture parses with scripting on but runs no scripts,** so `<noscript>`
   content is raw text instead of the fallback markup a script-less browser
   must show.
5. Tree-construction **parse errors** are checked only as "at least one", not by
   code and position, and **CI never re-audits against upstream**; the audit
   script needs local checkouts.

These are the first things this roadmap fixes, because every later claim
about Venture rests on the parser.

## 3. Principles

- **The spec's algorithm, not the corpus's answers.** Code is organised the way
  the WHATWG specification is (insertion modes, stack of open elements, list
  of active formatting elements), with section references inline. No code path
  may recognise a specific test input.
- **A case we cannot pass is an expected failure, visibly.** Scripted cases are
  expected failures until the JavaScript engine exists; the list can only
  shrink.
- **Conformance is measured against pinned upstream suites in CI**, not only
  against a checked-in snapshot.
- **One spec area per PR,** each with its upstream tests imported.

## 4. Phases

Each phase is a sequence of PRs; within a phase the order is the dependency
order.

### P1 — Parser honesty (first)

1. Delete the scripted-case string matching. Run the scripted cases as
   declared expected failures. Venture parses with **scripting off** until it
   has a JavaScript engine, so `<noscript>` renders.
2. Run every unflagged tree-construction case in **both** scripting modes.
3. Import the missing `template.dat:124` case.
4. A CI job that fetches pinned WPT and html5lib-tests revisions and runs the
   coverage audit.

### P2 — Tree builder on the specification's structure

1. Insertion modes as an enum, the stack of open elements and the list of
   active formatting elements as the spec defines them (§13.2.4), the adoption
   agency algorithm as written (§13.2.6.4.7), fragment parsing with a real
   context element (§13.4).
2. Delete the post-parse repair passes and `normalize_document_shell`. The
   corpus must still pass without them; that is the proof.
3. Differential fuzzing against an independent parser (html5ever as a
   dev-dependency oracle only), with found divergences added as regression
   cases.
4. Parse errors compared by code and position.

### P3 — Bytes to characters (§13.2.3)

BOM sniffing, transport-layer charset, the `<meta>` prescan, windows-1252
fallback, and changing the encoding mid-parse, with the html5lib `encoding/`
tests. Needs a WHATWG Encoding Standard decoder crate for the legacy
encodings.

### P4 — A real DOM

`dom-core` becomes the DOM: node identity, parent/child/sibling links,
mutation, `DocumentFragment` and template contents, attributes as the DOM
defines them, events (dispatch, capture, bubble), and HTML serialization
(html5lib `serializer/` tests). The render tree and the browser descriptors are
derived from it. The JavaScript engine binds to this DOM, so it comes first.

### P5 — Networking and URLs

WHATWG URL Standard (with the WPT `url/` tests) → HTTPS through
`tls-platform` → HTTP/1.1 (keep-alive, chunked, gzip/deflate/brotli from the
repo's own codecs) → cookies → cache → MIME sniffing.

### P6 — Text on every host

A Linux text backend (FreeType + the repo's shaping work, per the owner's
from-scratch HarfBuzz direction) so the Cairo hosts measure real glyphs. Then
bidi (UAX #9) and line breaking (UAX #14) in layout.

### P7 — Images

PNG decoding (the `png` crate only encodes), WebP and ICO, then SVG.

### P8 — CSS

CSS Syntax Level 3 tokenizer and parser with error recovery; the remaining
Selectors Level 4; `calc()` and viewport units; full inheritance and the
global keywords; `@font-face` (with `font-parser`), `@supports`, `@layer`;
then a WPT CSS reftest subset as the gate.

### P9 — JavaScript in Venture

Once the engine (V8C series) runs its first test262 subset: DOM bindings over
P4's DOM, `<script>` execution, the event loop and microtasks, then scripting
turned on in the parser — `document.write` feeding the tokenizer — which
retires P1's expected failures.

### P10 — The rest

Platform accessibility trees for page content, tabs and windows, devtools
panels, the web build (#14494), and iOS/iPadOS/Android hosts once Mosaic's
mobile targets exist.

## 5. First PR

P1.1: remove `apply_scripted_tree_construction_side_effects`, mark the scripted
cases as expected failures with a reason, default Venture to scripting off,
and correct BR01's scope list and the html-parser conformance notes so they say
what is and is not measured.

## 6. The JavaScript engine

Tracked in the V8C series, revised for this roadmap: values on `gc-core`
rather than `Rc<RefCell<…>>`, the LANG VM's `vm-core` as the execution target,
and a test262 runner as the gate from the first PR (parse tests first, against
the existing `javascript-parser`).
