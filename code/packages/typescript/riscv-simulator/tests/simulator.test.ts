import { describe, it, expect } from "vitest";
import {
  RiscVSimulator,
  CSR_MTVEC, CSR_MEPC, CSR_MCAUSE, CSR_MSTATUS,
  MIE, CAUSE_ECALL_MMODE,
  encodeAddi, encodeAdd, encodeSub,
  encodeAnd, encodeOr, encodeXor,
  encodeSlli, encodeSrli, encodeSrai,
  encodeSlti,
  encodeLw, encodeSw, encodeLb, encodeSb, encodeLbu,
  encodeBeq, encodeBne,
  encodeJal, encodeJalr, encodeLui, encodeAuipc,
  encodeEcall, encodeMret,
  encodeCsrrw, encodeCsrrs,
  assemble,
} from "../src/index.js";

describe("RiscVSimulator", () => {
  describe("I-type arithmetic", () => {
    it("addi loads immediate into register", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([encodeAddi(1, 0, 42), encodeEcall()]));
      expect(sim.cpu.registers.read(1)).toBe(42);
    });

    it("addi with negative immediate", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([encodeAddi(1, 0, 100), encodeAddi(2, 1, -30), encodeEcall()]));
      expect(sim.cpu.registers.read(2)).toBe(70);
    });

    it("slti sets 1 when rs1 < imm (signed)", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([
        encodeAddi(1, 0, -5),
        encodeSlti(2, 1, 0),
        encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(2)).toBe(1);
    });

    it("slli shifts left", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([encodeAddi(1, 0, 3), encodeSlli(2, 1, 4), encodeEcall()]));
      expect(sim.cpu.registers.read(2)).toBe(48);
    });

    it("srli shifts right logical", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([encodeAddi(1, 0, 48), encodeSrli(2, 1, 4), encodeEcall()]));
      expect(sim.cpu.registers.read(2)).toBe(3);
    });

    it("srai shifts right arithmetic", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([
        encodeAddi(1, 0, -16),
        encodeSrai(2, 1, 2),
        encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(2)).toBe(0xfffffffc >>> 0);
    });
  });

  describe("R-type arithmetic", () => {
    it("add computes sum", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([
        encodeAddi(1, 0, 10), encodeAddi(2, 0, 20),
        encodeAdd(3, 1, 2), encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(3)).toBe(30);
    });

    it("sub computes difference", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([
        encodeAddi(1, 0, 50), encodeAddi(2, 0, 30),
        encodeSub(3, 1, 2), encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(3)).toBe(20);
    });

    it("and/or/xor bitwise operations", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([
        encodeAddi(1, 0, 0b1100), encodeAddi(2, 0, 0b1010),
        encodeAnd(3, 1, 2), encodeOr(4, 1, 2), encodeXor(5, 1, 2),
        encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(3)).toBe(0b1000);
      expect(sim.cpu.registers.read(4)).toBe(0b1110);
      expect(sim.cpu.registers.read(5)).toBe(0b0110);
    });
  });

  describe("x0 hardwired to zero", () => {
    it("writes to x0 are ignored", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([encodeAddi(0, 0, 42), encodeEcall()]));
      expect(sim.cpu.registers.read(0)).toBe(0);
    });
  });

  describe("Load/Store", () => {
    it("sw and lw round-trip a word", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([
        encodeAddi(1, 0, 0x42),
        encodeSw(1, 0, 0x100),
        encodeLw(2, 0, 0x100),
        encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(2)).toBe(0x42);
    });

    it("sb and lbu round-trip a byte", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([
        encodeAddi(1, 0, 0xab),
        encodeSb(1, 0, 0x100),
        encodeLbu(2, 0, 0x100),
        encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(2)).toBe(0xab);
    });

    it("lb sign-extends negative bytes", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([
        encodeAddi(1, 0, 0xff),
        encodeSb(1, 0, 0x100),
        encodeLb(2, 0, 0x100),
        encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(2)).toBe(0xffffffff >>> 0);
    });
  });

  describe("Branches", () => {
    it("beq takes branch when equal", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([
        encodeAddi(1, 0, 5), encodeAddi(2, 0, 5),
        encodeBeq(1, 2, 8),
        encodeAddi(3, 0, 99),
        encodeAddi(4, 0, 42),
        encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(3)).toBe(0);
      expect(sim.cpu.registers.read(4)).toBe(42);
    });

    it("backward branch creates a loop", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([
        encodeAddi(1, 0, 0), encodeAddi(2, 0, 5),
        encodeAddi(1, 1, 1),
        encodeBne(1, 2, -4),
        encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(1)).toBe(5);
    });
  });

  describe("Jumps", () => {
    it("jal stores return address and jumps", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([
        encodeJal(1, 8),
        encodeAddi(2, 0, 99),
        encodeAddi(3, 0, 42),
        encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(1)).toBe(4);
      expect(sim.cpu.registers.read(2)).toBe(0);
      expect(sim.cpu.registers.read(3)).toBe(42);
    });

    it("jalr jumps to register + imm", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([
        encodeAddi(5, 0, 12),
        encodeJalr(1, 5, 0),
        encodeAddi(2, 0, 99),
        encodeAddi(3, 0, 42),
        encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(1)).toBe(8);
      expect(sim.cpu.registers.read(2)).toBe(0);
      expect(sim.cpu.registers.read(3)).toBe(42);
    });
  });

  describe("Upper immediate", () => {
    it("lui loads upper 20 bits", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([encodeLui(1, 0x12345), encodeEcall()]));
      expect(sim.cpu.registers.read(1)).toBe(0x12345000 >>> 0);
    });

    it("lui + addi constructs full 32-bit value", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([
        encodeLui(1, 0x12345), encodeAddi(1, 1, 0x678), encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(1)).toBe(0x12345678 >>> 0);
    });

    it("auipc adds upper immediate to PC", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([encodeAuipc(1, 1), encodeEcall()]));
      expect(sim.cpu.registers.read(1)).toBe(0x1000);
    });
  });

  describe("CSR instructions", () => {
    it("csrrw swaps register and CSR", () => {
      const sim = new RiscVSimulator();
      sim.csr.write(CSR_MSTATUS, 0x42);
      sim.run(assemble([
        encodeAddi(1, 0, 0x99),
        encodeCsrrw(2, CSR_MSTATUS, 1),
        encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(2)).toBe(0x42);
      expect(sim.csr.read(CSR_MSTATUS)).toBe(0x99);
    });

    it("csrrs reads and sets bits", () => {
      const sim = new RiscVSimulator();
      sim.csr.write(CSR_MSTATUS, 0b0100);
      sim.run(assemble([
        encodeAddi(1, 0, 0b0011),
        encodeCsrrs(2, CSR_MSTATUS, 1),
        encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(2)).toBe(0b0100);
      expect(sim.csr.read(CSR_MSTATUS)).toBe(0b0111);
    });
  });

  describe("ecall / mret trap handling", () => {
    it("ecall halts when mtvec is 0", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([encodeAddi(1, 0, 42), encodeEcall()]));
      expect(sim.cpu.halted).toBe(true);
    });

    it("ecall traps to mtvec when configured", () => {
      const sim = new RiscVSimulator();
      sim.csr.write(CSR_MTVEC, 20);
      sim.csr.write(CSR_MSTATUS, MIE);
      sim.run(assemble([
        encodeAddi(1, 0, 42),
        encodeEcall(),
        encodeAddi(2, 0, 99),
        encodeAddi(3, 0, 88),
        encodeAddi(4, 0, 77),
        // trap handler at address 20:
        encodeAddi(5, 0, 55),
        encodeEcall(), // halts (MIE cleared)
      ]));
      expect(sim.cpu.registers.read(1)).toBe(42);
      expect(sim.cpu.registers.read(5)).toBe(55);
      // Second ecall (at addr 24) overwrites mepc
      expect(sim.csr.read(CSR_MEPC)).toBe(24);
      expect(sim.csr.read(CSR_MCAUSE)).toBe(CAUSE_ECALL_MMODE);
    });
  });

  describe("encoding helpers", () => {
    it("assemble produces little-endian bytes", () => {
      const bytes = assemble([0x12345678]);
      expect(bytes).toEqual([0x78, 0x56, 0x34, 0x12]);
    });
  });

  describe("complete program: sum 1..10", () => {
    it("computes sum = 55", () => {
      const sim = new RiscVSimulator();
      sim.run(assemble([
        encodeAddi(1, 0, 0), encodeAddi(2, 0, 1), encodeAddi(3, 0, 11),
        encodeAdd(1, 1, 2),
        encodeAddi(2, 2, 1),
        encodeBne(2, 3, -8),
        encodeEcall(),
      ]));
      expect(sim.cpu.registers.read(1)).toBe(55);
    });
  });
});
