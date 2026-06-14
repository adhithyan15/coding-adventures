/**
 * types.ts — public signatures for the meta/link tag emitter.
 *
 * The emitter handles five distinct categories of `<head>` tag:
 *
 *   - `canonical`  — `<link rel="canonical">`.  At most one.
 *   - `prev` / `next` — pagination `<link>` tags.  At most one each.
 *   - `icons`      — favicons (`<link rel="icon">`), array.
 *   - `preload`    — resource hints (`<link rel="preload|prefetch|preconnect|dns-prefetch">`), array.
 *   - `meta`       — arbitrary `<meta name|http-equiv ...>` pairs, array.
 *
 * Every field is optional; an empty config emits the empty string.
 * The shape is intentionally flat — no nesting beyond one level —
 * so callers can build it by spreading individual feature modules
 * (an SEO helper sets `canonical`, a favicon helper sets `icons`,
 * etc.) and the emitter merges them at the call site.
 *
 * @module types
 */

/**
 * Favicon `<link rel="icon">` entry.
 *
 *   - `href`  — required.  http(s):// or root-relative `/path`.
 *   - `type`  — optional MIME (`image/png`, `image/svg+xml`, etc).
 *               Pure pass-through string; no allowlist (browsers
 *               accept many image MIMEs; we don't gatekeep here).
 *   - `sizes` — optional sizes attribute, e.g. `"32x32"` or
 *               `"any"` for SVG.
 *   - `rel`   — optional `rel` override.  Defaults to `"icon"`.
 *               Allowlist: `icon` | `shortcut icon` |
 *               `apple-touch-icon` | `mask-icon`.
 */
export interface IconLink {
  readonly href: string;
  readonly type?: string;
  readonly sizes?: string;
  readonly rel?: IconRel;
}

export type IconRel = "icon" | "shortcut icon" | "apple-touch-icon" | "mask-icon";

/**
 * Resource-hint `<link>` entry.  Covers preload / prefetch /
 * preconnect / dns-prefetch via the `rel` field.
 *
 *   - `href`        — required.  Same URL accept-list.
 *   - `rel`         — required.  `preload` | `prefetch` |
 *                     `preconnect` | `dns-prefetch` | `modulepreload`.
 *   - `as`          — required for `preload` / `modulepreload`;
 *                     ignored otherwise.  Allowlist of the
 *                     standard fetch destinations the platform
 *                     accepts here (`script`, `style`, `image`,
 *                     `font`, `fetch`, `document`, `audio`,
 *                     `video`, `track`, `worker`).
 *   - `type`        — optional MIME (e.g. `font/woff2`).
 *   - `crossorigin` — optional.  `anonymous` | `use-credentials`.
 *                     Bare boolean coercion: when `crossorigin`
 *                     is provided without value, emit
 *                     `crossorigin="anonymous"` (per HTML spec
 *                     default).  We require explicit string here
 *                     for clarity.
 */
export interface ResourceHint {
  readonly href: string;
  readonly rel: ResourceHintRel;
  readonly as?: ResourceHintAs;
  readonly type?: string;
  readonly crossorigin?: CrossOrigin;
}

export type ResourceHintRel =
  | "preload"
  | "prefetch"
  | "preconnect"
  | "dns-prefetch"
  | "modulepreload";

export type ResourceHintAs =
  | "script"
  | "style"
  | "image"
  | "font"
  | "fetch"
  | "document"
  | "audio"
  | "video"
  | "track"
  | "worker";

export type CrossOrigin = "anonymous" | "use-credentials";

/**
 * Arbitrary `<meta>` tag.  Exactly one of `name` / `httpEquiv`
 * must be provided.  `content` is required.
 *
 * Examples:
 *   - `{ name: "description", content: "..." }`
 *   - `{ name: "viewport", content: "width=device-width" }`
 *   - `{ httpEquiv: "content-security-policy", content: "..." }`
 *
 * `charset` is intentionally NOT supported here — emit
 * `<meta charset="utf-8">` via a higher-level head builder, or
 * pass `{ name: "charset", content: "utf-8" }` (which emits
 * `<meta name="charset" content="utf-8">` — not the canonical
 * form).  Charset-as-attribute is special-cased by HTML; we
 * keep this emitter narrowly focused on name/http-equiv pairs.
 */
export interface MetaTag {
  readonly name?: string;
  readonly httpEquiv?: string;
  readonly content: string;
}

/**
 * Top-level config consumed by `generateMetaLinkTags`.
 *
 * All fields optional.  Output order is deterministic and
 * matches the field order documented in {@link MetaLinkConfig}:
 *
 *   1. `<meta>` (in caller's array order)
 *   2. `<link rel="canonical">`
 *   3. `<link rel="prev">`
 *   4. `<link rel="next">`
 *   5. `<link rel="icon">` / variants (in caller's array order)
 *   6. resource hints (in caller's array order)
 *
 * Rationale: `<meta>` (charset, viewport, description) tends to
 * appear first in real `<head>` templates; canonical / pagination
 * are SEO signals (logically a unit); favicons follow; resource
 * hints last (they're performance hints, not document metadata).
 */
export interface MetaLinkConfig {
  readonly canonical?: string;
  readonly prev?: string;
  readonly next?: string;
  readonly icons?: readonly IconLink[];
  readonly preload?: readonly ResourceHint[];
  readonly meta?: readonly MetaTag[];
}
