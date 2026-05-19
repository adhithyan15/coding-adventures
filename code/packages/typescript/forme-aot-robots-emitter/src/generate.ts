/**
 * generate.ts — main `generateRobots` entry.
 *
 * Two-pass: validate every directive value first, then emit.
 * If anything throws, the caller never sees a partial
 * robots.txt.  Same fail-fast posture as
 * `forme-aot-sitemap-emitter`.
 *
 * Output line format (per https://www.robotstxt.org/orig.html
 * + Google extensions):
 *
 *   User-agent: <ua>
 *   Allow: <path>
 *   Disallow: <path>
 *   Crawl-delay: <n>
 *
 *   [blank line]
 *
 *   [next rule block...]
 *
 *   Sitemap: <url>
 *   Host: <host>
 *
 * Rule blocks separated by exactly one blank line.  Sitemap
 * and Host directives appear after all rule blocks (a single
 * blank line separates them from the last rule).
 *
 * @module generate
 */

import type { RobotsConfig, RobotsRule } from "./types.js";
import {
  validateCrawlDelay,
  validateDirectiveValue,
  validateHost,
  validateSitemapUrl,
} from "./validate.js";

/**
 * Generate the robots.txt content as a plain-text string.
 *
 * Throws `TypeError` synchronously on any validation failure
 * BEFORE any output is built.
 *
 * Reproducibility: same `config` → byte-identical output.
 * Input objects are never mutated.
 *
 * ```ts
 * generateRobots({
 *   rules: [
 *     { userAgent: "*",         disallow: ["/admin", "/private"] },
 *     { userAgent: "Googlebot", allow: ["/"]                    },
 *   ],
 *   sitemap: "https://example.com/sitemap.xml",
 *   host: "example.com",
 * });
 * ```
 */
export function generateRobots(config: RobotsConfig): string {
  // ─── Validation pass ────────────────────────────────────
  const rules: RobotsRule[] = Array.isArray(config.rules) ? [...config.rules] : [];
  const validatedRules = rules.map((rule, i) => validateRule(rule, i));
  const validatedSitemaps = normaliseSitemap(config.sitemap).map(validateSitemapUrl);
  const validatedHost = config.host === undefined ? undefined : validateHost(config.host);

  // ─── Emit pass ──────────────────────────────────────────
  const blocks: string[] = [];

  for (let i = 0; i < validatedRules.length; i++) {
    blocks.push(emitRuleBlock(validatedRules[i]!));
  }

  if (validatedSitemaps.length > 0 || validatedHost !== undefined) {
    const tail: string[] = [];
    for (let i = 0; i < validatedSitemaps.length; i++) {
      tail.push(`Sitemap: ${validatedSitemaps[i]}`);
    }
    if (validatedHost !== undefined) {
      tail.push(`Host: ${validatedHost}`);
    }
    blocks.push(tail.join("\n"));
  }

  // Rule blocks (and the trailing sitemap/host block) joined
  // by exactly one blank line.  Trailing newline included so
  // the file ends cleanly per Unix convention.
  return blocks.length === 0 ? "" : blocks.join("\n\n") + "\n";
}

/**
 * Internal scratch shape for a fully-validated rule.  We
 * normalise `userAgent` to always-an-array so the emit pass
 * has one shape to deal with.
 */
interface ValidatedRule {
  readonly userAgents: readonly string[];
  readonly allow: readonly string[];
  readonly disallow: readonly string[];
  readonly crawlDelay: number | undefined;
}

function validateRule(rule: RobotsRule, index: number): ValidatedRule {
  if (rule === null || typeof rule !== "object") {
    throw new TypeError(
      `forme-aot-robots-emitter: rules[${index}] must be a non-null object; got ${typeof rule}`,
    );
  }

  const uaList = normaliseUserAgent(rule.userAgent, index);
  if (uaList.length === 0) {
    throw new TypeError(
      `forme-aot-robots-emitter: rules[${index}].userAgent must yield at least one value`,
    );
  }
  const validatedUAs = uaList.map((ua) =>
    validateDirectiveValue(ua, `rules[${index}].userAgent`)
  );

  const allowList = rule.allow ?? [];
  if (!Array.isArray(allowList)) {
    throw new TypeError(
      `forme-aot-robots-emitter: rules[${index}].allow must be an array; got ${typeof allowList}`,
    );
  }
  const validatedAllow = allowList.map((p, j) =>
    validateDirectiveValue(p, `rules[${index}].allow[${j}]`)
  );

  const disallowList = rule.disallow ?? [];
  if (!Array.isArray(disallowList)) {
    throw new TypeError(
      `forme-aot-robots-emitter: rules[${index}].disallow must be an array; got ${typeof disallowList}`,
    );
  }
  const validatedDisallow = disallowList.map((p, j) =>
    validateDirectiveValue(p, `rules[${index}].disallow[${j}]`)
  );

  const validatedCrawlDelay = rule.crawlDelay === undefined
    ? undefined
    : validateCrawlDelay(rule.crawlDelay);

  return {
    userAgents: validatedUAs,
    allow: validatedAllow,
    disallow: validatedDisallow,
    crawlDelay: validatedCrawlDelay,
  };
}

function normaliseUserAgent(value: string | readonly string[], index: number): readonly string[] {
  if (typeof value === "string") return [value];
  if (Array.isArray(value)) return value;
  throw new TypeError(
    `forme-aot-robots-emitter: rules[${index}].userAgent must be a string or string[]; got ${typeof value}`,
  );
}

function normaliseSitemap(value: string | readonly string[] | undefined): readonly string[] {
  if (value === undefined) return [];
  if (typeof value === "string") return [value];
  if (Array.isArray(value)) return value;
  throw new TypeError(
    `forme-aot-robots-emitter: sitemap must be a string, string[], or undefined; got ${typeof value}`,
  );
}

/**
 * Emit one rule block.  Order of lines:
 *   1. All `User-agent:` lines (one per UA).
 *   2. All `Allow:` lines.
 *   3. All `Disallow:` lines.
 *   4. `Crawl-delay:` line if present.
 *
 * Why allow before disallow?  The most-specific-rule-wins
 * convention used by major crawlers makes `Allow` exceptions
 * to `Disallow` rules more readable when emitted first.  The
 * spec itself doesn't mandate an order; we pick one for
 * determinism.
 */
function emitRuleBlock(rule: ValidatedRule): string {
  const lines: string[] = [];
  for (const ua of rule.userAgents) {
    lines.push(`User-agent: ${ua}`);
  }
  for (const path of rule.allow) {
    lines.push(`Allow: ${path}`);
  }
  for (const path of rule.disallow) {
    lines.push(`Disallow: ${path}`);
  }
  if (rule.crawlDelay !== undefined) {
    lines.push(`Crawl-delay: ${rule.crawlDelay}`);
  }
  return lines.join("\n");
}
