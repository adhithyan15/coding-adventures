/**
 * @coding-adventures/wasm-leb128
 *
 * LEB128 variable-length integer encoding for WASM binary format
 *
 * This package is part of the coding-adventures monorepo, a ground-up
 * implementation of the computing stack from transistors to operating systems.
 *
 * Re-exports everything from the main implementation module so consumers
 * can simply write:
 *
 *   import { encodeUnsigned, decodeUnsigned, LEB128Error } from "@coding-adventures/wasm-leb128";
 */

export const VERSION = "0.1.0";

export {
  LEB128Error,
  decodeUnsigned,
  decodeSigned,
  encodeUnsigned,
  encodeSigned,
} from "./wasm_leb128.js";
