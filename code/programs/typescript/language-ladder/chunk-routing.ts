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
