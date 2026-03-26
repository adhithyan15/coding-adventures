/**
 * Tests for IOBlock.
 */

import { describe, it, expect } from "vitest";
import { IOBlock, IOMode } from "../src/index.js";

describe("IOBlock", () => {
  it("creates with name and default INPUT mode", () => {
    const io = new IOBlock("pin0");
    expect(io.name).toBe("pin0");
    expect(io.mode).toBe(IOMode.INPUT);
  });

  it("INPUT mode: drivePad -> readInternal", () => {
    const io = new IOBlock("pin0", IOMode.INPUT);
    io.drivePad(1);
    expect(io.readInternal()).toBe(1);
  });

  it("INPUT mode: readPad returns pad value", () => {
    const io = new IOBlock("pin0", IOMode.INPUT);
    io.drivePad(1);
    expect(io.readPad()).toBe(1);
  });

  it("OUTPUT mode: driveInternal -> readPad", () => {
    const io = new IOBlock("led0", IOMode.OUTPUT);
    io.driveInternal(1);
    expect(io.readPad()).toBe(1);
  });

  it("OUTPUT mode: readInternal returns internal value", () => {
    const io = new IOBlock("led0", IOMode.OUTPUT);
    io.driveInternal(1);
    expect(io.readInternal()).toBe(1);
  });

  it("TRISTATE mode: readPad returns null (high-Z)", () => {
    const io = new IOBlock("bus0", IOMode.TRISTATE);
    io.driveInternal(1);
    expect(io.readPad()).toBeNull();
  });

  it("TRISTATE mode: readInternal returns internal value", () => {
    const io = new IOBlock("bus0", IOMode.TRISTATE);
    io.driveInternal(1);
    expect(io.readInternal()).toBe(1);
  });

  it("configure changes mode", () => {
    const io = new IOBlock("pin0", IOMode.INPUT);
    io.configure(IOMode.OUTPUT);
    expect(io.mode).toBe(IOMode.OUTPUT);
  });

  it("switch from INPUT to OUTPUT changes readPad behavior", () => {
    const io = new IOBlock("pin0", IOMode.INPUT);
    io.drivePad(1);
    expect(io.readPad()).toBe(1);

    io.configure(IOMode.OUTPUT);
    io.driveInternal(0);
    expect(io.readPad()).toBe(0);
  });

  it("switch to TRISTATE disconnects pad", () => {
    const io = new IOBlock("pin0", IOMode.OUTPUT);
    io.driveInternal(1);
    expect(io.readPad()).toBe(1);

    io.configure(IOMode.TRISTATE);
    expect(io.readPad()).toBeNull();
  });

  it("rejects empty name", () => {
    expect(() => new IOBlock("")).toThrow(TypeError);
  });

  it("rejects non-string name", () => {
    expect(() => new IOBlock(42 as any)).toThrow(TypeError);
  });

  it("rejects invalid pad value", () => {
    const io = new IOBlock("pin0");
    expect(() => io.drivePad(2 as any)).toThrow(RangeError);
  });

  it("rejects invalid internal value", () => {
    const io = new IOBlock("pin0", IOMode.OUTPUT);
    expect(() => io.driveInternal(2 as any)).toThrow(RangeError);
  });

  it("configure rejects invalid mode", () => {
    const io = new IOBlock("pin0");
    expect(() => io.configure("invalid" as any)).toThrow(TypeError);
  });
});
