/**
 * Tests for combinational circuits — MUX, DEMUX, decoder, encoder, tri-state.
 */

import { describe, it, expect } from "vitest";
import {
  mux2,
  mux4,
  muxN,
  demuxN,
  decoder,
  encoder,
  priorityEncoder,
  triState,
  type Bit,
} from "../src/index.js";

// === mux2 — 2-to-1 Multiplexer ===

describe("mux2", () => {
  it("sel=0 selects d0", () => {
    expect(mux2(0, 1, 0)).toBe(0);
    expect(mux2(1, 0, 0)).toBe(1);
  });

  it("sel=1 selects d1", () => {
    expect(mux2(0, 1, 1)).toBe(1);
    expect(mux2(1, 0, 1)).toBe(0);
  });

  it("both same value", () => {
    expect(mux2(1, 1, 0)).toBe(1);
    expect(mux2(0, 0, 1)).toBe(0);
  });

  it("invalid sel throws", () => {
    expect(() => mux2(0, 1, 2 as Bit)).toThrow(RangeError);
  });
});

// === mux4 — 4-to-1 Multiplexer ===

describe("mux4", () => {
  it("sel=[0,0] selects d0", () => {
    expect(mux4(1, 0, 0, 0, [0, 0])).toBe(1);
  });

  it("sel=[1,0] selects d1", () => {
    expect(mux4(0, 1, 0, 0, [1, 0])).toBe(1);
  });

  it("sel=[0,1] selects d2", () => {
    expect(mux4(0, 0, 1, 0, [0, 1])).toBe(1);
  });

  it("sel=[1,1] selects d3", () => {
    expect(mux4(0, 0, 0, 1, [1, 1])).toBe(1);
  });

  it("all zero inputs returns 0 for any sel", () => {
    expect(mux4(0, 0, 0, 0, [0, 0])).toBe(0);
    expect(mux4(0, 0, 0, 0, [1, 1])).toBe(0);
  });

  it("invalid sel length throws", () => {
    expect(() => mux4(0, 0, 0, 0, [0] as any)).toThrow(RangeError);
    expect(() => mux4(0, 0, 0, 0, [0, 0, 0] as any)).toThrow(RangeError);
  });
});

// === muxN — N-to-1 Multiplexer ===

describe("muxN", () => {
  it("works as 2:1 MUX", () => {
    expect(muxN([1, 0], [0])).toBe(1);
    expect(muxN([1, 0], [1])).toBe(0);
  });

  it("works as 4:1 MUX", () => {
    expect(muxN([1, 0, 0, 0], [0, 0])).toBe(1);
    expect(muxN([0, 1, 0, 0], [1, 0])).toBe(1);
    expect(muxN([0, 0, 1, 0], [0, 1])).toBe(1);
    expect(muxN([0, 0, 0, 1], [1, 1])).toBe(1);
  });

  it("works as 8:1 MUX", () => {
    // Select input 5 (binary 101 LSB-first = [1, 0, 1])
    const data: Bit[] = [0, 0, 0, 0, 0, 1, 0, 0];
    expect(muxN(data, [1, 0, 1])).toBe(1);
  });

  it("works as 16:1 MUX", () => {
    // Select input 10 (binary 1010 LSB-first = [0, 1, 0, 1])
    const data: Bit[] = Array(16).fill(0) as Bit[];
    data[10] = 1;
    expect(muxN(data, [0, 1, 0, 1])).toBe(1);
  });

  it("rejects non-power-of-2 inputs", () => {
    expect(() => muxN([0, 0, 0] as Bit[], [0, 0])).toThrow(RangeError);
  });

  it("rejects wrong sel length", () => {
    expect(() => muxN([0, 0, 0, 0], [0])).toThrow(RangeError);
  });

  it("rejects too few inputs", () => {
    expect(() => muxN([0], [])).toThrow(RangeError);
  });
});

// === demuxN — 1-to-N Demultiplexer ===

describe("demuxN", () => {
  it("routes data=1 to selected output (4 outputs)", () => {
    expect(demuxN(1, [0, 0], 4)).toEqual([1, 0, 0, 0]);
    expect(demuxN(1, [1, 0], 4)).toEqual([0, 1, 0, 0]);
    expect(demuxN(1, [0, 1], 4)).toEqual([0, 0, 1, 0]);
    expect(demuxN(1, [1, 1], 4)).toEqual([0, 0, 0, 1]);
  });

  it("data=0 produces all zeros", () => {
    expect(demuxN(0, [1, 0], 4)).toEqual([0, 0, 0, 0]);
    expect(demuxN(0, [0, 0], 4)).toEqual([0, 0, 0, 0]);
  });

  it("works with 2 outputs", () => {
    expect(demuxN(1, [0], 2)).toEqual([1, 0]);
    expect(demuxN(1, [1], 2)).toEqual([0, 1]);
  });

  it("works with 8 outputs", () => {
    const result = demuxN(1, [1, 0, 1], 8);
    expect(result[5]).toBe(1);
    expect(result.filter((b) => b === 1).length).toBe(1);
  });

  it("rejects non-power-of-2 nOutputs", () => {
    expect(() => demuxN(1, [0], 3)).toThrow(RangeError);
  });

  it("rejects wrong sel length", () => {
    expect(() => demuxN(1, [0, 0, 0], 4)).toThrow(RangeError);
  });
});

// === decoder — Binary to One-Hot ===

describe("decoder", () => {
  it("1-bit decoder (1-to-2)", () => {
    expect(decoder([0])).toEqual([1, 0]);
    expect(decoder([1])).toEqual([0, 1]);
  });

  it("2-bit decoder (2-to-4)", () => {
    expect(decoder([0, 0])).toEqual([1, 0, 0, 0]);
    expect(decoder([1, 0])).toEqual([0, 1, 0, 0]);
    expect(decoder([0, 1])).toEqual([0, 0, 1, 0]);
    expect(decoder([1, 1])).toEqual([0, 0, 0, 1]);
  });

  it("3-bit decoder (3-to-8)", () => {
    const result = decoder([1, 0, 1]); // index = 1 + 0 + 4 = 5
    expect(result.length).toBe(8);
    expect(result[5]).toBe(1);
    expect(result.filter((b) => b === 1).length).toBe(1);
  });

  it("always produces exactly one 1", () => {
    for (let i = 0; i < 4; i++) {
      const bits: Bit[] = [((i >> 0) & 1) as Bit, ((i >> 1) & 1) as Bit];
      const result = decoder(bits);
      expect(result.filter((b) => b === 1).length).toBe(1);
      expect(result[i]).toBe(1);
    }
  });

  it("rejects empty inputs", () => {
    expect(() => decoder([])).toThrow(RangeError);
  });
});

// === encoder — One-Hot to Binary ===

describe("encoder", () => {
  it("4-to-2 encoder", () => {
    expect(encoder([1, 0, 0, 0])).toEqual([0, 0]);
    expect(encoder([0, 1, 0, 0])).toEqual([1, 0]);
    expect(encoder([0, 0, 1, 0])).toEqual([0, 1]);
    expect(encoder([0, 0, 0, 1])).toEqual([1, 1]);
  });

  it("2-to-1 encoder", () => {
    expect(encoder([1, 0])).toEqual([0]);
    expect(encoder([0, 1])).toEqual([1]);
  });

  it("8-to-3 encoder", () => {
    const input: Bit[] = [0, 0, 0, 0, 0, 1, 0, 0]; // index 5
    expect(encoder(input)).toEqual([1, 0, 1]); // 5 = 101 in binary
  });

  it("rejects non-one-hot (multiple active)", () => {
    expect(() => encoder([1, 1, 0, 0])).toThrow(RangeError);
  });

  it("rejects non-one-hot (none active)", () => {
    expect(() => encoder([0, 0, 0, 0])).toThrow(RangeError);
  });

  it("rejects non-power-of-2 length", () => {
    expect(() => encoder([1, 0, 0])).toThrow(RangeError);
  });
});

// === priorityEncoder — Highest-Priority Active Input ===

describe("priorityEncoder", () => {
  it("single active input", () => {
    expect(priorityEncoder([1, 0, 0, 0])).toEqual([[0, 0], 1]);
    expect(priorityEncoder([0, 1, 0, 0])).toEqual([[1, 0], 1]);
    expect(priorityEncoder([0, 0, 1, 0])).toEqual([[0, 1], 1]);
    expect(priorityEncoder([0, 0, 0, 1])).toEqual([[1, 1], 1]);
  });

  it("multiple active — highest wins", () => {
    expect(priorityEncoder([1, 0, 1, 0])).toEqual([[0, 1], 1]); // I2 wins
    expect(priorityEncoder([1, 1, 0, 0])).toEqual([[1, 0], 1]); // I1 wins
    expect(priorityEncoder([1, 1, 1, 1])).toEqual([[1, 1], 1]); // I3 wins
  });

  it("no active input — valid=0", () => {
    expect(priorityEncoder([0, 0, 0, 0])).toEqual([[0, 0], 0]);
  });

  it("works with 8 inputs", () => {
    const inputs: Bit[] = [0, 0, 0, 1, 0, 1, 0, 0]; // I3 and I5 active
    const [encoded, valid] = priorityEncoder(inputs);
    expect(valid).toBe(1);
    expect(encoded).toEqual([1, 0, 1]); // 5 = 101
  });

  it("rejects non-power-of-2 length", () => {
    expect(() => priorityEncoder([1, 0, 0])).toThrow(RangeError);
  });
});

// === triState — Three-State Buffer ===

describe("triState", () => {
  it("enable=1 passes data through", () => {
    expect(triState(0, 1)).toBe(0);
    expect(triState(1, 1)).toBe(1);
  });

  it("enable=0 returns null (high-impedance)", () => {
    expect(triState(0, 0)).toBeNull();
    expect(triState(1, 0)).toBeNull();
  });

  it("invalid data throws", () => {
    expect(() => triState(2 as Bit, 1)).toThrow(RangeError);
  });

  it("invalid enable throws", () => {
    expect(() => triState(0, 2 as Bit)).toThrow(RangeError);
  });
});

// === Roundtrip: decoder -> encoder ===

describe("decoder/encoder roundtrip", () => {
  it("encoder(decoder(bits)) returns original bits", () => {
    const testCases: Bit[][] = [
      [0, 0],
      [1, 0],
      [0, 1],
      [1, 1],
    ];
    for (const bits of testCases) {
      const decoded = decoder(bits);
      const encoded = encoder(decoded);
      expect(encoded).toEqual(bits);
    }
  });
});
