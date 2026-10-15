## Unreleased - Bounded Railroad themes and lexical styles

All four Railroad dialects resolve themeVariables from init directives or
top-level/config YAML front matter before explicit Railroad overrides. The
selected effective font size must be finite and in (0,65536], including theme
fallbacks; a valid explicit size overrides an excessive theme size. Unparseable,
nonfinite and nonpositive theme numbers retain the producer's default fallback.
Explicit-field numeric error domains and precedence remain unchanged.

Authored color and font-family strings use the producer's trimmed lexical
allowlists. A selected theme primary/secondary/default is sanitized against the
field default, then an invalid explicit value uses that sanitized theme. An
invalid selected primary does not retry secondary. Marker fallback follows the
theme line independently of an explicit line override. These filters do not
certify complete CSS/color/font grammar or sanitization of caller-forged typed IR.

Integrate producer theme fallback and lexical sanitization without changing
its string policy. Repair only the selected inherited-font admission bypass;
the 72-case test-first cap failure remains recorded. Retain all explicit
numeric, spacing, font, glyph, arc, style and resource controls. New full-config
tests cover 2,112 accepted colors, 1,188 explicit-to-theme fallbacks, 120 font
cases and 12 selected-invalid-primary default cases across four notations and
three source forms. Full fourteen-package validation passes 890/0/0 over
37 targets, plus strict lint; focused parser22/layout14/paint6 pass. Complete
native inputs retain actual c0f2 source acceptance; this is not new native
execution. Broader visual/resource and full Closure provenance debts remain
open. Final-source Miri DEFERRED/PENDING, never passed.
