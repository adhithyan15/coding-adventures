/**
 * @coding-adventures/draw-instructions-svg
 *
 * This package is intentionally boring in the best possible way.
 *
 * It knows how to serialize a generic draw scene to SVG, and nothing more.
 * It should not contain barcode rules, graph rules, or any other producer
 * domain logic. That separation is the whole reason this package exists.
 */
import type {
  DrawGroupInstruction,
  DrawInstruction,
  DrawMetadata,
  DrawRectInstruction,
  DrawRenderer,
  DrawScene,
  DrawTextInstruction,
} from "@coding-adventures/draw-instructions";

export const VERSION = "0.1.0";

/** Escape user-provided text before embedding it into XML. */
function xmlEscape(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

/**
 * Metadata is serialized as `data-*` attributes.
 *
 * This is a nice compromise:
 * - SVG stays valid and readable
 * - semantic information survives into the output
 * - browser tooling and visualizers can inspect the metadata later
 */
function metadataToAttributes(metadata?: DrawMetadata): string {
  if (metadata === undefined) {
    return "";
  }

  return Object.entries(metadata)
    .map(([key, value]) => ` data-${key}="${xmlEscape(String(value))}"`)
    .join("");
}

/** Serialize one rectangle instruction into one SVG `<rect>`. */
function renderRect(instruction: DrawRectInstruction): string {
  return `  <rect x="${instruction.x}" y="${instruction.y}" width="${instruction.width}" height="${instruction.height}" fill="${xmlEscape(instruction.fill)}"${metadataToAttributes(instruction.metadata)} />`;
}

/** Serialize one text instruction into one SVG `<text>`. */
function renderText(instruction: DrawTextInstruction): string {
  return `  <text x="${instruction.x}" y="${instruction.y}" text-anchor="${instruction.align}" font-family="${xmlEscape(instruction.fontFamily)}" font-size="${instruction.fontSize}" fill="${xmlEscape(instruction.fill)}"${metadataToAttributes(instruction.metadata)}>${xmlEscape(instruction.value)}</text>`;
}

/** Serialize a group recursively into an SVG `<g>`. */
function renderGroup(instruction: DrawGroupInstruction): string {
  const children = instruction.children.map(renderInstruction).join("\n");
  return [`  <g${metadataToAttributes(instruction.metadata)}>`, children, "  </g>"].join("\n");
}

/** Dispatch one generic instruction to the matching SVG serializer. */
function renderInstruction(instruction: DrawInstruction): string {
  switch (instruction.kind) {
    case "rect":
      return renderRect(instruction);
    case "text":
      return renderText(instruction);
    case "group":
      return renderGroup(instruction);
  }
}

export const SVG_RENDERER: DrawRenderer<string> = {
  render(scene: DrawScene): string {
    // The scene-level label becomes the SVG accessibility label when present.
    const instructions = scene.instructions.map(renderInstruction).join("\n");
    const label = scene.metadata?.label === undefined
      ? "draw instructions scene"
      : xmlEscape(String(scene.metadata.label));

    return [
      `<svg xmlns="http://www.w3.org/2000/svg" width="${scene.width}" height="${scene.height}" viewBox="0 0 ${scene.width} ${scene.height}" role="img" aria-label="${label}">`,
      `  <rect x="0" y="0" width="${scene.width}" height="${scene.height}" fill="${xmlEscape(scene.background)}" />`,
      instructions,
      `</svg>`,
    ].join("\n");
  },
};

/** Convenience wrapper for the common case: scene in, SVG string out. */
export function renderSvg(scene: DrawScene): string {
  return SVG_RENDERER.render(scene);
}
