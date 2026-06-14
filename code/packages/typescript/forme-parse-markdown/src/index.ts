/**
 * @coding-adventures/forme-parse-markdown
 *
 * Pure-transform Forme stage: `ContentSource` → `ContentNode`.
 *
 *   consumes:    Kinds.ContentSource
 *   produces:    Kinds.ContentNode
 *   capabilities: []      ← no I/O, no env, no shell
 *   configSchema: { gfm?: boolean }
 *
 * Decodes the input bytes as UTF-8, splits off any YAML-style
 * frontmatter (see `frontmatter.ts` for the v0 grammar — strings only,
 * no nesting), and hands the remaining body to
 * `@coding-adventures/gfm-parser`.  The result is wrapped in a
 * `ContentNode` carrying through the source's `LogicalId` identity and
 * a freshly computed `RevisionId` derived from the parsed document +
 * frontmatter + source path.
 *
 * === Identity & revision discipline ===
 *
 * - **identity** is *passed through* from the source.  Identity is the
 *   document's persistent name — it survives re-parsing, reformatting,
 *   even moving to a different filesystem.  Two re-parses of the same
 *   source are the same logical document.
 *
 * - **revision** is recomputed from `{ documentJson, frontmatter,
 *   sourcePath }` so that:
 *     * a content edit changes it (DocumentNode shape changes);
 *     * a frontmatter edit changes it;
 *     * moving the file to a new path changes it (downstream
 *       collectors / route assigners care about path).
 *   Two parses of the same input bytes from the same path produce
 *   byte-identical revisions, so the cache layer can short-circuit.
 *
 * === Spec adherence ===
 *
 * No deliberate divergences from FM00/FM01.  v0 simplifications:
 *
 *   - Frontmatter grammar is intentionally tiny (see `frontmatter.ts`).
 *     Quoted strings, arrays, nested maps are deferred to a future
 *     sibling stage that wraps a real YAML parser.
 *   - `route` is always emitted as `null` — assignment is a collector's
 *     job (`forme-collect-chronological`), not the parser's.
 *   - `assetRefs` is always `[]`.  Asset extraction (FM00 §5.3) is a
 *     separate stage that will run between parse and collect.
 *
 * @module index
 */

import {
  Kinds,
  type ContentSource,
  type ContentNode,
  type AssetRef,
  type JsonValue,
} from "@coding-adventures/forme-types";
import { defineStage } from "@coding-adventures/forme-stage";
import { computeRevisionId } from "@coding-adventures/forme-identity";
import { parse as parseGfm } from "@coding-adventures/gfm-parser";
import { splitFrontmatter } from "./frontmatter.js";

/**
 * v0 config surface.  `gfm` defaults to true — the wrapped parser
 * doesn't currently support disabling GFM extensions, but we accept
 * (and ignore) the flag so the surface is forward-compatible.
 */
export interface ParseMarkdownConfig {
  readonly gfm?: boolean;
}

const decoder = new TextDecoder("utf-8", { fatal: false });

/**
 * Decode source bytes as UTF-8, stripping an optional BOM.
 *
 * The BOM strip is deliberate: GitHub, VS Code, and Microsoft tools
 * routinely emit BOM-prefixed UTF-8.  Leaving the BOM in place breaks
 * the frontmatter-at-byte-0 check below for no good reason.
 */
function decodeBody(bytes: Uint8Array): string {
  const text = decoder.decode(bytes);
  return text.startsWith("﻿") ? text.slice(1) : text;
}

const parseMarkdown = defineStage({
  name: "@coding-adventures/forme-parse-markdown",
  version: "0.1.0",
  apiVersion: 1,
  description: "Parse a Markdown ContentSource into a ContentNode (DocumentNode + frontmatter).",
  consumes: Kinds.ContentSource,
  produces: Kinds.ContentNode,
  capabilities: [],
  configSchema: {
    type: "object",
    properties: {
      gfm: { type: "boolean" },
    },
  },
  run(rawInput, _rawConfig, _ctx) {
    const source = rawInput as ContentSource;

    // 1. Decode + split.  Both steps are total (no exceptions on
    //    malformed input — bad frontmatter degrades silently).
    const text = decodeBody(source.bytes);
    const { data, body } = splitFrontmatter(text);

    // 2. Parse the body.  gfm-parser is pure: same input → same AST.
    //    We do NOT catch here — a parse error is a real bug and the
    //    orchestrator's failure semantics handle it cleanly.
    const document = parseGfm(body);

    // 3. Build the frontmatter as a frozen Record<string, JsonValue>.
    //    Values are strings in v0 (see frontmatter.ts).  Casting to
    //    JsonValue is safe — string is a JsonValue.
    const frontmatter: Record<string, JsonValue> = {};
    for (const k of Object.keys(data)) {
      frontmatter[k] = data[k]!;
    }

    // 4. Recompute revision.  Including sourcePath here means moving
    //    a post (e.g. posts/foo.md → archive/foo.md) invalidates the
    //    cached node — collectors usually key off path, so a move *is*
    //    a content change from their perspective.
    const revision = computeRevisionId({
      documentJson: document as unknown as JsonValue,
      frontmatter,
      sourcePath: source.path,
    });

    // 5. Assemble the ContentNode.  assetRefs is empty in v0 — a later
    //    stage will walk the document and extract image / video refs.
    const assetRefs: readonly AssetRef[] = [];
    const node: ContentNode = {
      identity: source.identity,
      revision,
      document,
      frontmatter,
      route: null,
      assetRefs,
      sourcePath: source.path,
    };
    return node as never;
  },
});

export default parseMarkdown;
export { parseMarkdown, splitFrontmatter };
