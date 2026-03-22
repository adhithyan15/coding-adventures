/**
 * Tests for SwitchMatrix.
 */

import { describe, it, expect } from "vitest";
import { SwitchMatrix } from "../src/index.js";

describe("SwitchMatrix", () => {
  const defaultPorts = () => new Set(["north", "south", "east", "west", "clbOut"]);

  it("creates with valid ports", () => {
    const sm = new SwitchMatrix(defaultPorts());
    expect(sm.ports.size).toBe(5);
    expect(sm.connectionCount).toBe(0);
  });

  it("connect and route", () => {
    const sm = new SwitchMatrix(defaultPorts());
    sm.connect("clbOut", "east");
    sm.connect("north", "south");

    const result = sm.route({ clbOut: 1, north: 0 });
    expect(result).toEqual({ east: 1, south: 0 });
  });

  it("unconnected destinations don't appear in output", () => {
    const sm = new SwitchMatrix(defaultPorts());
    sm.connect("clbOut", "east");

    const result = sm.route({ clbOut: 1, north: 0 });
    expect(result).toEqual({ east: 1 });
    expect("south" in result).toBe(false);
  });

  it("missing source in inputs is skipped", () => {
    const sm = new SwitchMatrix(defaultPorts());
    sm.connect("clbOut", "east");

    const result = sm.route({ north: 0 }); // clbOut not provided
    expect(result).toEqual({});
  });

  it("fan-out: one source to multiple destinations", () => {
    const sm = new SwitchMatrix(defaultPorts());
    sm.connect("clbOut", "east");
    sm.connect("clbOut", "south");

    const result = sm.route({ clbOut: 1 });
    expect(result).toEqual({ east: 1, south: 1 });
  });

  it("disconnect removes a route", () => {
    const sm = new SwitchMatrix(defaultPorts());
    sm.connect("clbOut", "east");
    expect(sm.connectionCount).toBe(1);

    sm.disconnect("east");
    expect(sm.connectionCount).toBe(0);
    expect(sm.route({ clbOut: 1 })).toEqual({});
  });

  it("clear removes all connections", () => {
    const sm = new SwitchMatrix(defaultPorts());
    sm.connect("clbOut", "east");
    sm.connect("north", "south");
    expect(sm.connectionCount).toBe(2);

    sm.clear();
    expect(sm.connectionCount).toBe(0);
  });

  it("connections property returns copy", () => {
    const sm = new SwitchMatrix(defaultPorts());
    sm.connect("clbOut", "east");
    expect(sm.connections).toEqual({ east: "clbOut" });
  });

  it("rejects empty ports", () => {
    expect(() => new SwitchMatrix(new Set())).toThrow(RangeError);
  });

  it("rejects empty string port name", () => {
    expect(() => new SwitchMatrix(new Set(["a", ""]))).toThrow(TypeError);
  });

  it("rejects unknown source port", () => {
    const sm = new SwitchMatrix(defaultPorts());
    expect(() => sm.connect("unknown", "east")).toThrow(RangeError);
  });

  it("rejects unknown destination port", () => {
    const sm = new SwitchMatrix(defaultPorts());
    expect(() => sm.connect("east", "unknown")).toThrow(RangeError);
  });

  it("rejects self-connection", () => {
    const sm = new SwitchMatrix(defaultPorts());
    expect(() => sm.connect("east", "east")).toThrow(RangeError);
  });

  it("rejects duplicate destination", () => {
    const sm = new SwitchMatrix(defaultPorts());
    sm.connect("clbOut", "east");
    expect(() => sm.connect("north", "east")).toThrow(RangeError);
  });

  it("disconnect rejects unknown port", () => {
    const sm = new SwitchMatrix(defaultPorts());
    expect(() => sm.disconnect("unknown")).toThrow(RangeError);
  });

  it("disconnect rejects unconnected port", () => {
    const sm = new SwitchMatrix(defaultPorts());
    expect(() => sm.disconnect("east")).toThrow(RangeError);
  });
});
