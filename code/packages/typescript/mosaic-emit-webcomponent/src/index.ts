/**
 * @coding-adventures/mosaic-emit-webcomponent
 *
 * Web Components backend: emits Custom Element classes from MosaicIR.
 *
 * Usage:
 *
 *     import { MosaicWebComponentRenderer } from "@coding-adventures/mosaic-emit-webcomponent";
 *     import { MosaicVM } from "@coding-adventures/mosaic-vm";
 *     import { analyzeMosaic } from "@coding-adventures/mosaic-analyzer";
 *
 *     const ir = analyzeMosaic(source);
 *     const vm = new MosaicVM(ir);
 *     const result = vm.run(new MosaicWebComponentRenderer());
 *     // result.files[0].filename === "mosaic-component-name.ts"
 *     // result.files[0].content  === "// AUTO-GENERATED ..."
 */

export { MosaicWebComponentRenderer } from "./webcomponent-renderer.js";
