# CV04b: owned Timeline font-family transport

Status: specified before tests or implementation. This is a bounded current-main
reconciliation in the sole Closure PR and provenance epic #15830. Full Closure
parity and transformation lineage remain open. Miri is **DEFERRED/PENDING**,
never passed; its frozen checkout, run, inputs, receipts and target stay untouched.

## Contract

Add the owned `TimelineConfig.font_family: String` field with default
`"trebuchet ms", verdana, arial, sans-serif` (including the quotes around the
first family). Read `fontFamily` from the same owned `themeVariables` selection
as Timeline colors. The first closed init root and direct family member retain
authority; absent theme selection permits the existing top-level YAML fallback,
while a present empty/invalid/nonobject theme replaces YAML with defaults.
Nested objects, later roots, quoted key text, labels and accessibility blocks
cannot select the field. Keep all preprocessing and bounded reader behavior.

Source family strings use the existing unchanged railroad family sanitizer:
trim, then require a nonempty ASCII alphanumeric string with only space,
underscore, double/single quote, comma, period and hyphen as additional bytes.
Invalid, empty or absent values use the default above. No new scalar limit or
dependency is introduced. Typed families are opaque font resolver names rather
than geometry inputs; layout transports them without changing any geometry or
existing numeric error priority. The painter applies the same trim/character
fallback to a direct forged typed configuration before querying or recording
metadata, so malformed direct payloads have a deterministic effective family.

Trimming precedes validation and includes Unicode whitespace. A raw leading or
trailing TAB/newline therefore trims away; an interior control or NUL rejects.
The CSS-looking default remains one opaque `family_names` query member; this
slice does not parse it into a list of independently resolved fallback families.
The reused reader/sanitizer is a text allowlist, not a CSS parser or JSON-string
type gate: bare `true`, `null` and `12`, commas-only text and unmatched quotes
are admitted when their resulting nonempty text consists only of allowed bytes.
Tests must declare these existing lexical semantics rather than silently adding
a stricter field reader. Resolver caching excludes size; identical family,
weight, style and stretch may deduplicate label/title queries after replacement.

When `timeline_config` is `Some`, clone the caller's label and title fonts and
replace only their family. Preserve each size, weight, style, stretch and line
height, including the current title-font role of TimelinePeriod. Use the family
for Timeline sections/events/legacy embedded events and temporal titles in that
Timeline payload. Add exactly `timeline.fontFamily` to its metadata, containing
the effective queried family. When `timeline_config` is `None`, preserve the
entire existing caller-font and metadata behavior, including other temporal
families. Do not apply default Timeline fonts globally to Gantt/Git/Journey.

No font_size field, font scaling, title-size multiplier, period-font role switch,
wrapping, box sizing, event width, colon splitting, section palette grouping,
Architecture fields or raw-main merge is included. This is resolver-query
transport, not native font availability or rendering acceptance.

## Tests, migration and delivery

Inventory complete owned/main endpoint bodies, every TimelineConfig literal,
producer and consumer and all old complete Scene/query oracles before changes.
Preserve all old authored source bytes, numeric fixtures, shape/arrow/glyph and
error assertions. A mandatory ABI field necessarily requires a narrowly scoped
constructor migration. Commit the literal default-field test first, then the
field/default and constructor-only bootstrap; no renderer or reader behavior
is admitted by a successful compile at this stage.

Next commit semantic tests and expected-font-only migrations before reader or
painter implementation. Require compiled runtime red on complete source IR and
full Scene/query expectations, not a missing-field compile error. The migration
may add the declared default field, the single metadata entry and change only
the expected family inside ordered font queries. Every other old oracle field,
source payload and assertion must remain exact, demonstrated with complete
forward/inverse recipes and saved old bodies. New expected values must be
declared from literals before production calls, never copied from actual output.

New controls cover default/custom/trimmed/invalid source families, init/YAML
ownership and suppression, all metadata and query attributes/order/cache,
TD/LR, title, section, period, separate and embedded events, direct malformed
typed families and complete None-config non-Timeline controls. Compare whole
IR, checked layouts, Scenes and query vectors. Retain the separate numeric and
palette stress controls. Derive the full all-kind manifest closure anew; the
current IR-root scope is 14 reverse consumers and 50 forward inputs.

Run every affected executable target, doctest and all-target strict lint on the
exact final source, plus independent source and actual-terminal security review.
Preserve failed drafts and real diagnostics without allowances or new skips.
Update affected READMEs/changelogs and durable backlog/issues. Publish only to
the dynamically rediscovered sole PR after concrete review; retain current
conflict/hosted/exact-head review and actual-merge gates before auto-merge.
Final-source Miri follows implementation delivery; the heartbeat stays active.
