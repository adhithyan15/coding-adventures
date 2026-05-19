# Changelog — @coding-adventures/forme-aot-manifest-emitter

## 0.1.0 — 2026-05-19

Initial release.  Thirteenth FM00 v0 stage package — web app
manifest.json emitter per https://www.w3.org/TR/appmanifest/.

Pure transform: `WebAppManifest` config → JSON.stringify-ed
string with sorted keys for deterministic, diff-friendly
output.  Validation runs in a fail-fast pre-pass BEFORE
emission so callers never see a partial manifest.

### Added

- `generateManifest(config): string` — main entry.  Returns
  pretty-printed JSON (2-space indent).  Throws `TypeError`
  synchronously on any validation failure.
- `validateManifestUrl(url, field)` — URL accept-list:
  http(s):// (case-insensitive) OR root-relative `/path`.
- `validateDisplay(value)` — allowlist:
  fullscreen / standalone / minimal-ui / browser
  (case-insensitive).
- `validateColor(value, field)` — hex pattern: `#rgb`,
  `#rgba`, `#rrggbb`, `#rrggbbaa`.
- `WebAppManifest`, `ManifestIcon`, `DisplayMode` types.

### Spec adherence

Implements W3C Web App Manifest (Working Draft) subset
covering the fields needed for FM00 v0 PWA support.  No spec
divergences.

### Behavioural notes

- **Validation BEFORE emission.**  All fields validated in a
  single pass; any throw means no output reaches the caller.
- **Deterministic key ordering.**  Top-level keys alphabetised;
  icon keys ordered as `src` first, then alphabetical (matches
  W3C example output style).  Byte-determinism + diff-
  friendly diffs.
- **URL fields** (`start_url`, `scope`, `icons[].src`)
  validated against http(s)://-or-root-relative accept-list.
- **`display`** validated against the W3C allowlist; case-
  insensitive, output lowercased.
- **Hex colour only.**  Named colours, rgb()/hsl(), attr()
  all rejected for determinism + CSS-parser-confusion
  defence.
- **Icons preserve caller order.**  Sort the array before
  passing if you want a specific output order.
- **Pretty-printed** with 2-space indent (web convention).
- **Empty config (`{}`)** → `"{}"` literal output.

### Security posture

Four concerns explicitly addressed (pre-push review):

- **URL scheme injection.**  `javascript:`, `data:`, `file:`,
  `vbscript:`, protocol-relative `//host`, `/\backslash-variant`
  all rejected.  Manifest URLs are followed by browsers when
  installing the PWA; bad schemes could be XSS or
  exfiltration vectors.
- **Display allowlist.**  Defends downstream consumers from
  unknown display modes that might trigger fallback behaviour
  with different security properties.
- **Colour validation.**  Hex-only — CSS names / rgb() / hsl()
  / attr() rejected.  Prevents CSS-context confusion in
  browsers that try to parse the colour into a CSS string.
- **JSON output safe to interpolate.**  `JSON.stringify` is
  the source of truth; no manual string concatenation.
  Output is well-formed JSON; safe to write to `.json` file
  and reference via `<link rel="manifest">`.

### Capabilities

`[]` — pure transform.  No I/O, no network, no shell, no env,
no fs.

### Tests

83 tests across 2 files:

- `validate.test.ts` (46) — `validateManifestUrl` accept
  (http, https, case-insensitive scheme, root-relative,
  bare /, multi-segment) + reject (javascript:, data:,
  file:, protocol-relative, backslash-variant, bare
  relative, empty, non-string, null, error contains field
  name, long-URL truncation); `validateDisplay` allowlist
  (all four modes parameterised, case-insensitive) +
  reject ('tab', 'window-controls-overlay', empty,
  non-string, null, error contains bad value);
  `validateColor` accept (3/4/6/8-digit hex, uppercase,
  mixed case) + reject (missing #, CSS name, rgb/rgba/hsl,
  non-hex chars, invalid length 5/7, empty, non-string,
  error contains field name).
- `generate.test.ts` (37) — minimal config (empty → {},
  single name); plain string fields (name + short_name +
  description, lang + dir, orientation, non-string throws);
  URL fields (start_url + scope absolute + relative,
  javascript:/data: rejected, empty rejected); display
  allowlist (standalone, minimal-ui, case-insensitive,
  'tab' rejected); colour fields (hex accept including
  alpha, CSS name reject, rgba() reject); icons array
  (single, multiple, all-fields-with-purpose, order
  preserved, not-array throw, null icon throw, javascript:
  src throw, error identifies bad index); fail-fast (bad
  display before icons validation, bad theme_color);
  deterministic key ordering (alphabetical top-level,
  byte-identical output, input order doesn't matter, icon
  src first then alphabetical); pretty-print (2-space
  indent); purity (no input mutation); full real-world PWA
  example.

Coverage: **100% line / 100% branch** across all source
files with logic (`types.ts` is type-only declarations).

### v0 simplifications (documented)

- **No `shortcuts` field.**  PWA shortcut menu entries
  deferred to v1.
- **No `screenshots` field.**  Same reason.
- **No `related_applications` / `prefer_related_applications`**
  — native-app discovery deferred.
- **No `protocol_handlers` / `share_target`** — advanced
  install integration deferred.
- **No CSS colour syntax beyond hex.**  Named colours and
  rgb()/hsl() are spec-compliant but the determinism /
  CSS-parser-confusion tradeoff isn't worth it for v0.
- **No `window-controls-overlay` display mode.**  Newer spec
  extension; deferred.
