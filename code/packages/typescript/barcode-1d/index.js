import { createRequire } from "module";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

import { layoutCodabar } from "@coding-adventures/codabar";
import { layoutCode128 } from "@coding-adventures/code128";
import { layoutCode39 } from "@coding-adventures/code39";
import { layoutEan13 } from "@coding-adventures/ean-13";
import { layoutItf } from "@coding-adventures/itf";
import { layoutUpcA } from "@coding-adventures/upc-a";

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(join(__dirname, "package.json"));
const native = require("./barcode_1d_native_node.node");

export const SUPPORTED_BARCODE_1D_SYMBOLOGIES = Object.freeze([
  "code39",
  "codabar",
  "code128",
  "ean-13",
  "itf",
  "upc-a",
]);

function baseLayoutOptions(request) {
  return {
    renderConfig: request.renderConfig,
    metadata: request.metadata,
    label: request.label,
  };
}

export function layoutBarcode1D(request) {
  switch (request.symbology) {
    case "code39":
      return layoutCode39(request.data, baseLayoutOptions(request));
    case "codabar":
      return layoutCodabar(request.data, {
        ...baseLayoutOptions(request),
        start: request.start,
        stop: request.stop,
      });
    case "code128":
      return layoutCode128(request.data, baseLayoutOptions(request));
    case "ean-13":
      return layoutEan13(request.data, baseLayoutOptions(request));
    case "itf":
      return layoutItf(request.data, baseLayoutOptions(request));
    case "upc-a":
      return layoutUpcA(request.data, baseLayoutOptions(request));
    default:
      throw new Error(`Unsupported 1D barcode symbology: ${request.symbology}`);
  }
}

export function renderPaintSceneToPng(scene) {
  const sceneJson = typeof scene === "string" ? scene : JSON.stringify(scene);
  return native.renderSceneToPng(sceneJson);
}

export function renderBarcode1DToPng(request) {
  return renderPaintSceneToPng(layoutBarcode1D(request));
}

export function getPaintBackend() {
  return native.getPaintBackend();
}
