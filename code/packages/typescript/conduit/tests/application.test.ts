/**
 * application.test.ts — unit tests for the Application DSL class.
 *
 * Application is pure TypeScript — no native code involved.
 */

import { describe, it, expect, vi } from "vitest";
import { Application } from "../src/application.js";
import type { Handler, ErrorHandler } from "../src/application.js";

// ── Route registration ─────────────────────────────────────────────────────────

describe("Application — route registration", () => {
  it("registers GET routes", () => {
    const app = new Application();
    const h: Handler = () => undefined;
    app.get("/hello", h);
    expect(app.routes).toHaveLength(1);
    expect(app.routes[0]!.method).toBe("GET");
    expect(app.routes[0]!.pattern).toBe("/hello");
    expect(app.routes[0]!.handler).toBe(h);
  });

  it("registers POST routes", () => {
    const app = new Application();
    app.post("/echo", () => undefined);
    expect(app.routes[0]!.method).toBe("POST");
  });

  it("registers PUT routes", () => {
    const app = new Application();
    app.put("/items/1", () => undefined);
    expect(app.routes[0]!.method).toBe("PUT");
  });

  it("registers DELETE routes", () => {
    const app = new Application();
    app.delete("/items/1", () => undefined);
    expect(app.routes[0]!.method).toBe("DELETE");
  });

  it("registers PATCH routes", () => {
    const app = new Application();
    app.patch("/items/1", () => undefined);
    expect(app.routes[0]!.method).toBe("PATCH");
  });

  it("maintains insertion order across multiple routes", () => {
    const app = new Application();
    app.get("/a", () => undefined);
    app.post("/b", () => undefined);
    app.delete("/c", () => undefined);
    const methods = app.routes.map((r) => r.method);
    expect(methods).toEqual(["GET", "POST", "DELETE"]);
  });

  it("supports method chaining", () => {
    const app = new Application();
    const result = app.get("/a", () => undefined).post("/b", () => undefined);
    expect(result).toBe(app);
    expect(app.routes).toHaveLength(2);
  });
});

// ── Before / after filters ─────────────────────────────────────────────────────

describe("Application — before / after filters", () => {
  it("registers before filters in order", () => {
    const app = new Application();
    const f1: Handler = () => undefined;
    const f2: Handler = () => undefined;
    app.before(f1).before(f2);
    expect(app.beforeFilters).toEqual([f1, f2]);
  });

  it("registers after filters in order", () => {
    const app = new Application();
    const a1: Handler = () => undefined;
    const a2: Handler = () => undefined;
    app.after(a1).after(a2);
    expect(app.afterFilters).toEqual([a1, a2]);
  });

  it("before() returns this for chaining", () => {
    const app = new Application();
    expect(app.before(() => undefined)).toBe(app);
  });

  it("after() returns this for chaining", () => {
    const app = new Application();
    expect(app.after(() => undefined)).toBe(app);
  });

  it("before and after filters start empty", () => {
    const app = new Application();
    expect(app.beforeFilters).toEqual([]);
    expect(app.afterFilters).toEqual([]);
  });
});

// ── Special handlers ───────────────────────────────────────────────────────────

describe("Application — notFound / onError", () => {
  it("notFoundHandler starts as null", () => {
    expect(new Application().notFoundHandler).toBeNull();
  });

  it("notFound() sets the handler", () => {
    const app = new Application();
    const h: Handler = () => undefined;
    app.notFound(h);
    expect(app.notFoundHandler).toBe(h);
  });

  it("notFound() returns this", () => {
    const app = new Application();
    expect(app.notFound(() => undefined)).toBe(app);
  });

  it("errorHandler starts as null", () => {
    expect(new Application().errorHandler).toBeNull();
  });

  it("onError() sets the error handler", () => {
    const app = new Application();
    const h: ErrorHandler = () => undefined;
    app.onError(h);
    expect(app.errorHandler).toBe(h);
  });

  it("onError() returns this", () => {
    const app = new Application();
    expect(app.onError(() => undefined)).toBe(app);
  });
});

// ── Settings ───────────────────────────────────────────────────────────────────

describe("Application — settings", () => {
  it("set() stores a value accessible by getSetting()", () => {
    const app = new Application();
    app.set("appName", "Test App");
    expect(app.getSetting("appName")).toBe("Test App");
  });

  it("set() supports booleans and numbers", () => {
    const app = new Application();
    app.set("debug", true).set("port", 4000);
    expect(app.getSetting("debug")).toBe(true);
    expect(app.getSetting("port")).toBe(4000);
  });

  it("getSetting() returns undefined for unknown keys", () => {
    const app = new Application();
    expect(app.getSetting("missing")).toBeUndefined();
  });

  it("settings are accessible via the settings map directly", () => {
    const app = new Application();
    app.set("key", "val");
    expect(app.settings["key"]).toBe("val");
  });

  it("set() returns this for chaining", () => {
    const app = new Application();
    expect(app.set("k", "v")).toBe(app);
  });

  it("settings start empty", () => {
    expect(new Application().settings).toEqual({});
  });
});

// ── Full DSL chaining ──────────────────────────────────────────────────────────

describe("Application — full DSL chain", () => {
  it("can chain all methods together", () => {
    const app = new Application()
      .before(() => undefined)
      .get("/", () => undefined)
      .post("/users", () => undefined)
      .notFound(() => undefined)
      .onError(() => undefined)
      .set("appName", "Chain Test");

    expect(app.beforeFilters).toHaveLength(1);
    expect(app.routes).toHaveLength(2);
    expect(app.notFoundHandler).not.toBeNull();
    expect(app.errorHandler).not.toBeNull();
    expect(app.settings["appName"]).toBe("Chain Test");
  });
});
