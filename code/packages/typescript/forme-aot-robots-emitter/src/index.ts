/**
 * @coding-adventures/forme-aot-robots-emitter
 *
 * Emit `robots.txt` from a structured `RobotsConfig` per
 * https://www.robotstxt.org/orig.html + Google extensions.
 * Pure transform — returns plain-text string; caller writes
 * it wherever.
 *
 * ```ts
 * import { generateRobots } from "@coding-adventures/forme-aot-robots-emitter";
 *
 * const txt = generateRobots({
 *   rules: [
 *     { userAgent: "*",         disallow: ["/admin", "/private"] },
 *     { userAgent: "Googlebot", allow: ["/"]                    },
 *   ],
 *   sitemap: "https://example.com/sitemap.xml",
 *   host: "example.com",
 * });
 *
 * fs.writeFileSync("dist/robots.txt", txt);
 * ```
 *
 * Validation runs BEFORE emission.  Any error throws
 * `TypeError` synchronously and no partial output reaches the
 * caller.  Header-injection chars (CR / LF / NUL / DEL / other
 * C0) in any value are rejected — this is the line-format
 * analogue of HTTP header injection.
 *
 * Twelfth FM00 v0 stage package — joins the FM00 v0 cluster.
 *
 * @module index
 */

export { generateRobots } from "./generate.js";
export {
  validateDirectiveValue,
  validateCrawlDelay,
  validateSitemapUrl,
  validateHost,
} from "./validate.js";
export type { RobotsConfig, RobotsRule } from "./types.js";
