/**
 * request.test.ts — unit tests for the Request class.
 *
 * Request wraps a string-to-string env map produced by the Rust cdylib.
 * All tests are pure TypeScript — no native code needed.
 */

import { describe, it, expect } from "vitest";
import { Request } from "../src/request.js";

// ── Helpers ───────────────────────────────────────────────────────────────────

/** Minimal env map for a simple GET request. */
function getEnv(overrides: Record<string, string> = {}): Record<string, string> {
  return {
    REQUEST_METHOD: "GET",
    PATH_INFO: "/",
    QUERY_STRING: "",
    ...overrides,
  };
}

// ── Core fields ───────────────────────────────────────────────────────────────

describe("Request — core fields", () => {
  it("reads method, path, queryString from env", () => {
    const req = new Request(getEnv({
      REQUEST_METHOD: "POST",
      PATH_INFO: "/users",
      QUERY_STRING: "debug=1",
    }));
    expect(req.method).toBe("POST");
    expect(req.path).toBe("/users");
    expect(req.queryString).toBe("debug=1");
  });

  it("defaults method to GET, path to /, queryString to empty", () => {
    const req = new Request({});
    expect(req.method).toBe("GET");
    expect(req.path).toBe("/");
    expect(req.queryString).toBe("");
  });

  it("exposes the raw env map", () => {
    const env = getEnv({ REQUEST_METHOD: "DELETE" });
    const req = new Request(env);
    expect(req.env).toBe(env);
  });
});

// ── Route params ──────────────────────────────────────────────────────────────

describe("Request — params (route captures)", () => {
  it("parses JSON route params", () => {
    const req = new Request(getEnv({
      "conduit.route_params": JSON.stringify({ name: "Alice", id: "42" }),
    }));
    expect(req.params["name"]).toBe("Alice");
    expect(req.params["id"]).toBe("42");
  });

  it("returns empty object when no route params", () => {
    const req = new Request(getEnv());
    expect(req.params).toEqual({});
  });

  it("returns empty object when route_params is malformed JSON", () => {
    const req = new Request(getEnv({ "conduit.route_params": "not-json" }));
    expect(req.params).toEqual({});
  });
});

// ── Query params ──────────────────────────────────────────────────────────────

describe("Request — queryParams", () => {
  it("parses query params from conduit.query_params JSON", () => {
    const req = new Request(getEnv({
      QUERY_STRING: "q=hello&n=5",
      "conduit.query_params": JSON.stringify({ q: "hello", n: "5" }),
    }));
    expect(req.queryParams["q"]).toBe("hello");
    expect(req.queryParams["n"]).toBe("5");
  });

  it("falls back to parsing QUERY_STRING if conduit.query_params absent", () => {
    const req = new Request(getEnv({ QUERY_STRING: "q=hello&n=5" }));
    expect(req.queryParams["q"]).toBe("hello");
    expect(req.queryParams["n"]).toBe("5");
  });

  it("returns empty object for empty query string", () => {
    const req = new Request(getEnv({ QUERY_STRING: "" }));
    expect(req.queryParams).toEqual({});
  });

  it("URL-decodes query string values", () => {
    const req = new Request(getEnv({ QUERY_STRING: "msg=hello+world" }));
    expect(req.queryParams["msg"]).toBe("hello world");
  });

  it("handles %XX percent-encoded values", () => {
    const req = new Request(getEnv({ QUERY_STRING: "city=New%20York" }));
    expect(req.queryParams["city"]).toBe("New York");
  });

  it("handles key with no value", () => {
    const req = new Request(getEnv({ QUERY_STRING: "flag" }));
    expect(req.queryParams["flag"]).toBe("");
  });
});

// ── Headers ───────────────────────────────────────────────────────────────────

describe("Request — headers", () => {
  it("parses headers from conduit.headers JSON", () => {
    const req = new Request(getEnv({
      "conduit.headers": JSON.stringify({
        "content-type": "application/json",
        "x-request-id": "abc123",
      }),
    }));
    expect(req.headers["content-type"]).toBe("application/json");
    expect(req.headers["x-request-id"]).toBe("abc123");
  });

  it("returns empty object when no headers", () => {
    const req = new Request(getEnv());
    expect(req.headers).toEqual({});
  });
});

// ── Body ──────────────────────────────────────────────────────────────────────

describe("Request — body", () => {
  it("reads raw body text", () => {
    const req = new Request(getEnv({ "conduit.body": "hello body" }));
    expect(req.bodyText).toBe("hello body");
  });

  it("defaults bodyText to empty string", () => {
    const req = new Request(getEnv());
    expect(req.bodyText).toBe("");
  });

  it("reads content-type and content-length", () => {
    const req = new Request(getEnv({
      "conduit.content_type": "application/json",
      "conduit.content_length": "13",
    }));
    expect(req.contentType).toBe("application/json");
    expect(req.contentLength).toBe(13);
  });

  it("defaults contentLength to 0 when absent", () => {
    const req = new Request(getEnv());
    expect(req.contentLength).toBe(0);
  });

  it("defaults contentLength to 0 for non-numeric value", () => {
    const req = new Request(getEnv({ "conduit.content_length": "abc" }));
    expect(req.contentLength).toBe(0);
  });
});

// ── json() ────────────────────────────────────────────────────────────────────

describe("Request#json()", () => {
  it("parses valid JSON body", () => {
    const req = new Request(getEnv({ "conduit.body": '{"name":"Alice","age":30}' }));
    const body = req.json<{ name: string; age: number }>();
    expect(body.name).toBe("Alice");
    expect(body.age).toBe(30);
  });

  it("throws SyntaxError on invalid JSON", () => {
    const req = new Request(getEnv({ "conduit.body": "not json" }));
    expect(() => req.json()).toThrow(SyntaxError);
  });

  it("caches the parsed result (same reference)", () => {
    const req = new Request(getEnv({ "conduit.body": '{"x":1}' }));
    const a = req.json();
    const b = req.json();
    expect(a).toBe(b); // same reference → cached
  });

  it("parses arrays correctly", () => {
    const req = new Request(getEnv({ "conduit.body": '[1,2,3]' }));
    expect(req.json()).toEqual([1, 2, 3]);
  });
});

// ── form() ────────────────────────────────────────────────────────────────────

describe("Request#form()", () => {
  it("parses URL-encoded form body", () => {
    const req = new Request(getEnv({
      "conduit.body": "name=Bob&role=admin",
      "conduit.content_type": "application/x-www-form-urlencoded",
    }));
    const form = req.form();
    expect(form["name"]).toBe("Bob");
    expect(form["role"]).toBe("admin");
  });

  it("URL-decodes form values", () => {
    const req = new Request(getEnv({ "conduit.body": "msg=hello+world" }));
    expect(req.form()["msg"]).toBe("hello world");
  });

  it("returns empty object for empty body", () => {
    const req = new Request(getEnv());
    expect(req.form()).toEqual({});
  });

  it("caches the parsed result", () => {
    const req = new Request(getEnv({ "conduit.body": "x=1" }));
    expect(req.form()).toBe(req.form());
  });
});
