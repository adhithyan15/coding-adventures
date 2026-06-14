/**
 * forme-stage — logger tests
 */

import { describe, it, expect, vi } from "vitest";
import {
  LOG_LEVELS,
  consoleLogger,
  silentLogger,
} from "../src/index.js";

describe("LOG_LEVELS", () => {
  it("ladder is trace < debug < info < warn < error", () => {
    expect(LOG_LEVELS).toEqual(["trace", "debug", "info", "warn", "error"]);
  });

  it("is frozen", () => {
    expect(() => {
      // @ts-expect-error — readonly tuple
      LOG_LEVELS.push("fatal");
    }).toThrow(TypeError);
  });
});

describe("consoleLogger emission", () => {
  it("emits one JSON line per call", () => {
    const lines: string[] = [];
    const log = consoleLogger({ write: (l) => lines.push(l), now: () => 1000 });
    log.info("hello");
    log.warn("watch out", { code: 42 });
    expect(lines.length).toBe(2);
    const a = JSON.parse(lines[0]!);
    const b = JSON.parse(lines[1]!);
    expect(a).toEqual({ level: "info", message: "hello", ts: 1000 });
    expect(b).toEqual({ level: "warn", message: "watch out", ts: 1000, code: 42 });
  });

  it("respects the level threshold", () => {
    const lines: string[] = [];
    const log = consoleLogger({ write: (l) => lines.push(l), level: "warn", now: () => 0 });
    log.trace("x"); log.debug("x"); log.info("x");
    log.warn("kept");
    log.error("kept");
    expect(lines.length).toBe(2);
    expect(JSON.parse(lines[0]!).message).toBe("kept");
  });

  it("default threshold is info — trace and debug are dropped", () => {
    const lines: string[] = [];
    const log = consoleLogger({ write: (l) => lines.push(l), now: () => 0 });
    log.trace("x"); log.debug("x");
    log.info("kept");
    expect(lines.length).toBe(1);
  });

  it("default write goes to console.error", () => {
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    const log = consoleLogger({ now: () => 0 });
    log.info("hi");
    expect(spy).toHaveBeenCalledTimes(1);
    spy.mockRestore();
  });

  it("default now uses Date.now", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-05-15T00:00:00Z"));
    const lines: string[] = [];
    const log = consoleLogger({ write: (l) => lines.push(l) });
    log.info("hi");
    expect(JSON.parse(lines[0]!).ts).toBe(Date.UTC(2026, 4, 15));
    vi.useRealTimers();
  });

  it("each named level method emits with its own level tag", () => {
    const lines: string[] = [];
    const log = consoleLogger({ write: (l) => lines.push(l), level: "trace", now: () => 0 });
    log.trace("a"); log.debug("b"); log.info("c"); log.warn("d"); log.error("e");
    const levels = lines.map(l => JSON.parse(l).level);
    expect(levels).toEqual(["trace", "debug", "info", "warn", "error"]);
  });
});

describe("consoleLogger child scoping", () => {
  it("child fields are mixed into every emit", () => {
    const lines: string[] = [];
    const log = consoleLogger({ write: (l) => lines.push(l), now: () => 0 });
    const scoped = log.child({ stage: "@forme/parse-markdown", instance: "p1" });
    scoped.info("started");
    expect(JSON.parse(lines[0]!)).toEqual({
      level: "info", message: "started", ts: 0,
      stage: "@forme/parse-markdown", instance: "p1",
    });
  });

  it("per-call fields override child fields with the same key", () => {
    const lines: string[] = [];
    const log = consoleLogger({ write: (l) => lines.push(l), now: () => 0 });
    const scoped = log.child({ phase: "init" });
    scoped.info("hi", { phase: "run" });
    expect(JSON.parse(lines[0]!).phase).toBe("run");
  });

  it("child of child accumulates fields", () => {
    const lines: string[] = [];
    const log = consoleLogger({ write: (l) => lines.push(l), now: () => 0 });
    log.child({ a: 1 }).child({ b: 2 }).info("hi");
    const parsed = JSON.parse(lines[0]!);
    expect(parsed.a).toBe(1);
    expect(parsed.b).toBe(2);
  });
});

describe("silentLogger", () => {
  it("drops every level", () => {
    const log = silentLogger();
    expect(() => {
      log.trace("x"); log.debug("x"); log.info("x"); log.warn("x"); log.error("x");
    }).not.toThrow();
  });

  it("child returns the same silent logger", () => {
    const log = silentLogger();
    const child = log.child({ a: 1 });
    expect(child).toBe(log);
  });
});
