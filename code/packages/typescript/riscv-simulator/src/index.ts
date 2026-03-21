/**
 * RISC-V Simulator -- full RV32I + M-mode extensions.
 *
 * Provides decoder, executor, CSR file, encoding helpers, and the
 * high-level RiscVSimulator class.
 */

export {
  RiscVSimulator,
  RiscVDecoder,
  RiscVExecutor,
  CSRFile,
  CSR_MSTATUS, CSR_MTVEC, CSR_MEPC, CSR_MCAUSE,
  MIE, CAUSE_ECALL_MMODE,
} from "./simulator.js";

export {
  encodeAddi, encodeSlti, encodeSltiu, encodeXori, encodeOri, encodeAndi,
  encodeSlli, encodeSrli, encodeSrai,
  encodeAdd, encodeSub, encodeSll, encodeSlt, encodeSltu,
  encodeXor, encodeSrl, encodeSra, encodeOr, encodeAnd,
  encodeLb, encodeLh, encodeLw, encodeLbu, encodeLhu,
  encodeSb, encodeSh, encodeSw,
  encodeBeq, encodeBne, encodeBlt, encodeBge, encodeBltu, encodeBgeu,
  encodeJal, encodeJalr, encodeLui, encodeAuipc,
  encodeEcall, encodeMret, encodeCsrrw, encodeCsrrs, encodeCsrrc,
  assemble,
} from "./encoding.js";

export * from "./opcodes.js";
