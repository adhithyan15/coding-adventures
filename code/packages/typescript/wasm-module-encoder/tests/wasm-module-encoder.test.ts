import { describe, expect, it } from "vitest";
import { makeFuncType, ValueType, ExternalKind, WasmModule } from "@coding-adventures/wasm-types";
import { getOpcodeByName } from "@coding-adventures/wasm-opcodes";
import { encodeSigned } from "@coding-adventures/wasm-leb128";
import { WasmModuleParser } from "@coding-adventures/wasm-module-parser";
import { validate } from "@coding-adventures/wasm-validator";

import { encodeModule, WASM_MAGIC, WASM_VERSION } from "../src/index.js";

describe("encodeModule", () => {
  it("encodes a simple exported function into a valid wasm binary", () => {
    const i32Const = getOpcodeByName("i32.const")!.opcode;
    const end = getOpcodeByName("end")!.opcode;

    const module = new WasmModule();
    module.types.push(makeFuncType([], [ValueType.I32]));
    module.functions.push(0);
    module.exports.push({
      name: "answer",
      kind: ExternalKind.FUNCTION,
      index: 0,
    });
    module.code.push({
      locals: [],
      code: new Uint8Array([i32Const, ...encodeSigned(42), end]),
    });

    const bytes = encodeModule(module);
    expect(bytes.slice(0, 4)).toEqual(WASM_MAGIC);
    expect(bytes.slice(4, 8)).toEqual(WASM_VERSION);

    const parsed = new WasmModuleParser().parse(bytes);
    expect(parsed.exports[0]?.name).toBe("answer");
    expect(() => validate(parsed)).not.toThrow();
  });
});
