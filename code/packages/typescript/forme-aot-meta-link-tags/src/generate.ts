/**
 * generate.ts — main `generateMetaLinkTags` entry.
 *
 * Two-pass fail-fast design:
 *
 *   1. Walk the entire config validating every field.  Build a
 *      flat "resolved" list of tag descriptors in deterministic
 *      output order.  Any validation error throws BEFORE step 2.
 *   2. Render each descriptor to its tag string.  Join with
 *      newlines.  No trailing newline.
 *
 * Output order (matches the documented `MetaLinkConfig` order):
 *
 *     <meta>* → canonical → prev → next → icons* → hints*
 *
 * Attribute order within each tag is fixed (see `renderXxx`
 * below).  Same input → byte-identical output.
 *
 * @module generate
 */

import { escapeHtmlAttr } from "./escape.js";
import type {
  IconLink,
  MetaLinkConfig,
  MetaTag,
  ResourceHint,
} from "./types.js";
import {
  validateCrossOrigin,
  validateHintAs,
  validateHintRel,
  validateIconRel,
  validateOptionalString,
  validateUrl,
} from "./validate.js";

// Internal resolved (post-validation) descriptors.  These are
// what the render pass consumes; building them up-front means
// the render pass can't fail.
interface ResolvedMeta {
  readonly kind: "meta";
  readonly nameAttr: "name" | "http-equiv";
  readonly nameValue: string;
  readonly content: string;
}
interface ResolvedSimpleLink {
  readonly kind: "simple-link";
  readonly rel: "canonical" | "prev" | "next";
  readonly href: string;
}
interface ResolvedIcon {
  readonly kind: "icon";
  readonly rel: string;
  readonly href: string;
  readonly type?: string;
  readonly sizes?: string;
}
interface ResolvedHint {
  readonly kind: "hint";
  readonly rel: string;
  readonly href: string;
  readonly as?: string;
  readonly type?: string;
  readonly crossorigin?: string;
}
type Resolved = ResolvedMeta | ResolvedSimpleLink | ResolvedIcon | ResolvedHint;

/**
 * Generate the concatenated `<head>` tag string from a
 * structured config.  Returns the empty string for an empty
 * config (no fields set).  Throws `TypeError` synchronously
 * for any validation failure before emitting anything.
 *
 * ```ts
 * generateMetaLinkTags({
 *   canonical: "https://example.com/post",
 *   meta: [
 *     { name: "viewport", content: "width=device-width" },
 *     { name: "description", content: "Hello" },
 *   ],
 *   icons: [{ href: "/favicon.png", type: "image/png", sizes: "32x32" }],
 *   preload: [{ href: "/main.js", rel: "preload", as: "script" }],
 * });
 * ```
 */
export function generateMetaLinkTags(config: MetaLinkConfig): string {
  if (config === null || typeof config !== "object") {
    throw new TypeError(
      `forme-aot-meta-link-tags: config must be a non-null object; got ${typeof config}`,
    );
  }

  const resolved: Resolved[] = [];

  // 1. <meta> tags (in caller's array order, first).
  if (config.meta !== undefined) {
    if (!Array.isArray(config.meta)) {
      throw new TypeError(
        `forme-aot-meta-link-tags: meta must be an array; got ${typeof config.meta}`,
      );
    }
    for (let i = 0; i < config.meta.length; i++) {
      resolved.push(validateMeta(config.meta[i]!, i));
    }
  }

  // 2. canonical.
  if (config.canonical !== undefined) {
    resolved.push({
      kind: "simple-link",
      rel: "canonical",
      href: validateUrl(config.canonical, "canonical"),
    });
  }

  // 3. prev.
  if (config.prev !== undefined) {
    resolved.push({
      kind: "simple-link",
      rel: "prev",
      href: validateUrl(config.prev, "prev"),
    });
  }

  // 4. next.
  if (config.next !== undefined) {
    resolved.push({
      kind: "simple-link",
      rel: "next",
      href: validateUrl(config.next, "next"),
    });
  }

  // 5. icons.
  if (config.icons !== undefined) {
    if (!Array.isArray(config.icons)) {
      throw new TypeError(
        `forme-aot-meta-link-tags: icons must be an array; got ${typeof config.icons}`,
      );
    }
    for (let i = 0; i < config.icons.length; i++) {
      resolved.push(validateIcon(config.icons[i]!, i));
    }
  }

  // 6. preload / hints.
  if (config.preload !== undefined) {
    if (!Array.isArray(config.preload)) {
      throw new TypeError(
        `forme-aot-meta-link-tags: preload must be an array; got ${typeof config.preload}`,
      );
    }
    for (let i = 0; i < config.preload.length; i++) {
      resolved.push(validateHint(config.preload[i]!, i));
    }
  }

  // Render pass — no validation failures possible past this point.
  const lines: string[] = new Array(resolved.length);
  for (let i = 0; i < resolved.length; i++) {
    lines[i] = renderResolved(resolved[i]!);
  }
  return lines.join("\n");
}

function validateMeta(m: MetaTag, i: number): ResolvedMeta {
  if (m === null || typeof m !== "object") {
    throw new TypeError(
      `forme-aot-meta-link-tags: meta[${i}] must be a non-null object; got ${typeof m}`,
    );
  }
  const hasName = m.name !== undefined;
  const hasHttpEquiv = m.httpEquiv !== undefined;
  if (hasName === hasHttpEquiv) {
    throw new TypeError(
      `forme-aot-meta-link-tags: meta[${i}] must have exactly one of name/httpEquiv; got name=${
        hasName ? JSON.stringify(m.name) : "undefined"
      } httpEquiv=${hasHttpEquiv ? JSON.stringify(m.httpEquiv) : "undefined"}`,
    );
  }
  const nameValue = hasName ? validateOptionalString(m.name, `meta[${i}].name`)!
                             : validateOptionalString(m.httpEquiv, `meta[${i}].httpEquiv`)!;
  if (nameValue.length === 0) {
    throw new TypeError(
      `forme-aot-meta-link-tags: meta[${i}].${hasName ? "name" : "httpEquiv"} must be non-empty`,
    );
  }
  if (typeof m.content !== "string") {
    throw new TypeError(
      `forme-aot-meta-link-tags: meta[${i}].content must be a string; got ${typeof m.content}`,
    );
  }
  return {
    kind: "meta",
    nameAttr: hasName ? "name" : "http-equiv",
    nameValue,
    content: m.content,
  };
}

function validateIcon(icon: IconLink, i: number): ResolvedIcon {
  if (icon === null || typeof icon !== "object") {
    throw new TypeError(
      `forme-aot-meta-link-tags: icons[${i}] must be a non-null object; got ${typeof icon}`,
    );
  }
  const href = validateUrl(icon.href, `icons[${i}].href`);
  const rel = icon.rel === undefined ? "icon" : validateIconRel(icon.rel, `icons[${i}].rel`);
  const type = validateOptionalString(icon.type, `icons[${i}].type`);
  const sizes = validateOptionalString(icon.sizes, `icons[${i}].sizes`);
  return { kind: "icon", rel, href, type, sizes };
}

function validateHint(hint: ResourceHint, i: number): ResolvedHint {
  if (hint === null || typeof hint !== "object") {
    throw new TypeError(
      `forme-aot-meta-link-tags: preload[${i}] must be a non-null object; got ${typeof hint}`,
    );
  }
  const href = validateUrl(hint.href, `preload[${i}].href`);
  const rel = validateHintRel(hint.rel, `preload[${i}].rel`);
  // `as` required for preload / modulepreload; rejected as an error
  // if MISSING (helps callers — preload without `as` is broken in
  // most browsers).  For prefetch / preconnect / dns-prefetch it's
  // ignored if present (we still validate the value to catch typos).
  let asValue: string | undefined;
  if (hint.as !== undefined) {
    asValue = validateHintAs(hint.as, `preload[${i}].as`);
  } else if (rel === "preload" || rel === "modulepreload") {
    throw new TypeError(
      `forme-aot-meta-link-tags: preload[${i}].as is required when rel="${rel}"`,
    );
  }
  // Suppress `as` on non-preload variants — it's invalid HTML there.
  const asOut = (rel === "preload" || rel === "modulepreload") ? asValue : undefined;
  const type = validateOptionalString(hint.type, `preload[${i}].type`);
  const crossorigin = hint.crossorigin === undefined
    ? undefined
    : validateCrossOrigin(hint.crossorigin, `preload[${i}].crossorigin`);
  return { kind: "hint", rel, href, as: asOut, type, crossorigin };
}

function renderResolved(r: Resolved): string {
  switch (r.kind) {
    case "meta":      return renderMeta(r);
    case "simple-link": return renderSimpleLink(r);
    case "icon":      return renderIcon(r);
    case "hint":      return renderHint(r);
  }
}

function renderMeta(r: ResolvedMeta): string {
  return `<meta ${r.nameAttr}="${escapeHtmlAttr(r.nameValue)}" content="${escapeHtmlAttr(r.content)}">`;
}

function renderSimpleLink(r: ResolvedSimpleLink): string {
  return `<link rel="${r.rel}" href="${escapeHtmlAttr(r.href)}">`;
}

function renderIcon(r: ResolvedIcon): string {
  const parts: string[] = [`<link rel="${escapeHtmlAttr(r.rel)}"`];
  if (r.type !== undefined)  parts.push(`type="${escapeHtmlAttr(r.type)}"`);
  if (r.sizes !== undefined) parts.push(`sizes="${escapeHtmlAttr(r.sizes)}"`);
  parts.push(`href="${escapeHtmlAttr(r.href)}">`);
  return parts.join(" ");
}

function renderHint(r: ResolvedHint): string {
  const parts: string[] = [`<link rel="${escapeHtmlAttr(r.rel)}"`];
  if (r.as !== undefined)          parts.push(`as="${escapeHtmlAttr(r.as)}"`);
  if (r.type !== undefined)        parts.push(`type="${escapeHtmlAttr(r.type)}"`);
  if (r.crossorigin !== undefined) parts.push(`crossorigin="${escapeHtmlAttr(r.crossorigin)}"`);
  parts.push(`href="${escapeHtmlAttr(r.href)}">`);
  return parts.join(" ");
}
