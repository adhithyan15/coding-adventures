/**
 * types.ts — public signatures for the script tag emitter.
 *
 * Emits a single HTML `<script src="...">` tag string per
 * `ScriptTag` entry.  Inline scripts (`<script>...code...</script>`)
 * are out of scope here — that's a separate path (security
 * posture is fundamentally different: external `src` runs after
 * URL validation + SRI; inline executes whatever the caller
 * passes verbatim, so it needs a dedicated trust boundary).
 *
 * @module types
 */

/**
 * `<script>` `type` attribute.
 *
 *   - `"module"`     — ES module script.  Implicitly `defer`-like
 *                      semantics in browsers.
 *   - `"importmap"`  — JSON import map.  No `src` semantics
 *                      typically (importmaps are inline) but
 *                      modern browsers accept `<script
 *                      type="importmap" src="...">` for external
 *                      maps; we permit it.
 *
 * Classic scripts (no `type=`) are emitted by omitting the
 * `type` field.  Other MIME types (legacy `text/javascript`,
 * `application/javascript`) are intentionally NOT in the
 * allowlist — they're equivalent to omission in modern browsers
 * and accepting them widens the surface for typo-driven bugs.
 */
export type ScriptType = "module" | "importmap";

/**
 * `crossorigin` attribute value.  Same allowlist as
 * `forme-aot-meta-link-tags`.
 */
export type CrossOrigin = "anonymous" | "use-credentials";

/**
 * `referrerpolicy` attribute value.  Spec-defined enum from the
 * Referrer Policy Living Standard.
 */
export type ReferrerPolicy =
  | "no-referrer"
  | "no-referrer-when-downgrade"
  | "origin"
  | "origin-when-cross-origin"
  | "same-origin"
  | "strict-origin"
  | "strict-origin-when-cross-origin"
  | "unsafe-url";

/**
 * One `<script>` tag descriptor.
 *
 *   - `src`            — required.  http(s):// or root-relative.
 *   - `type`           — optional.  `"module"` | `"importmap"`.
 *                        Omit for classic script.
 *   - `integrity`      — optional SRI string.  Must be of the
 *                        form `sha256-<base64>`,
 *                        `sha384-<base64>`, or `sha512-<base64>`,
 *                        possibly space-separated for multiple
 *                        hashes.
 *   - `crossorigin`    — optional.  `"anonymous"` | `"use-credentials"`.
 *                        Required by browsers when `integrity`
 *                        is set on a cross-origin resource; we
 *                        emit it verbatim, leaving the policy
 *                        choice to the caller.
 *   - `async`          — optional.  Emit `async` boolean attr.
 *   - `defer`          — optional.  Emit `defer` boolean attr.
 *   - `nomodule`       — optional.  Emit `nomodule` boolean attr.
 *                        Useful for ES-module-aware browsers to
 *                        skip a legacy fallback bundle.
 *   - `referrerpolicy` — optional.  Spec-allowlist value.
 */
export interface ScriptTag {
  readonly src: string;
  readonly type?: ScriptType;
  readonly integrity?: string;
  readonly crossorigin?: CrossOrigin;
  readonly async?: boolean;
  readonly defer?: boolean;
  readonly nomodule?: boolean;
  readonly referrerpolicy?: ReferrerPolicy;
}
