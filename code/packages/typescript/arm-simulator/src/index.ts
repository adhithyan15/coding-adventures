/**
 * ARM Simulator -- Layer 4 of the computing stack.
 *
 * ARMv7 instruction decoder and executor.
 * Plugs into the CPU simulator via the decoder/executor protocol.
 */

export { ARMSimulator } from "./simulator.js";
export {
  ARMDecoder,
  ARMExecutor,
  assemble,
  encodeMovImm,
  encodeAdd,
  encodeSub,
  encodeHlt,
  COND_AL,
  OPCODE_MOV,
  OPCODE_ADD,
  OPCODE_SUB,
  HLT_INSTRUCTION,
} from "./simulator.js";
