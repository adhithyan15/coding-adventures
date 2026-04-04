/**
 * Mosaic Analyzer — validates Mosaic ASTs and produces typed MosaicIR.
 *
 * Usage:
 *
 *     import { analyzeMosaic } from "@coding-adventures/mosaic-analyzer";
 *
 *     const ir = analyzeMosaic(`
 *       component Label { slot text: text; Text { content: @text; } }
 *     `);
 *     console.log(ir.component.name);       // "Label"
 *     console.log(ir.component.slots[0].type); // { kind: "text" }
 */

export { analyzeMosaic, analyzeMosaicAST, AnalysisError } from "./analyzer.js";

export type {
  MosaicIR,
  MosaicComponent,
  MosaicImport,
  MosaicSlot,
  MosaicType,
  MosaicNode,
  MosaicChild,
  MosaicProperty,
  MosaicValue,
} from "./ir.js";
