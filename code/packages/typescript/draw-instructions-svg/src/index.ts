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

function xmlEscape(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}

function metadataToAttributes(metadata?: DrawMetadata): string {
  if (metadata === undefined) {
    return "";
  }

  return Object.entries(metadata)
    .map(([key, value]) => ` data-${key}="${xmlEscape(String(value))}"`)
    .join("");
}

function renderRect(instruction: DrawRectInstruction): string {
  return `  <rect x="${instruction.x}" y="${instruction.y}" width="${instruction.width}" height="${instruction.height}" fill="${xmlEscape(instruction.fill)}"${metadataToAttributes(instruction.metadata)} />`;
}

function renderText(instruction: DrawTextInstruction): string {
  return `  <text x="${instruction.x}" y="${instruction.y}" text-anchor="${instruction.align}" font-family="${xmlEscape(instruction.fontFamily)}" font-size="${instruction.fontSize}" fill="${xmlEscape(instruction.fill)}"${metadataToAttributes(instruction.metadata)}>${xmlEscape(instruction.value)}</text>`;
}

function renderGroup(instruction: DrawGroupInstruction): string {
  const children = instruction.children.map(renderInstruction).join("\n");
  return [`  <g${metadataToAttributes(instruction.metadata)}>`, children, "  </g>"].join("\n");
}

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

export function renderSvg(scene: DrawScene): string {
  return SVG_RENDERER.render(scene);
}
