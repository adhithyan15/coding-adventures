/**
 * @coding-adventures/forme-collect-chronological
 *
 * Forme collector stage: `Stream<ContentNode>` → single `Collection`,
 * sorted by a frontmatter date field (descending — newest first), with
 * a derived URL route attached to each entry's overlay.
 *
 *   consumes:    streamOf(Kinds.ContentNode)
 *   produces:    Kinds.Collection
 *   capabilities: []                ← pure transform
 *   configSchema: { name?: string; dateField?: string; slugField?: string;
 *                   routeTemplate?: string }
 *
 * === Why "collector" is its own stage ===
 *
 * Parsing happens per file (and is therefore stream-shaped).  But
 * deciding "what URL does this post live at" and "what order do they
 * appear on the index page" requires looking at ALL the parsed posts
 * — exactly the boundary FM00 calls a Collector.  Splitting the
 * concern out keeps the parser pure (input file → output node) and
 * lets the collector own ordering policy.
 *
 * === Sorting semantics ===
 *
 * The default date field is `"date"`.  Values are compared as strings
 * (ISO-8601 dates sort lexicographically) — this means
 * `"2026-05-15"` correctly comes before `"2026-05-16"`, and any
 * arbitrary date format that uses fixed-width numeric prefixes works
 * too.  Free-form dates ("May 15, 2026") will sort, but not the way
 * you want — the post template should commit to ISO-8601 and the
 * convention is documented in the README.
 *
 * Posts missing the date field are emitted with a sentinel date of
 * `"0000-01-01"` so they reliably land at the END of a descending
 * sort.  A warning is logged via `ctx.logger.warn` per dateless post
 * — silent dropping would be a sharp tool, sorting-to-back is
 * recoverable.
 *
 * Ties are broken by `sourcePath` (lexicographic ascending) so the
 * output is byte-deterministic across runs — important for cache
 * hashing and for reviewing diffs of generated indexes.
 *
 * === Route assignment ===
 *
 * Each entry gets a `route` formatted from `routeTemplate` (default
 * `"/blog/{slug}.html"`).  The slug is the explicit
 * `frontmatter[slugField]` value if present and non-empty; otherwise
 * derived from `sourcePath` via `slugify()`.  The route is stored on
 * the `CollectionEntry.route` field — downstream renderers consume it
 * to know where to emit each page.
 *
 * Note: `ContentNode.route` itself remains whatever the parser
 * emitted (typically `null`).  Routes belong to *collections* — a
 * single document can appear in multiple collections under different
 * routes.  Renderers walk the collection, not the bare node.
 *
 * === Spec adherence ===
 *
 * No deliberate divergences from FM00 §5.4.  v0 simplifications:
 *
 *   - Only string-typed date frontmatter values are supported (the
 *     parser-markdown v0 only produces strings anyway).
 *   - Only `{slug}` substitution in route templates.  `{year}` /
 *     `{month}` etc. wait until a v0.2 collector.
 *
 * @module index
 */

import {
  Kinds,
  streamOf,
  type ContentNode,
  type Collection,
  type CollectionEntry,
  type JsonValue,
  type OrderKey,
} from "@coding-adventures/forme-types";
import { defineStage } from "@coding-adventures/forme-stage";
import { slugify, formatRoute } from "./slug.js";

/**
 * v0 collector config.  Every field has a sensible default so a bare
 * `{}` (or `undefined`) works for the common blog case.
 */
export interface CollectChronologicalConfig {
  /** Collection name.  Default `"posts"`. */
  readonly name?: string;
  /** Frontmatter key carrying the publication date.  Default `"date"`. */
  readonly dateField?: string;
  /** Frontmatter key carrying an explicit slug.  Default `"slug"`. */
  readonly slugField?: string;
  /** Route template string.  Default `"/blog/{slug}.html"`. */
  readonly routeTemplate?: string;
}

const DEFAULT_DATE_FIELD = "date";
const DEFAULT_SLUG_FIELD = "slug";
const DEFAULT_ROUTE_TEMPLATE = "/blog/{slug}.html";
const DEFAULT_NAME = "posts";

/**
 * Sentinel date for posts missing the configured date frontmatter
 * field.  Sorts strictly before every plausible real date so missing-
 * date posts reliably land at the end of a *descending* sort.
 */
const SENTINEL_DATE = "0000-01-01";

const collectChronological = defineStage({
  name: "@coding-adventures/forme-collect-chronological",
  version: "0.1.0",
  apiVersion: 1,
  description: "Collect ContentNodes into a chronological Collection (newest first), assigning routes.",
  consumes: streamOf(Kinds.ContentNode),
  produces: Kinds.Collection,
  capabilities: [],
  configSchema: {
    type: "object",
    properties: {
      name:          { type: "string" },
      dateField:     { type: "string" },
      slugField:     { type: "string" },
      routeTemplate: { type: "string" },
    },
  },
  async run(rawInput, rawConfig, ctx) {
    const config = (rawConfig ?? {}) as CollectChronologicalConfig;
    const name          = config.name          ?? DEFAULT_NAME;
    const dateField     = config.dateField     ?? DEFAULT_DATE_FIELD;
    const slugField     = config.slugField     ?? DEFAULT_SLUG_FIELD;
    const routeTemplate = config.routeTemplate ?? DEFAULT_ROUTE_TEMPLATE;

    // 1. Buffer the stream.  Collectors are inherently un-streamable
    //    — we need every node before we can sort.  Memory is fine for
    //    the blog scale we're targeting (a typical site has < 10⁴
    //    posts; 10⁵ would still fit comfortably).
    const nodes: ContentNode[] = [];
    const stream = rawInput as AsyncIterable<ContentNode>;
    for await (const node of stream) {
      ctx.cancellation.throwIfCancelled();
      nodes.push(node);
    }

    // 2. Build a parallel array of { node, dateStr, slug, route, missingDate }
    //    so subsequent steps don't re-derive the same values.
    const annotated = nodes.map((node) => {
      const rawDate = node.frontmatter[dateField];
      const hasDate = typeof rawDate === "string" && rawDate.length > 0;
      const dateStr = hasDate ? (rawDate as string) : SENTINEL_DATE;
      const explicitSlug = node.frontmatter[slugField];
      const slug = (typeof explicitSlug === "string" && explicitSlug.length > 0)
        ? explicitSlug
        : slugify(node.sourcePath);
      const route = formatRoute(routeTemplate, slug);
      if (!hasDate) {
        ctx.logger.warn("forme-collect-chronological: post missing date frontmatter", {
          sourcePath: node.sourcePath,
          dateField,
        });
      }
      return { node, dateStr, slug, route };
    });

    // 3. Sort: date DESCENDING (newest first), tie-break by sourcePath
    //    ASCENDING (deterministic).
    annotated.sort((a, b) => {
      if (a.dateStr < b.dateStr) return 1;       // a older → a comes after
      if (a.dateStr > b.dateStr) return -1;      // a newer → a comes before
      if (a.node.sourcePath < b.node.sourcePath) return -1;
      if (a.node.sourcePath > b.node.sourcePath) return 1;
      return 0;
    });

    // 4. Build entries.  Each carries identity + revision references
    //    (the entry is a *pointer* into the node, not the node itself
    //    — millions-of-posts collections must stay cheap).
    const entries: CollectionEntry[] = annotated.map(({ node, dateStr, slug, route }) => {
      const overlay: Record<string, JsonValue> = {
        date: dateStr,
        slug,
        title: stringFromFrontmatter(node.frontmatter, "title") ?? slug,
      };
      const excerpt = stringFromFrontmatter(node.frontmatter, "excerpt");
      if (excerpt !== null) overlay.excerpt = excerpt;
      const orderKey: OrderKey = { kind: "date", value: dateStr };
      return {
        identity: node.identity,
        revision: node.revision,
        route,
        orderKey,
        overlay,
      };
    });

    // 5. Emit the single Collection.  `discriminant` is the FM00 §5.4
    //    grouping key — "chronological" means downstream stages can
    //    have a single pagination / index-rendering rule that handles
    //    every chronological collection (per-tag, per-year, etc.).
    const collection: Collection = {
      name,
      entries,
      discriminant: "chronological",
      meta: {},
    };
    ctx.logger.debug("forme-collect-chronological: collected", {
      name, count: entries.length,
    });
    return collection as never;
  },
});

/**
 * Tiny helper: return the value of `frontmatter[key]` if it's a
 * non-empty string, else null.  Keeps the entry-building code
 * readable.
 */
function stringFromFrontmatter(
  frontmatter: { readonly [k: string]: JsonValue | undefined },
  key: string,
): string | null {
  const v = frontmatter[key];
  return (typeof v === "string" && v.length > 0) ? v : null;
}

export default collectChronological;
export { collectChronological, slugify, formatRoute };
