export {
  OP_END,
  OP_LOCAL_GET,
  OP_LOCAL_SET,
  OP_I32_CONST,
  OP_I32_ADD,
  OP_I32_SUB,
  WasmDecoder,
  WasmExecutor,
  WasmSimulator,
  encodeI32Const,
  encodeI32Add,
  encodeI32Sub,
  encodeLocalGet,
  encodeLocalSet,
  encodeEnd,
  assembleWasm,
} from "./simulator.js";

export type { WasmInstruction, WasmStepTrace } from "./simulator.js";
export type { WasmState } from "./state.js";
