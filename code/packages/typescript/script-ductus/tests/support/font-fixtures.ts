import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { boundsOf, parseFont } from "../../src/truetype";
import type { GlyphOutline } from "../../src/ductusview";

const FONT_DIR = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../../../../learning/human-languages/_fonts",
);

const cache = new Map<string, ReturnType<typeof parseFont>>();

export const parsedFont = (name: string): ReturnType<typeof parseFont> => {
  const cached = cache.get(name);
  if (cached !== undefined) return cached;
  const bytes = readFileSync(resolve(FONT_DIR, name));
  const buffer = bytes.buffer.slice(
    bytes.byteOffset,
    bytes.byteOffset + bytes.byteLength,
  ) as ArrayBuffer;
  const font = parseFont(buffer);
  cache.set(name, font);
  return font;
};

const outline = (font: string, character: string): GlyphOutline => {
  const glyph = parsedFont(font).glyphFor(character)!;
  return { path: glyph.path, bounds: boundsOf(glyph.contours) };
};

export const tamilOutline = (character: string) =>
  outline("NotoSansTamil-Static.ttf", character);
export const japaneseOutline = (character: string) =>
  outline("NotoSansJP-Subset.ttf", character);
export const naskhOutline = (character: string) =>
  outline("NotoNaskhArabic-Static.ttf", character);
export const hebrewOutline = (character: string) =>
  outline("NotoSansHebrew-Static.ttf", character);
export const chineseOutline = (character: string) =>
  outline("NotoSansSC-Subset.ttf", character);
export const devanagariOutline = (character: string) =>
  outline("NotoSansDevanagari-Static.ttf", character);
export const cyrillicOutline = (character: string) =>
  outline("NotoSansCyrillic-Static.ttf", character);
export const gujaratiOutline = (character: string) =>
  outline("NotoSansGujarati-Static.ttf", character);
export const teluguOutline = (character: string) =>
  outline("NotoSansTelugu-Static.ttf", character);
export const kannadaOutline = (character: string) =>
  outline("NotoSansKannada-Static.ttf", character);
export const malayalamOutline = (character: string) =>
  outline("NotoSansMalayalam-Static.ttf", character);
