# @coding-adventures/forme-aot-manifest-emitter

Emit web app `manifest.json` from a structured `WebAppManifest`
config per [W3C Web App Manifest](https://www.w3.org/TR/appmanifest/).

Pure transform — returns the JSON string with sorted keys
(deterministic, diff-friendly).  Validation runs BEFORE
emission, so callers never see a partial manifest.

Thirteenth FM00 v0 stage package — joins the FM00 v0 cluster.

## Quick start

```ts
import { generateManifest } from "@coding-adventures/forme-aot-manifest-emitter";

const json = generateManifest({
  name: "My App",
  short_name: "App",
  description: "An installable web app",
  start_url: "/",
  scope: "/",
  display: "standalone",
  lang: "en-US",
  theme_color: "#0066cc",
  background_color: "#ffffff",
  icons: [
    { src: "/icon-192.png", sizes: "192x192", type: "image/png" },
    { src: "/icon-512.png", sizes: "512x512", type: "image/png", purpose: "maskable" },
  ],
});

fs.writeFileSync("dist/manifest.json", json);
// Then in your HTML <head>:
//   <link rel="manifest" href="/manifest.json">
```

## API

### `generateManifest(config): string`

Main entry.  Returns the pretty-printed JSON document
(2-space indent).

```ts
interface WebAppManifest {
  readonly name?: string;
  readonly short_name?: string;
  readonly description?: string;
  readonly lang?: string;
  readonly dir?: "ltr" | "rtl" | "auto" | string;
  readonly start_url?: string;        // http(s):// OR /root-relative
  readonly scope?: string;            // http(s):// OR /root-relative
  readonly display?: DisplayMode;     // allowlisted
  readonly orientation?: string;
  readonly theme_color?: string;      // hex: #rgb, #rgba, #rrggbb, #rrggbbaa
  readonly background_color?: string; // hex
  readonly icons?: ManifestIcon[];
}

type DisplayMode = "fullscreen" | "standalone" | "minimal-ui" | "browser";

interface ManifestIcon {
  readonly src: string;               // http(s):// OR /root-relative
  readonly sizes?: string;
  readonly type?: string;
  readonly purpose?: string;
}
```

Throws `TypeError` synchronously BEFORE any output if:
- A URL field isn't `http(s)://` or root-relative.
- `display` isn't in the allowlist.
- A colour field isn't `#rgb` / `#rgba` / `#rrggbb` / `#rrggbbaa`.
- Required object structure is wrong (e.g. `icons` not an array,
  icon entry not an object).

### Sub-helpers (exposed)

- `validateManifestUrl(url, field)` — URL accept-list.
- `validateDisplay(value)` — display allowlist.
- `validateColor(value, field)` — hex colour pattern.

## Deterministic key ordering

Top-level keys are emitted in alphabetical order.  Icon entry
keys are emitted with `src` first, then alphabetical (matches
W3C example output style).

Two benefits:
- **Byte-determinism.**  Same input → identical output
  regardless of caller-object property insertion order.
- **Diff-friendly.**  Sites checking the manifest into git
  see clean diffs.

## Validation

| Field                            | Validator                              |
|----------------------------------|----------------------------------------|
| `start_url`, `scope`, `icons[].src` | `http(s)://` OR root-relative `/path` |
| `display`                        | allowlist: fullscreen / standalone / minimal-ui / browser (case-insensitive) |
| `theme_color`, `background_color`| hex: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa` |
| Plain string fields              | typeof string check                    |
| `icons`                          | array of non-null objects              |

## Behavioural contract

| Aspect                           | Behaviour                              |
|----------------------------------|----------------------------------------|
| Input config                     | Never mutated                          |
| Icons array order                | Preserved (caller decides)             |
| Top-level keys                   | Alphabetically sorted                  |
| Icon keys                        | src first, then alphabetical           |
| Validation                       | All fields validated BEFORE emit       |
| Bad field                        | Throws `TypeError`; no partial output  |
| Same input                       | Byte-identical output                  |
| Empty config (`{}`)              | `"{}"` literal                          |

## Reproducibility (FM03)

Same `config` → byte-identical manifest.json.

## Security posture

Four concerns explicitly addressed (pre-push review):

- **URL scheme injection.**  `javascript:`, `data:`, `file:`,
  `vbscript:`, protocol-relative `//host`, `/\backslash-variant`
  all rejected.  Manifest URLs are followed by browsers when
  installing the PWA; bad schemes here could be XSS or
  exfiltration vectors.
- **Display allowlist.**  Caller-supplied values match the
  W3C-defined set or throw.  Prevents downstream consumers
  from encountering unknown display modes that might trigger
  fallback behaviour with different security properties.
- **Colour validation.**  Hex-only — `red` / `rgb()` / `hsl()`
  / `attr()` rejected.  Defends against CSS-context confusion
  in browsers that try to parse the colour into a CSS string.
- **JSON output safe to interpolate.**  `JSON.stringify` is
  the source of truth; no manual string concatenation.
  Output is well-formed JSON; safe to write to a `.json` file
  and reference via `<link rel="manifest">`.

## Capabilities — `[]`

Pure transform.  No I/O, no network, no shell, no env, no fs.

## Tests

83 tests across 2 files:

- `validate.test.ts` (46) — `validateManifestUrl` accept
  (http, https, case-insensitive, root-relative, bare /,
  multi-segment) + reject (javascript:, data:, file:,
  protocol-relative, backslash-variant, bare relative,
  empty, non-string, null, error contains field name,
  long-URL truncation); `validateDisplay` allowlist (all
  4 modes parameterised, case-insensitive) + reject ('tab',
  'window-controls-overlay', empty, non-string, null, error
  contains bad value); `validateColor` accept (3/4/6/8-digit
  hex, uppercase, mixed case) + reject (no #, CSS name,
  rgb/rgba/hsl, non-hex chars, invalid length 5/7, empty,
  non-string, error contains field name).
- `generate.test.ts` (37) — minimal config (empty, name only);
  plain string fields (name + short_name + description, lang +
  dir, orientation, non-string throws); URL fields (start_url
  + scope, javascript:/data: rejected, empty rejected);
  display allowlist (standalone, minimal-ui, case-insensitive,
  reject 'tab'); colour fields (hex accept, with alpha, CSS
  name reject, rgba reject); icons array (single, multiple,
  all fields with purpose, order preserved, not-array throw,
  null icon throw, javascript: src throw, error identifies
  bad index); fail-fast (bad display before icons, bad
  theme_color); deterministic key ordering (alphabetical top-
  level, byte-identical output, input order doesn't matter,
  icon src first then alphabetical); pretty-print (2-space
  indent); purity (no input mutation); full real-world PWA
  example.

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only).

## Spec adherence

Implements W3C Web App Manifest (Working Draft).  Subset that
covers the fields needed for FM00 v0 PWA support.  No spec
divergences.

## v0 simplifications

- **No `shortcuts` field.**  PWA shortcut menu entries
  deferred to v1; rarely used in static-site context.
- **No `screenshots` field.**  Same reason — rarely set on
  blog / docs sites.
- **No `related_applications` / `prefer_related_applications`**
  — native-app discovery deferred.
- **No `protocol_handlers` / `share_target`**  — advanced
  install integration deferred.
- **No CSS colour syntax beyond hex.**  Named colours and
  rgb()/hsl() are spec-compliant but the determinism /
  CSS-parser-confusion tradeoff isn't worth it for v0.
- **No `window-controls-overlay` display mode.**  Newer spec
  extension; deferred.
