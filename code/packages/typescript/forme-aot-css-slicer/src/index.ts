/**
 * @coding-adventures/forme-aot-css-slicer
 *
 * Per-page CSS slicer for the Forme AOT compiler.  Takes a
 * `StyleDocument` and a per-page `usedRuleIds` map (the renderer's
 * `usedStyle` accumulator from FM01 §2.3.6) and emits a content-
 * addressed CSS artefact per page (FM06 §3).
 *
 * ```ts
 * import { slicePerPage, defaultScopePrefix } from "@coding-adventures/forme-aot-css-slicer";
 *
 * const { artefacts } = slicePerPage(doc, [
 *   { id: "/index.html",     usedRuleIds: ["body", "headline"] },
 *   { id: "/about.html",     usedRuleIds: ["body", "headline", "nav"] },
 *   { id: "/blog/post.html", usedRuleIds: ["body", "headline", "nav", "code"] },
 * ], {
 *   activeContexts: ["screen"],
 * });
 *
 * for (const [pageId, artefact] of artefacts) {
 *   console.log(pageId, artefact.byteSize, "bytes,", artefact.sha256.slice(0, 8));
 *   fs.writeFileSync(routeToFilePath(pageId), artefact.css);
 * }
 * ```
 *
 * The sha256 is over the **unscoped** CSS bytes — pages with
 * identical `usedRuleIds` produce identical fingerprints and a
 * downstream cache can deduplicate by content while still serving
 * per-page-scoped CSS to the browser.
 *
 * @module index
 */

export { slicePerPage, defaultScopePrefix } from "./slicer.js";
export type {
  PageSlice, SliceOptions, SliceResult, CssArtifact,
} from "./slicer.js";
