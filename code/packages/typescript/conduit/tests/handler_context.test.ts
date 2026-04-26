/**
 * handler_context.test.ts — unit tests for response builder helpers.
 *
 * html(), json(), text(), respond() are pure functions — no native code.
 */

import { describe, it, expect } from "vitest";
import { html, json, text, respond } from "../src/handler_context.js";
import { halt, redirect, HaltError } from "../src/handler_context.js";

// ── html() ────────────────────────────────────────────────────────────────────

describe("html()", () => {
  it("returns a 200 response with text/html content-type", () => {
    const [status, headers, body] = html("<h1>Hello</h1>");
    expect(status).toBe(200);
    expect(headers["Content-Type"]).toContain("text/html");
    expect(body).toBe("<h1>Hello</h1>");
  });

  it("accepts a custom status code", () => {
    const [status, , body] = html("<h1>Not Found</h1>", 404);
    expect(status).toBe(404);
    expect(body).toBe("<h1>Not Found</h1>");
  });

  it("includes charset=utf-8 in content type", () => {
    const [, headers] = html("");
    expect(headers["Content-Type"]).toContain("utf-8");
  });
});

// ── json() ────────────────────────────────────────────────────────────────────

describe("json()", () => {
  it("serialises an object and returns application/json", () => {
    const [status, headers, body] = json({ ok: true });
    expect(status).toBe(200);
    expect(headers["Content-Type"]).toBe("application/json");
    expect(body).toBe('{"ok":true}');
  });

  it("accepts a custom status code", () => {
    const [status] = json({ id: 42 }, 201);
    expect(status).toBe(201);
  });

  it("serialises arrays", () => {
    const [, , body] = json([1, 2, 3]);
    expect(body).toBe("[1,2,3]");
  });

  it("serialises null", () => {
    const [, , body] = json(null);
    expect(body).toBe("null");
  });

  it("serialises nested objects", () => {
    const [, , body] = json({ user: { name: "Alice", age: 30 } });
    expect(JSON.parse(body)).toEqual({ user: { name: "Alice", age: 30 } });
  });
});

// ── text() ────────────────────────────────────────────────────────────────────

describe("text()", () => {
  it("returns a 200 response with text/plain", () => {
    const [status, headers, body] = text("pong");
    expect(status).toBe(200);
    expect(headers["Content-Type"]).toContain("text/plain");
    expect(body).toBe("pong");
  });

  it("accepts a custom status code", () => {
    const [status] = text("Gone", 410);
    expect(status).toBe(410);
  });
});

// ── respond() ─────────────────────────────────────────────────────────────────

describe("respond()", () => {
  it("returns the exact status, headers, and body provided", () => {
    const [s, h, b] = respond(204, "", {});
    expect(s).toBe(204);
    expect(h).toEqual({});
    expect(b).toBe("");
  });

  it("includes custom headers", () => {
    const [, headers] = respond(201, "", { Location: "/items/1" });
    expect(headers["Location"]).toBe("/items/1");
  });

  it("defaults headers to empty object", () => {
    const [, headers] = respond(200, "body");
    expect(headers).toEqual({});
  });
});

// ── halt() and redirect() re-exported ─────────────────────────────────────────

describe("handler_context re-exports halt/redirect/HaltError", () => {
  it("halt is the same function as from halt_error", () => {
    expect(() => halt(503)).toThrow(HaltError);
  });

  it("redirect throws HaltError with Location header", () => {
    let err: HaltError | null = null;
    try { redirect("/new"); } catch (e) { err = e as HaltError; }
    expect(err).toBeInstanceOf(HaltError);
    expect(err!.haltHeaderPairs).toEqual([["Location", "/new"]]);
  });
});
