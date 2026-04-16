import type { WasmModule } from "@coding-adventures/wasm-types";
export declare const WASM_MAGIC: Uint8Array<ArrayBuffer>;
export declare const WASM_VERSION: Uint8Array<ArrayBuffer>;
export declare class WasmEncodeError extends Error {
    constructor(message: string);
}
export declare function encodeModule(module: WasmModule): Uint8Array;
//# sourceMappingURL=wasm_module_encoder.d.ts.map