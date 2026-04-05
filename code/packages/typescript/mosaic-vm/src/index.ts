/**
 * mosaic-vm — Generic tree-walking driver for Mosaic compiler backends.
 *
 * Usage:
 *
 *     import { MosaicVM } from "@coding-adventures/mosaic-vm";
 *     import { analyzeMosaic } from "@coding-adventures/mosaic-analyzer";
 *
 *     const ir = analyzeMosaic(source);
 *     const vm = new MosaicVM(ir);
 *     const result = vm.run(myRenderer);
 */

export { MosaicVM, MosaicVMError } from "./vm.js";

export type {
  MosaicRenderer,
  MosaicEmitResult,
  ResolvedValue,
  ResolvedProperty,
  SlotContext,
} from "./types.js";

// Re-export MosaicIR types so backends only need to depend on mosaic-vm,
// not mosaic-analyzer directly.
export type {
  MosaicIR,
  MosaicComponent,
  MosaicSlot,
  MosaicType,
  MosaicNode,
  MosaicChild,
  MosaicProperty,
  MosaicValue,
  MosaicImport,
} from "@coding-adventures/mosaic-analyzer";
