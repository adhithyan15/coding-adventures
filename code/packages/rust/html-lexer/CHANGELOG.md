# Changelog

All notable changes to the `coding-adventures-html-lexer` crate will be
documented in this file.

## [0.1.0] - 2026-04-23

### Added
- Initial Rust HTML lexer package.
- Statically linked HTML skeleton state machine over the generic
  `state-machine-tokenizer` runtime.
- Build-time `html-skeleton.lexer.states.toml` authoring artifact.
- Build-time `html1.lexer.states.toml` authoring artifact for the Mosaic-era
  HTML compatibility floor, covering tags, attributes, comments, doctypes, and
  EOF recovery without runtime TOML loading.
- Checked-in generated Rust module for the HTML skeleton lexer definition, so
  the crate links static source instead of loading TOML at runtime.
- Fixture-backed tests proving the generated lexer definition and emitted
  runtime tokens stay aligned.

### Changed
- Switched the stable `create_html_lexer` and `lex_html` API over from the
  bootstrap skeleton to the generated `html1` compatibility-floor lexer.
- Kept the bootstrap skeleton helpers available for focused comparisons while
  the default wrapper now exercises attributes, comments, doctypes, and
  Mosaic-era fixtures.

### Added
- Repo-native JSON conformance fixture suites for the bootstrap skeleton and the
  `html1` compatibility-floor lexer.
- A shared Rust conformance harness that runs the fixture suites against both
  explicit generated constructors and the default wrapper API.
- Documentation for the fixture schema and the planned WHATWG/WPT normalization
  path.
- A first raw html5lib-style tokenizer smoke file plus a Rust normalizer that
  lowers supported upstream cases into Venture's portable conformance schema.
- A checked-in importer script and generated normalized `html5lib-smoke.json`
  corpus so broader upstream-style cases can be regenerated instead of
  hand-maintained in Rust tests.
- RCDATA and `lastStartTag` support in the html5lib importer, with normalized
  fixture metadata that seeds the first executable Rust RCDATA cases while the
  importer still records unsupported upstream states separately.
- RAWTEXT support in the authored `html1` machine, the html5lib importer, and
  the Rust conformance harness, so seeded style-like tokenizer cases now
  execute through the generated static wrapper instead of being skipped.
- Named character reference support for the current generated Rust lexer in
  data, RCDATA, and attribute values, covering `amp`, `lt`, `gt`, `quot`, and
  `apos` as the first shared entity set.
- Decimal and hexadecimal numeric character references in data, RCDATA, and
  attribute values, including replacement-character fallback for null or
  invalid scalar values.
- Missing-semicolon recovery for decimal and hexadecimal numeric character
  references in data, RCDATA, and attribute values.
- Legacy named character references `nbsp`, `copy`, and `reg` in data, RCDATA,
  and attribute values.
- Missing-semicolon recovery for legacy named character references `nbsp`,
  `copy`, and `reg` before delimiters and EOF.
- Generic named character reference scanning with literal fallback for unknown
  names, reducing the need to hand-author one state path per entity.
- Classic Latin-1 named character references in data, RCDATA, and attribute
  values, with case-preserving entity-name scanning for names such as
  `Agrave` and `agrave`.
- Seeded `PLAINTEXT` tokenizer-state support, including shared Venture and
  html5lib-style fixture coverage proving markup and character references stay
  literal in that submode.
- Seeded `script_data` tokenizer-state support with matching end-tag recovery
  and fixture coverage that keeps character references literal in script text.
- First script-data escaped tokenizer states, covering `<!-- ... -->` style
  script text and matching `</script>` end-tag emission from escaped script
  text.
- Script-data double-escaped tokenizer states, so nested-looking
  `<script>...</script>` text inside escaped script comments does not
  prematurely emit the outer script end tag.
- Seeded CDATA section tokenizer-state support for future parser-controlled
  foreign-content tokenization, keeping markup and character references literal
  until the `]]>` delimiter returns the lexer to data state.
- Markup declaration `<![CDATA[` opener support that reaches the CDATA section
  state from data-state lexing while preserving malformed partial openers as
  bogus comments.
- HTML comment start-dash handling, including `<!-->` abrupt empty-comment
  recovery and `<!--->` empty-comment closure.
- HTML comment less-than-sign handling for nested-looking `<!--` sequences
  inside open comments, preserving the text while reporting `nested-comment`.
- HTML comment end-bang handling for `--!>` recovery and non-closing `--!`
  text preservation.
- Processing-instruction-looking `<?...?>` markup now recovers as a bogus
  comment with `unexpected-question-mark-instead-of-tag-name`.
- EOF in bogus-comment recovery now emits the recovered comment without adding
  an unrelated `eof-in-comment` diagnostic.
- Malformed markup declarations such as `<!foo>` now report
  `incorrectly-opened-comment` while recovering as bogus comments.
- Malformed markup declaration fallback now reconsumes the first non-matching
  byte in bogus-comment state, so `<!>` emits an empty comment and returns to
  data state.
- EOF after a malformed markup declaration opener such as `<!` now emits an
  empty bogus comment with `incorrectly-opened-comment` instead of preserving
  the opener as text.
- Duplicate attributes now follow HTML recovery by keeping the first attribute,
  dropping later attributes with the same interpreted name, and reporting
  `duplicate-attribute`.
- Unquoted attribute values now preserve unexpected characters such as `"`,
  `'`, `<`, `=`, and `` ` `` while reporting
  `unexpected-character-in-unquoted-attribute-value`.
- NULL characters in data/RCDATA/RAWTEXT/PLAINTEXT/CDATA/script data and
  attribute values now recover with `unexpected-null-character` and append
  U+FFFD.
- NULL characters in tag names and attribute names now recover with
  `unexpected-null-character` and append U+FFFD.
- NULL characters in comments and bogus comments now recover with
  `unexpected-null-character` and append U+FFFD while preserving pending comment
  dashes.
- NULL characters in DOCTYPE names and quoted public/system identifiers now
  recover with `unexpected-null-character` and append U+FFFD.
- NULL characters in script escaped and double-escaped substates now recover
  with `unexpected-null-character` and append U+FFFD while preserving their
  dash-sensitive state transitions.
- Numeric character references now report invalid-code-point diagnostics and
  recover with replacement/remapping behavior for null, surrogate,
  out-of-range, noncharacter, and Windows-1252 control references.
- Digitless numeric character references such as `&#;` and `&#x;` now report
  `absence-of-digits-in-numeric-character-reference` while staying literal.
- One-dash markup declarations such as `<!->` and `<!-x>` now use
  incorrectly-opened bogus-comment recovery instead of empty-comment recovery.
- Invalid tag-open characters now follow HTML recovery: stray `<` text is
  preserved and malformed end-tag openers recover as bogus comments.
- Missing-name DOCTYPE recovery now marks force-quirks mode for `<!DOCTYPE>`
  and whitespace-only DOCTYPE names.
- DOCTYPE declarations cut off by EOF after a name now emit the current name
  with force-quirks mode enabled.
- DOCTYPE `PUBLIC` and `SYSTEM` identifier states now preserve quoted public
  and system identifiers on emitted tokens, including force-quirks recovery for
  missing identifiers.
- Standalone `SYSTEM` doctypes and trailing junk after system identifiers are
  now covered, with unexpected trailing junk marking force-quirks mode.
- DOCTYPE public/system recovery conformance now covers missing whitespace
  around identifiers, missing identifier quotes, and abrupt identifier
  termination.
- DOCTYPE declarations cut off while matching the `DOCTYPE` keyword now emit a
  force-quirks token instead of a clean partial declaration.
- Malformed `DOCTYPE` keyword text now marks force-quirks mode while preserving
  the recovered keyword text as the best-effort DOCTYPE name.
- Named character reference recovery now uses the longest matching known
  entity prefix in text and RCDATA, while preserving ambiguous ampersands
  literally in attribute values when the missing-semicolon reference would be
  followed by an ASCII alphanumeric character or `=`.
- Completed the HTML4-era math named reference table with `alefsym` and
  `oline`, including data, RCDATA, attribute, and normalized html5lib smoke
  coverage.
- Added a WHATWG named-reference batch for spacing, invisible operators,
  punctuation aliases, and math constants, including multi-codepoint
  replacements such as `ThickSpace`.
- Added a WHATWG relation/operator named-reference batch for equality, tilde,
  greater-than, less-than, and negated aliases, including combining-overlay
  replacements such as `NotNestedGreaterGreater`.
- Added a WHATWG arrow/vector named-reference batch covering basic, double,
  long, bar, tee, map, vector, and vector-bar arrow aliases.
- Added a WHATWG Greek variant named-reference batch covering epsilon, kappa,
  phi, pi, rho, sigma, theta, upsilon, digamma, and letter-like aliases.
- Added a WHATWG set/logic named-reference batch covering set operations,
  membership, subset/superset, square-set, and n-ary logic aliases.
- Added a WHATWG operator/shape named-reference batch covering circled
  operators, integrals, products, squares, lozenges, stars, suits, and symbols.
