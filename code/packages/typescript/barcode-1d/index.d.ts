import type { Barcode1DRenderConfig } from "@coding-adventures/barcode-layout-1d";
import type { PaintScene } from "@coding-adventures/paint-instructions";

export type Barcode1DSymbology =
  | "code39"
  | "codabar"
  | "code128"
  | "ean-13"
  | "itf"
  | "upc-a";

export interface Barcode1DBaseRequest {
  data: string;
  label?: string;
  metadata?: Record<string, string | number | boolean>;
  renderConfig?: Partial<Barcode1DRenderConfig>;
}

export interface Code39Barcode1DRequest extends Barcode1DBaseRequest {
  symbology: "code39";
}

export interface CodabarBarcode1DRequest extends Barcode1DBaseRequest {
  symbology: "codabar";
  start?: "A" | "B" | "C" | "D";
  stop?: "A" | "B" | "C" | "D";
}

export interface Code128Barcode1DRequest extends Barcode1DBaseRequest {
  symbology: "code128";
}

export interface Ean13Barcode1DRequest extends Barcode1DBaseRequest {
  symbology: "ean-13";
}

export interface ItfBarcode1DRequest extends Barcode1DBaseRequest {
  symbology: "itf";
}

export interface UpcABarcode1DRequest extends Barcode1DBaseRequest {
  symbology: "upc-a";
}

export type Barcode1DRequest =
  | Code39Barcode1DRequest
  | CodabarBarcode1DRequest
  | Code128Barcode1DRequest
  | Ean13Barcode1DRequest
  | ItfBarcode1DRequest
  | UpcABarcode1DRequest;

export const SUPPORTED_BARCODE_1D_SYMBOLOGIES: readonly Barcode1DSymbology[];

export function layoutBarcode1D(request: Barcode1DRequest): PaintScene;
export function renderPaintSceneToPng(scene: PaintScene | string): Uint8Array;
export function renderBarcode1DToPng(request: Barcode1DRequest): Uint8Array;
export function getPaintBackend(): string;
