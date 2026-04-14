import { AssemblerError, encodeInstruction, instructionSize } from "./encoder.js";
import { lexProgram, type ParsedLine } from "./lexer.js";

function parseNumber(value: string): number {
  const parsed = value.toLowerCase().startsWith("0x")
    ? Number.parseInt(value, 16)
    : Number.parseInt(value, 10);
  if (Number.isNaN(parsed)) {
    throw new AssemblerError(`Invalid address literal: '${value}'`);
  }
  return parsed;
}

export class Intel4004Assembler {
  assemble(text: string): Uint8Array {
    const lines = lexProgram(text);
    const symbols = this.pass1(lines);
    return this.pass2(lines, symbols);
  }

  private pass1(lines: readonly ParsedLine[]): Map<string, number> {
    const symbols = new Map<string, number>();
    let pc = 0;

    for (const parsed of lines) {
      if (parsed.label) {
        symbols.set(parsed.label, pc);
      }

      if (!parsed.mnemonic) {
        continue;
      }

      if (parsed.mnemonic === "ORG") {
        if (parsed.operands.length === 0) {
          throw new AssemblerError("ORG requires an address operand");
        }
        const address = parseNumber(parsed.operands[0]);
        if (address > 0xfff) {
          throw new AssemblerError(`ORG address 0x${address.toString(16).toUpperCase()} exceeds 0xFFF`);
        }
        pc = address;
        continue;
      }

      pc += instructionSize(parsed.mnemonic);
    }

    return symbols;
  }

  private pass2(lines: readonly ParsedLine[], symbols: ReadonlyMap<string, number>): Uint8Array {
    const output: number[] = [];
    let pc = 0;

    for (const parsed of lines) {
      if (!parsed.mnemonic) {
        continue;
      }

      if (parsed.mnemonic === "ORG") {
        if (parsed.operands.length === 0) {
          throw new AssemblerError("ORG requires an address operand");
        }
        const address = parseNumber(parsed.operands[0]);
        if (address > 0xfff) {
          throw new AssemblerError(`ORG address 0x${address.toString(16).toUpperCase()} exceeds 0xFFF`);
        }
        while (pc < address) {
          output.push(0x00);
          pc += 1;
        }
        continue;
      }

      const encoded = encodeInstruction(parsed.mnemonic, parsed.operands, symbols, pc);
      output.push(...encoded);
      pc += encoded.length;
    }

    return Uint8Array.from(output);
  }
}

export function assemble(text: string): Uint8Array {
  return new Intel4004Assembler().assemble(text);
}
