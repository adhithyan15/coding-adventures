/**
 * @coding-adventures/mosaic-emit-react
 *
 * React backend: emits TSX functional components from MosaicIR.
 *
 * Usage:
 *
 *     import { MosaicReactRenderer } from "@coding-adventures/mosaic-emit-react";
 *     import { MosaicVM } from "@coding-adventures/mosaic-vm";
 *     import { analyzeMosaic } from "@coding-adventures/mosaic-analyzer";
 *
 *     const ir = analyzeMosaic(source);
 *     const vm = new MosaicVM(ir);
 *     const result = vm.run(new MosaicReactRenderer());
 *     // result.files[0].filename === "ComponentName.tsx"
 *     // result.files[0].content  === "// AUTO-GENERATED ..."
 */

export { MosaicReactRenderer } from "./react-renderer.js";
