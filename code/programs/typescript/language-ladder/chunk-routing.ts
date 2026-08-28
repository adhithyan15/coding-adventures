/**
 * Decide whether a module belongs to the lazy handwriting-tools chunk.
 *
 * Vite supplies native-looking module ids, so the same repository path can
 * arrive with POSIX or Windows separators. Normalizing once also lets the
 * boundary checks below stay readable. The strokes owner is deliberately a
 * directory tree: script owners may add stable per-glyph descendants without
 * having to widen this bundling rule again.
 */
export function isHandwritingToolsModuleId(moduleId: string): boolean {
  const normalized = moduleId.replaceAll("\\", "/").split(/[?#]/, 1)[0];
  const sourceMatch = /(?:^|\/)script-ductus\/src\/(.+)$/.exec(normalized);
  const sourcePath = sourceMatch?.[1];

  if (!sourcePath) return false;
  if (sourcePath === "ductusview.ts" || sourcePath === "truetype.ts") {
    return true;
  }
  if (!sourcePath.endsWith(".ts")) return false;

  const pathWithoutExtension = sourcePath.slice(0, -".ts".length);
  const segments = pathWithoutExtension.split("/");

  return (
    segments[0] === "strokes" &&
    segments.every(
      (segment) => segment.length > 0 && segment !== "." && segment !== "..",
    )
  );
}

/**
 * Keep the two diagnostic inputs in one lazy chunk without widening the group
 * to the authored shard files behind their virtual modules.
 *
 * The browser sees one reconstructed chapter-capability module and one
 * reconstructed book-hash module per track. It must never see one module per
 * generated chapter owner: that would turn today's 1,088 records into 1,088
 * requests and make the bundle shape grow with chapters instead of tracks.
 */
export function isBookLedgerModuleId(moduleId: string): boolean {
  const normalized = moduleId.replaceAll("\\", "/").split(/[?#]/, 1)[0];
  return /^(?:\0)?virtual:human-language-ledger\/(?:chapters|book-hashes)\/[a-z][a-z0-9-]*$/.test(
    normalized,
  );
}
