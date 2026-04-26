/**
 * server.test.ts — end-to-end tests for the Conduit TypeScript package.
 *
 * These tests run against a real TCP server using the compiled Rust cdylib
 * (conduit_native_node.node).  Each test suite starts a server on a random
 * OS-assigned port (port 0) and tears it down after all tests complete.
 *
 * Threading model recap
 * ─────────────────────
 * - `serveBackground()` starts the Rust background thread but returns
 *   immediately (no blocking).
 * - `stop()` signals the background thread to stop and releases all TsFns.
 * - The Condvar + requestSlot pattern means JS handlers run synchronously
 *   on the V8 main thread while the Rust thread waits.
 *
 * HTTP client
 * ───────────
 * We use Node.js's built-in `fetch` (available since Node 18) to make
 * requests.  A small `get` / `post` helper wraps fetch for brevity.
 *
 * Port stability
 * ──────────────
 * Each describe block shares one server (created in beforeAll / torn down in
 * afterAll).  We use `server.localPort` to find the OS-assigned port.
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { Application, Server, halt, redirect, html, json, text } from "../src/index.js";

// ── HTTP helpers ──────────────────────────────────────────────────────────────

async function get(
  port: number,
  path: string,
  headers: Record<string, string> = {},
): Promise<{ status: number; headers: Headers; body: string }> {
  const res = await fetch(`http://127.0.0.1:${port}${path}`, { headers });
  const body = await res.text();
  return { status: res.status, headers: res.headers, body };
}

async function post(
  port: number,
  path: string,
  body: string,
  headers: Record<string, string> = {},
): Promise<{ status: number; headers: Headers; body: string }> {
  const res = await fetch(`http://127.0.0.1:${port}${path}`, {
    method: "POST",
    body,
    headers,
  });
  const respBody = await res.text();
  return { status: res.status, headers: res.headers, body: respBody };
}

async function del(
  port: number,
  path: string,
): Promise<{ status: number; body: string }> {
  const res = await fetch(`http://127.0.0.1:${port}${path}`, { method: "DELETE" });
  return { status: res.status, body: await res.text() };
}

// ── Helper: sleep ─────────────────────────────────────────────────────────────

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// ── Suite 1: basic routes ─────────────────────────────────────────────────────

describe("Conduit Server — basic routes", () => {
  let server: Server;
  let port: number;

  beforeAll(async () => {
    const app = new Application();

    app.get("/", () => html("<h1>Hello from Conduit!</h1>"));
    app.get("/ping", () => text("pong"));
    app.get("/health", () => json({ status: "ok" }));
    app.get("/hello/:name", (req) =>
      json({ message: `Hello ${req.params["name"] ?? ""}` }),
    );
    app.post("/echo", (req) => text(req.bodyText));
    app.delete("/items/:id", (req) =>
      json({ deleted: req.params["id"] }),
    );

    server = new Server(app, { host: "127.0.0.1", port: 0 });
    server.serveBackground();
    await sleep(100);
    port = server.localPort;
  });

  afterAll(() => {
    server.stop();
  });

  it("GET / returns 200 HTML", async () => {
    const { status, headers, body } = await get(port, "/");
    expect(status).toBe(200);
    expect(headers.get("content-type")).toContain("text/html");
    expect(body).toContain("Hello from Conduit!");
  });

  it("GET /ping returns plain text pong", async () => {
    const { status, headers, body } = await get(port, "/ping");
    expect(status).toBe(200);
    expect(headers.get("content-type")).toContain("text/plain");
    expect(body).toBe("pong");
  });

  it("GET /health returns JSON", async () => {
    const { status, body } = await get(port, "/health");
    expect(status).toBe(200);
    expect(JSON.parse(body)).toEqual({ status: "ok" });
  });

  it("GET /hello/:name captures route parameter", async () => {
    const { status, body } = await get(port, "/hello/Alice");
    expect(status).toBe(200);
    expect(JSON.parse(body)).toEqual({ message: "Hello Alice" });
  });

  it("POST /echo returns the request body", async () => {
    const { status, body } = await post(port, "/echo", "hello body");
    expect(status).toBe(200);
    expect(body).toBe("hello body");
  });

  it("DELETE /items/:id captures route parameter", async () => {
    const { status, body } = await del(port, "/items/42");
    expect(status).toBe(200);
    expect(JSON.parse(body)).toEqual({ deleted: "42" });
  });
});

// ── Suite 2: before filter ────────────────────────────────────────────────────

describe("Conduit Server — before filter", () => {
  let server: Server;
  let port: number;

  beforeAll(async () => {
    const app = new Application();

    // Maintenance mode: halt all requests to /down
    app.before((req) => {
      if (req.path === "/down") halt(503, "Under maintenance");
    });

    app.get("/", () => html("<h1>Up</h1>"));
    app.get("/down", () => html("Should never reach"));

    server = new Server(app, { host: "127.0.0.1", port: 0 });
    server.serveBackground();
    await sleep(100);
    port = server.localPort;
  });

  afterAll(() => server.stop());

  it("before filter short-circuits with halt(503)", async () => {
    const { status, body } = await get(port, "/down");
    expect(status).toBe(503);
    expect(body).toBe("Under maintenance");
  });

  it("before filter passes through for normal requests", async () => {
    const { status, body } = await get(port, "/");
    expect(status).toBe(200);
    expect(body).toContain("Up");
  });
});

// ── Suite 3: halt and redirect from route handlers ────────────────────────────

describe("Conduit Server — halt and redirect", () => {
  let server: Server;
  let port: number;

  beforeAll(async () => {
    const app = new Application();

    app.get("/halt", () => { halt(418, "I'm a teapot"); });
    app.get("/redirect", () => { redirect("/new-path"); });
    app.get("/redirect-301", () => { redirect("/permanent", 301); });
    app.get("/new-path", () => html("Redirected destination"));

    server = new Server(app, { host: "127.0.0.1", port: 0 });
    server.serveBackground();
    await sleep(100);
    port = server.localPort;
  });

  afterAll(() => server.stop());

  it("halt() returns the specified status and body", async () => {
    const { status, body } = await get(port, "/halt");
    expect(status).toBe(418);
    expect(body).toBe("I'm a teapot");
  });

  it("redirect() returns 302 with Location header", async () => {
    // Use manual fetch so we can inspect the 302 before following redirects.
    const res = await fetch(`http://127.0.0.1:${port}/redirect`, { redirect: "manual" });
    expect(res.status).toBe(302);
    expect(res.headers.get("location")).toBe("/new-path");
  });

  it("redirect(url, 301) returns 301 Moved Permanently", async () => {
    const res = await fetch(`http://127.0.0.1:${port}/redirect-301`, { redirect: "manual" });
    expect(res.status).toBe(301);
    expect(res.headers.get("location")).toBe("/permanent");
  });
});

// ── Suite 4: custom not-found handler ─────────────────────────────────────────

describe("Conduit Server — not-found handler", () => {
  let server: Server;
  let port: number;

  beforeAll(async () => {
    const app = new Application();

    app.get("/exists", () => html("Found"));
    app.notFound((req) => html(`Not Found: ${req.path}`, 404));

    server = new Server(app, { host: "127.0.0.1", port: 0 });
    server.serveBackground();
    await sleep(100);
    port = server.localPort;
  });

  afterAll(() => server.stop());

  it("custom not-found handler is called for unknown paths", async () => {
    const { status, body } = await get(port, "/missing");
    expect(status).toBe(404);
    expect(body).toContain("Not Found");
    expect(body).toContain("/missing");
  });

  it("known routes still work alongside not-found handler", async () => {
    const { status } = await get(port, "/exists");
    expect(status).toBe(200);
  });
});

// ── Suite 5: error handler ────────────────────────────────────────────────────

describe("Conduit Server — error handler", () => {
  let server: Server;
  let port: number;

  beforeAll(async () => {
    const app = new Application();

    app.get("/boom", (_req) => {
      throw new Error("Something went wrong!");
    });

    app.onError((_req, msg) =>
      json({ error: msg }, 500),
    );

    server = new Server(app, { host: "127.0.0.1", port: 0 });
    server.serveBackground();
    await sleep(100);
    port = server.localPort;
  });

  afterAll(() => server.stop());

  it("error handler is called when a route throws", async () => {
    const { status, body } = await get(port, "/boom");
    expect(status).toBe(500);
    const parsed = JSON.parse(body) as { error: string };
    expect(parsed.error).toContain("Something went wrong!");
  });
});

// ── Suite 6: query parameters ─────────────────────────────────────────────────

describe("Conduit Server — query parameters", () => {
  let server: Server;
  let port: number;

  beforeAll(async () => {
    const app = new Application();

    app.get("/search", (req) =>
      json({
        q: req.queryParams["q"] ?? "",
        page: req.queryParams["page"] ?? "1",
      }),
    );

    server = new Server(app, { host: "127.0.0.1", port: 0 });
    server.serveBackground();
    await sleep(100);
    port = server.localPort;
  });

  afterAll(() => server.stop());

  it("query parameters are accessible via req.queryParams", async () => {
    const { status, body } = await get(port, "/search?q=hello&page=2");
    expect(status).toBe(200);
    const data = JSON.parse(body) as { q: string; page: string };
    expect(data.q).toBe("hello");
    expect(data.page).toBe("2");
  });

  it("missing query params return empty string defaults", async () => {
    const { body } = await get(port, "/search");
    const data = JSON.parse(body) as { q: string; page: string };
    expect(data.q).toBe("");
    expect(data.page).toBe("1");
  });
});

// ── Suite 7: JSON request body ────────────────────────────────────────────────

describe("Conduit Server — JSON request body", () => {
  let server: Server;
  let port: number;

  beforeAll(async () => {
    const app = new Application();

    app.post("/users", (req) => {
      const body = req.json<{ name: string }>();
      return json({ created: body.name }, 201);
    });

    server = new Server(app, { host: "127.0.0.1", port: 0 });
    server.serveBackground();
    await sleep(100);
    port = server.localPort;
  });

  afterAll(() => server.stop());

  it("JSON body is parsed and accessible via req.json()", async () => {
    const { status, body } = await post(
      port,
      "/users",
      JSON.stringify({ name: "Alice" }),
      { "Content-Type": "application/json" },
    );
    expect(status).toBe(201);
    expect(JSON.parse(body)).toEqual({ created: "Alice" });
  });
});

// ── Suite 8: server metadata ──────────────────────────────────────────────────

describe("Conduit Server — server metadata", () => {
  let server: Server;

  beforeAll(async () => {
    const app = new Application();
    app.get("/", () => html("ok"));
    server = new Server(app, { host: "127.0.0.1", port: 0 });
    server.serveBackground();
    await sleep(100);
  });

  afterAll(() => server.stop());

  it("localPort returns a non-zero port", () => {
    expect(server.localPort).toBeGreaterThan(0);
    expect(server.localPort).toBeLessThanOrEqual(65535);
  });

  it("running returns true while serving", () => {
    expect(server.running).toBe(true);
  });

  it("running returns false after stop()", async () => {
    server.stop();
    await sleep(100);
    expect(server.running).toBe(false);
  });
});

// ── Suite 9: request headers ──────────────────────────────────────────────────

describe("Conduit Server — request headers", () => {
  let server: Server;
  let port: number;

  beforeAll(async () => {
    const app = new Application();

    app.get("/whoami", (req) =>
      json({ ua: req.headers["user-agent"] ?? "" }),
    );

    server = new Server(app, { host: "127.0.0.1", port: 0 });
    server.serveBackground();
    await sleep(100);
    port = server.localPort;
  });

  afterAll(() => server.stop());

  it("request headers are accessible via req.headers", async () => {
    const { body } = await get(port, "/whoami", { "User-Agent": "TestClient/1.0" });
    const data = JSON.parse(body) as { ua: string };
    expect(data.ua).toBe("TestClient/1.0");
  });
});

// ── Suite 10: after filter ────────────────────────────────────────────────────

describe("Conduit Server — after filter", () => {
  let server: Server;
  let port: number;
  const visited: string[] = [];

  beforeAll(async () => {
    const app = new Application();

    app.after((req) => {
      visited.push(req.path);
    });

    app.get("/track", () => html("tracked"));

    server = new Server(app, { host: "127.0.0.1", port: 0 });
    server.serveBackground();
    await sleep(100);
    port = server.localPort;
  });

  afterAll(() => server.stop());

  it("after filter runs after a successful route handler", async () => {
    await get(port, "/track");
    await sleep(50);
    expect(visited).toContain("/track");
  });
});
