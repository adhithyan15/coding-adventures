/**
 * Attach the one reviewed browser module used by the live Forme dogfood.
 *
 * The generic filesystem resolver discovers document images. Executable
 * assets are intentionally different: selecting one is an authority decision,
 * so this product adapter names the exact source, identity, role, and page.
 * Downstream loading remains generic, while forme-render-static and
 * forme-emit-site-fs enforce the route allowlist and reviewed digest.
 */

import {
  KERNEL_API_VERSION,
  Kinds,
  streamOf,
  type AssetRef,
  type ContentNode,
  type JsonValue,
  type LogicalId,
} from "@coding-adventures/forme-types";
import { computeRevisionId } from "@coding-adventures/forme-identity";
import { defineStage } from "@coding-adventures/forme-stage";

export const PIPELINE_ISLAND_ASSET_ID =
  "01952c0d-7e63-7000-8000-000000000202" as LogicalId;
export const PIPELINE_ISLAND_SOURCE_PATH = "assets/pipeline-steps.js";
export const INTERACTIVE_POST_SOURCE_PATH = "2026-05-15-hello-forme.md";

const pipelineIslandRef: AssetRef = Object.freeze({
  id: PIPELINE_ISLAND_ASSET_ID,
  nodePath: Object.freeze([]),
  role: "script",
  sourcePath: PIPELINE_ISLAND_SOURCE_PATH,
});

const attachBlogInteractivity = defineStage({
  name: "@coding-adventures/blog-attach-interactivity",
  version: "0.1.0",
  apiVersion: KERNEL_API_VERSION,
  description: "Attach the reviewed Hello, Forme progressive-enhancement module.",
  consumes: streamOf(Kinds.ContentNode),
  produces: streamOf(Kinds.ContentNode),
  capabilities: [],
  configSchema: { type: "object", properties: {} },
  async *run(rawInput, _rawConfig, ctx) {
    for await (const node of rawInput as AsyncIterable<ContentNode>) {
      ctx.cancellation.throwIfCancelled();
      if (node.sourcePath !== INTERACTIVE_POST_SOURCE_PATH) {
        yield node as never;
        continue;
      }

      const existing = node.assetRefs.filter(ref =>
        ref.id === PIPELINE_ISLAND_ASSET_ID ||
        ref.sourcePath === PIPELINE_ISLAND_SOURCE_PATH);
      if (existing.length > 0) {
        const [existingRef] = existing;
        if (
          existing.length !== 1 ||
          existingRef === undefined ||
          existingRef.id !== pipelineIslandRef.id ||
          existingRef.role !== pipelineIslandRef.role ||
          existingRef.sourcePath !== pipelineIslandRef.sourcePath ||
          existingRef.nodePath.length !== 0
        ) {
          throw new Error(
            "blog-attach-interactivity: conflicting script asset claim for the pipeline-step island",
          );
        }
        yield node as never;
        continue;
      }

      const assetRefs = Object.freeze([...node.assetRefs, pipelineIslandRef]);
      const revision = computeRevisionId({
        document: node.document as unknown as JsonValue,
        frontmatter: node.frontmatter,
        assetRefs: assetRefs as unknown as JsonValue,
        sourcePath: node.sourcePath,
      });
      yield { ...node, revision, assetRefs } as never;
    }
  },
});

export default attachBlogInteractivity;
export { attachBlogInteractivity };
