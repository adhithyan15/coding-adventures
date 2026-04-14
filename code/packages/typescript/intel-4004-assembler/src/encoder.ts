export class AssemblerError extends Error {}

function parseRegister(name: string): number {
  const match = /^R([0-9]|1[0-5])$/i.exec(name);
  if (!match) {
    throw new AssemblerError(`Invalid register name: '${name}'`);
  }
  return Number.parseInt(match[1], 10);
}

function parsePair(name: string): number {
  const match = /^P([0-7])$/i.exec(name);
  if (!match) {
    throw new AssemblerError(`Invalid register pair name: '${name}'`);
  }
  return Number.parseInt(match[1], 10);
}

function parseImmediate(value: string): number {
  const parsed = value.toLowerCase().startsWith("0x")
    ? Number.parseInt(value, 16)
    : Number.parseInt(value, 10);
  if (Number.isNaN(parsed)) {
    throw new AssemblerError(`Invalid numeric literal: '${value}'`);
  }
  return parsed;
}

function resolveOperand(operand: string, symbols: ReadonlyMap<string, number>, pc: number): number {
  if (operand === "$") {
    return pc;
  }
  if (operand.toLowerCase().startsWith("0x") || /^-?\d+$/.test(operand)) {
    return parseImmediate(operand);
  }
  const symbol = symbols.get(operand);
  if (symbol === undefined) {
    throw new AssemblerError(`Undefined label: '${operand}'`);
  }
  return symbol;
}

const FIXED_OPCODES = new Map<string, number>([
  ["NOP", 0x00],
  ["HLT", 0x01],
  ["WRM", 0xe0],
  ["WMP", 0xe1],
  ["WRR", 0xe2],
  ["WR0", 0xe4],
  ["WR1", 0xe5],
  ["WR2", 0xe6],
  ["WR3", 0xe7],
  ["SBM", 0xe8],
  ["RDM", 0xe9],
  ["RDR", 0xea],
  ["ADM", 0xeb],
  ["RD0", 0xec],
  ["RD1", 0xed],
  ["RD2", 0xee],
  ["RD3", 0xef],
  ["CLB", 0xf0],
  ["CLC", 0xf1],
  ["IAC", 0xf2],
  ["CMC", 0xf3],
  ["CMA", 0xf4],
  ["RAL", 0xf5],
  ["RAR", 0xf6],
  ["TCC", 0xf7],
  ["DAC", 0xf8],
  ["TCS", 0xf9],
  ["STC", 0xfa],
  ["DAA", 0xfb],
  ["KBP", 0xfc],
  ["DCL", 0xfd],
]);

function expectOperands(mnemonic: string, operands: readonly string[], count: number): void {
  if (operands.length !== count) {
    throw new AssemblerError(
      `${mnemonic} expects ${count} operand(s), got ${operands.length}: ${JSON.stringify(operands)}`,
    );
  }
}

function checkRange(name: string, value: number, lo: number, hi: number): void {
  if (value < lo || value > hi) {
    throw new AssemblerError(
      `${name} value ${value} (0x${value.toString(16).toUpperCase()}) is out of range [${lo}, ${hi}]`,
    );
  }
}

export function instructionSize(mnemonic: string): number {
  if (FIXED_OPCODES.has(mnemonic)) {
    return 1;
  }
  if (["INC", "ADD", "SUB", "LD", "XCH", "BBL", "LDM", "SRC", "FIN", "JIN"].includes(mnemonic)) {
    return 1;
  }
  if (["JCN", "FIM", "JUN", "JMS", "ISZ", "ADD_IMM"].includes(mnemonic)) {
    return 2;
  }
  if (mnemonic === "ORG") {
    return 0;
  }
  throw new AssemblerError(`Unknown mnemonic: '${mnemonic}'`);
}

export function encodeInstruction(
  mnemonic: string,
  operands: readonly string[],
  symbols: ReadonlyMap<string, number>,
  pc: number,
): Uint8Array {
  if (FIXED_OPCODES.has(mnemonic)) {
    expectOperands(mnemonic, operands, 0);
    return new Uint8Array([FIXED_OPCODES.get(mnemonic) ?? 0]);
  }

  if (mnemonic === "ORG") {
    return new Uint8Array();
  }

  if (mnemonic === "LDM") {
    expectOperands(mnemonic, operands, 1);
    const immediate = resolveOperand(operands[0], symbols, pc);
    checkRange("LDM", immediate, 0, 15);
    return new Uint8Array([0xd0 | immediate]);
  }

  if (mnemonic === "BBL") {
    expectOperands(mnemonic, operands, 1);
    const immediate = resolveOperand(operands[0], symbols, pc);
    checkRange("BBL", immediate, 0, 15);
    return new Uint8Array([0xc0 | immediate]);
  }

  if (mnemonic === "INC") {
    expectOperands(mnemonic, operands, 1);
    return new Uint8Array([0x60 | parseRegister(operands[0])]);
  }

  if (mnemonic === "ADD") {
    expectOperands(mnemonic, operands, 1);
    return new Uint8Array([0x80 | parseRegister(operands[0])]);
  }

  if (mnemonic === "SUB") {
    expectOperands(mnemonic, operands, 1);
    return new Uint8Array([0x90 | parseRegister(operands[0])]);
  }

  if (mnemonic === "LD") {
    expectOperands(mnemonic, operands, 1);
    return new Uint8Array([0xa0 | parseRegister(operands[0])]);
  }

  if (mnemonic === "XCH") {
    expectOperands(mnemonic, operands, 1);
    return new Uint8Array([0xb0 | parseRegister(operands[0])]);
  }

  if (mnemonic === "SRC") {
    expectOperands(mnemonic, operands, 1);
    return new Uint8Array([0x20 | (2 * parsePair(operands[0]) + 1)]);
  }

  if (mnemonic === "FIN") {
    expectOperands(mnemonic, operands, 1);
    return new Uint8Array([0x30 | (2 * parsePair(operands[0]))]);
  }

  if (mnemonic === "JIN") {
    expectOperands(mnemonic, operands, 1);
    return new Uint8Array([0x30 | (2 * parsePair(operands[0]) + 1)]);
  }

  if (mnemonic === "FIM") {
    expectOperands(mnemonic, operands, 2);
    const pair = parsePair(operands[0]);
    const immediate = resolveOperand(operands[1], symbols, pc);
    checkRange("FIM", immediate, 0, 255);
    return new Uint8Array([0x20 | (2 * pair), immediate]);
  }

  if (mnemonic === "JCN") {
    expectOperands(mnemonic, operands, 2);
    const cond = resolveOperand(operands[0], symbols, pc);
    const address = resolveOperand(operands[1], symbols, pc);
    checkRange("JCN condition", cond, 0, 15);
    checkRange("JCN address", address, 0, 0xfff);
    return new Uint8Array([0x10 | cond, address & 0xff]);
  }

  if (mnemonic === "JUN") {
    expectOperands(mnemonic, operands, 1);
    const address = resolveOperand(operands[0], symbols, pc);
    checkRange("JUN", address, 0, 0xfff);
    return new Uint8Array([0x40 | ((address >> 8) & 0xf), address & 0xff]);
  }

  if (mnemonic === "JMS") {
    expectOperands(mnemonic, operands, 1);
    const address = resolveOperand(operands[0], symbols, pc);
    checkRange("JMS", address, 0, 0xfff);
    return new Uint8Array([0x50 | ((address >> 8) & 0xf), address & 0xff]);
  }

  if (mnemonic === "ISZ") {
    expectOperands(mnemonic, operands, 2);
    const register = parseRegister(operands[0]);
    const address = resolveOperand(operands[1], symbols, pc);
    checkRange("ISZ address", address, 0, 0xff);
    return new Uint8Array([0x70 | register, address & 0xff]);
  }

  if (mnemonic === "ADD_IMM") {
    expectOperands(mnemonic, operands, 3);
    const register = parseRegister(operands[1]);
    const immediate = resolveOperand(operands[2], symbols, pc);
    checkRange("ADD_IMM immediate", immediate, 0, 15);
    return new Uint8Array([0xd0 | immediate, 0x80 | register]);
  }

  throw new AssemblerError(`Unknown mnemonic: '${mnemonic}'`);
}
