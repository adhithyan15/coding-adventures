import { describe, expect, it } from "vitest";
import {
  GE225Simulator,
  assembleFixed,
  assembleShift,
  decodeInstruction,
  encodeInstruction,
  packWords,
  unpackWords,
} from "../src/index.js";

function ins(opcode: number, address = 0, modifier = 0): number {
  return encodeInstruction(opcode, modifier, address);
}

describe("GE225Simulator", () => {
  it("encodes and decodes instructions", () => {
    const word = encodeInstruction(0o01, 0o2, 0x1234 & 0x1fff);
    expect(decodeInstruction(word)).toEqual([0o01, 0o2, 0x1234 & 0x1fff]);
    expect(unpackWords(packWords([word, assembleFixed("NOP")]))).toEqual([word, assembleFixed("NOP")]);
  });

  it("runs LDA/ADD/STA program", () => {
    const sim = new GE225Simulator();
    sim.loadWords([ins(0o00, 10), ins(0o01, 11), ins(0o03, 12), assembleFixed("NOP"), 0, 0, 0, 0, 0, 0, 1, 2, 0]);
    sim.run(4);
    const state = sim.getState();
    expect(state.a).toBe(3);
    expect(state.memory[12]).toBe(3);
  });

  it("stores P on SPB and loads through target", () => {
    const sim = new GE225Simulator();
    sim.loadWords([ins(0o07, 4, 2), assembleFixed("NOP"), assembleFixed("NOP"), assembleFixed("NOP"), ins(0o00, 10), assembleFixed("NOP"), 0, 0, 0, 0, 0x12345]);
    sim.run(3);
    const state = sim.getState();
    expect(state.xWords[2]).toBe(0);
    expect(state.a).toBe(0x12345);
  });

  it("honors odd-address double load/store", () => {
    const sim = new GE225Simulator();
    sim.writeWord(11, 0x13579);
    sim.loadWords([ins(0o10, 11), ins(0o13, 13), assembleFixed("NOP")]);
    sim.run(3);
    const state = sim.getState();
    expect(state.a).toBe(0x13579);
    expect(state.q).toBe(0x13579);
    expect(state.memory[13]).toBe(0x13579);
  });

  it("moves blocks with MOY", () => {
    const sim = new GE225Simulator();
    sim.writeWord(20, 0x11111);
    sim.writeWord(21, 0x22222);
    sim.writeWord(30, 40);
    sim.writeWord(31, ((1 << 20) - 2));
    sim.loadWords([ins(0o00, 30), assembleFixed("LQA"), ins(0o00, 31), assembleFixed("XAQ"), ins(0o24, 20), assembleFixed("NOP")]);
    sim.run(6);
    const state = sim.getState();
    expect(state.a).toBe(0);
    expect(state.memory[40]).toBe(0x11111);
    expect(state.memory[41]).toBe(0x22222);
  });

  it("supports console typewriter path", () => {
    const sim = new GE225Simulator();
    sim.setControlSwitches(0o1633);
    sim.loadWords([assembleFixed("RCS"), assembleFixed("TON"), assembleShift("SAN", 6), assembleFixed("TYP"), assembleFixed("NOP")]);
    sim.run(5);
    expect(sim.getTypewriterOutput()).toBe("-");
    expect(sim.getState().typewriterPower).toBe(true);
  });

  it("loads queued records with RCD", () => {
    const sim = new GE225Simulator();
    sim.queueCardReaderRecord([0x11111, 0x22222]);
    sim.loadWords([ins(0o25, 10), assembleFixed("NOP")]);
    sim.run(2);
    expect(sim.getState().memory[10]).toBe(0x11111);
    expect(sim.getState().memory[11]).toBe(0x22222);
  });
});
