### Added — Script closure (HL11)

- `measureScriptClosure()` asks the question the glyph budget cannot: for each
  glyph the reader is asked to read, had an earlier lesson taught it? Wired into
  the gap report, always present, report-only.
- First measurement: **932** lessons across 16 non-Latin tracks show a glyph
  nobody taught, and **12 of those 16 teach no letters at all**. The pace budget
  flags 61. A track can satisfy a cap on speed while teaching nothing.
- Exposure is drawn mechanically: a headword is exposure when the lesson declares
  a `romanization`. **489** native-script headwords carry none, so they are
  load-bearing — and each becomes exempt the moment somebody writes down how to
  say it, which is a real improvement rather than a way to hide from the number.
- Two numbers watch the exemption. `exposureOnly` counts lessons it flipped to
  clean (49); `exposureExemptedGlyphs` counts what it actually removed, including
  from lessons that violate anyway (**1,997**). The lesson count alone cannot see
  a lesson reporting five untaught glyphs while fifteen more were exempted.
- A track whose declared script is unknown is reported as UNMEASURED by name,
  never skipped. Both "genuinely Latin" and "unrecognised" used to fall out of
  the report identically, which is the silent zero this module exists to prevent.
- `belongsToAny` replaces `systemOf` at the two classification sites.
  `Script_Extensions` is set-valued, so the shared Vedic and Indic combining
  marks belong to Devanagari *and* to Bengali, Kannada, Malayalam, Tamil and
  Telugu at once — and first-match attribution gave every one of them to
  Devanagari, silently dropping them from every other abugida.
- `SCRIPT_SYSTEMS` is exported frozen. The regex matchers are derived from it
  once at module load, so a consumer adding a script afterwards would pass
  membership tests that `belongsToAny` never learned — and the track would report
  zero debt while appearing measured.
- `SCRIPT_SYSTEMS` and `systemOf` are exported from `ramp.ts` so the two script
  measurements share one definition of what belongs to a script.

All notable changes to `@coding-adventures/human-language-data` are documented here.

## [Unreleased]

