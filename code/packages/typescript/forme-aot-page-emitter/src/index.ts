/**
 * @coding-adventures/forme-aot-page-emitter
 *
 * Per-page artefact emitter for the Forme AOT compiler (FM06 §5).
 *
 * ```ts
 * import { emitPages } from "@coding-adventures/forme-aot-page-emitter";
 * import { slicePerPage } from "@coding-adventures/forme-aot-css-slicer";
 *
 * const { artefacts } = slicePerPage(doc, pages, { activeContexts: ["screen"] });
 * const { written, totalBytes } = await emitPages("./dist", artefacts, {
 *   writeHtml: true,
 *   htmlBody: (pageId) => `<h1>${pageId}</h1>`,
 * });
 * console.log(`wrote ${written.size} pages, ${totalBytes} bytes total`);
 * ```
 *
 * @module index
 */

export { emitPages } from "./page-emitter.js";
export type { EmitIO, EmitOptions, EmitResult, PageEmit } from "./page-emitter.js";
