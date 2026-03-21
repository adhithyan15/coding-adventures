/**
 * RISC-V Simulator -- Layer 4b of the computing stack.
 *
 * Minimal RV32I instruction decoder and executor.
 * Plugs into the CPU simulator via the decoder/executor protocol.
 */

export {
  RiscVSimulator,
  RiscVDecoder,
  RiscVExecutor,
  encodeAddi,
  encodeAdd,
  encodeEcall,
  assemble,
  OPCODE_OP_IMM,
  OPCODE_OP,
  OPCODE_SYSTEM,
} from "./simulator.js";
