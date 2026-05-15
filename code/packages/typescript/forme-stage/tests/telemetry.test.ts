/**
 * forme-stage — telemetry tests
 */

import { describe, it, expect, vi } from "vitest";
import {
  callbackTelemetryEmitter,
  noOpTelemetryEmitter,
} from "../src/index.js";

describe("noOpTelemetryEmitter", () => {
  it("drops every emit", () => {
    const e = noOpTelemetryEmitter();
    expect(() => e.emit("x", { a: 1 })).not.toThrow();
  });
});

describe("callbackTelemetryEmitter", () => {
  it("forwards every event to the sink", () => {
    const sink = vi.fn();
    const e = callbackTelemetryEmitter(sink);
    e.emit("a.b", { count: 1 });
    e.emit("c.d", {});
    expect(sink).toHaveBeenCalledTimes(2);
    expect(sink).toHaveBeenNthCalledWith(1, "a.b", { count: 1 });
    expect(sink).toHaveBeenNthCalledWith(2, "c.d", {});
  });
});
