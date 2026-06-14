/**
 * @coding-adventures/forme-router
 *
 * Forme route-derivation stage: `Stream<ContentNode>` →
 * `Stream<ContentNode>` with `route` populated.
 *
 *   consumes:    streamOf(Kinds.ContentNode)
 *   produces:    streamOf(Kinds.ContentNode)
 *   capabilities: []                ← pure transform
 *   configSchema: { routeTemplate?, slugField? }
 *
 * Replaces the duplicated route-derivation logic that lives in
 * `forme-collect-chronological/src/slug.ts` and
 * `forme-render-static/src/slug.ts`.  When this stage runs upstream
 * of those, they can read `node.route` directly instead of
 * re-deriving from `sourcePath`.
 *
 * === Slug derivation priority ===
 *
 * For each `ContentNode`:
 *
 *   1. If `node.frontmatter[slugField]` is a non-empty string, use it.
 *   2. Otherwise, slugify `node.sourcePath`.
 *
 * Step 1 lets authors override the URL via `slug:` in their
 * frontmatter (the convention used by every static site generator
 * in existence).  Step 2 is the fallback for posts that don't
 * specify one explicitly.
 *
 * === Identity & revision discipline ===
 *
 * `LogicalId` is preserved unchanged — routing is metadata, not
 * content.  `revision` is preserved too: setting `route` is a
 * pipeline-stage decoration, not a content edit.  Two runs with
 * the same input produce the same output (cacheable).
 *
 * === Spec adherence ===
 *
 * No deliberate divergences from FM00 §5.4.  See the package
 * CHANGELOG for v0 simplifications and the wiring plan.
 *
 * @module index
 */

import {
  Kinds,
  streamOf,
  type ContentNode,
  type JsonValue,
} from "@coding-adventures/forme-types";
import { defineStage } from "@coding-adventures/forme-stage";
import { slugify, formatRoute } from "./slug.js";

/** v0 config.  Every field optional. */
export interface RouterConfig {
  /** Route template.  Default `/blog/{slug}.html`.  Currently
   *  `{slug}` is the only supported substitution. */
  readonly routeTemplate?: string;
  /** Frontmatter key carrying the explicit slug override.
   *  Default `"slug"`. */
  readonly slugField?: string;
}

const DEFAULT_ROUTE_TEMPLATE = "/blog/{slug}.html";
const DEFAULT_SLUG_FIELD = "slug";

const router = defineStage({
  name: "@coding-adventures/forme-router",
  version: "0.1.0",
  apiVersion: 1,
  description: "Derive a URL route per ContentNode (frontmatter.slug → sourcePath fallback) and set ContentNode.route.",
  consumes: streamOf(Kinds.ContentNode),
  produces: streamOf(Kinds.ContentNode),
  capabilities: [],
  configSchema: {
    type: "object",
    properties: {
      routeTemplate: { type: "string" },
      slugField:     { type: "string" },
    },
  },
  async *run(rawInput, rawConfig, ctx) {
    const config        = (rawConfig ?? {}) as RouterConfig;
    const routeTemplate = config.routeTemplate ?? DEFAULT_ROUTE_TEMPLATE;
    const slugField     = config.slugField     ?? DEFAULT_SLUG_FIELD;
    const stream        = rawInput as AsyncIterable<ContentNode>;

    for await (const node of stream) {
      ctx.cancellation.throwIfCancelled();

      // 1. Explicit frontmatter slug, if present and non-empty.
      const explicit = node.frontmatter[slugField] as JsonValue | undefined;
      const slug = (typeof explicit === "string" && explicit.length > 0)
        ? explicit
        : slugify(node.sourcePath);

      // 2. Format the route via template substitution.
      const route = formatRoute(routeTemplate, slug);

      // 3. Emit a new ContentNode with `route` set.  All other
      //    fields are passed through unchanged — including
      //    `revision`, which intentionally does NOT change just
      //    because routing happened.  Two runs over the same
      //    inputs produce identical outputs.
      const routedNode: ContentNode = {
        ...node,
        route,
      };
      yield routedNode as never;
    }

    ctx.logger.debug("forme-router: stream complete");
  },
});

export default router;
export { router, slugify, formatRoute };
