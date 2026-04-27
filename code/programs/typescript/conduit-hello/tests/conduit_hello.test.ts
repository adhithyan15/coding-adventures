/**
 * conduit_hello.test.ts — integration tests for the conduit-hello demo.
 *
 * Each test starts a dedicated Conduit server (port 0 → OS-assigned) and
 * tears it down after the suite completes.  The application under test
 * mirrors hello.ts exactly so CI can verify the demo works end-to-end.
 *
 * HTTP client: Node.js 18+ built-in `fetch`.
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import {
  Application,
  Server,
  html,
  json,
  halt,
  redirect,
} from "../../../../packages/typescript/conduit/src/index.js";

// ── Helpers ───────────────────────────────────────────────────────────────────

async function get(
  port: number,
  path: string,
  opts: RequestInit = {},
): Promise<Response> {
  return fetch(`http://127.0.0.1:${port}${path}`, opts);
}

async function post(
  port: number,
  path: string,
  body: string,
  headers: Record<string, string> = {},
): Promise<Response> {
  return fetch(`http://127.0.0.1:${port}${path}`, {
    method: "POST",
    body,
    headers,
  });
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

// ── Build the hello application (mirrors hello.ts) ────────────────────────────

function buildHelloApp(): Application {
  const app = new Application();

  app.set("appName", "Conduit Hello");
  app.set("version", "0.1.0");

  app.before((req) => {
    if (req.path === "/down") halt(503, "Under maintenance");
  });

  app.after((req) => {
    // No-op in test — just confirm the filter doesn't throw.
    void req.path;
  });

  app.get("/", () =>
    html("<h1>Hello from Conduit!</h1><p>Try /hello/Adhithya</p>"),
  );

  app.get("/hello/:name", (req) =>
    json({
      message: `Hello ${req.params["name"] ?? "world"}!`,
      app: app.getSetting("appName"),
    }),
  );

  app.post("/echo", (req) => {
    const body = req.json<Record<string, unknown>>();
    return json(body);
  });

  app.get("/redirect", () => redirect("/", 301));
  app.get("/halt", () => halt(403, "Forbidden — this route always halts"));
  app.get("/down", () => html("unreachable"));

  app.get("/error", () => {
    throw new Error("Intentional error for demo purposes");
  });

  app.notFound((req) =>
    html(`<h1>404 Not Found</h1><p>No route: ${req.path}</p>`, 404),
  );

  app.onError((_req, msg) =>
    json({ error: "Internal Server Error", detail: msg }, 500),
  );

  return app;
}

// ── Test suite ────────────────────────────────────────────────────────────────

describe("conduit-hello integration tests", () => {
  let server: Server;
  let port: number;

  beforeAll(async () => {
    const app = buildHelloApp();
    server = new Server(app, { host: "127.0.0.1", port: 0 });
    server.serveBackground();
    await sleep(120);
    port = server.localPort;
  });

  afterAll(() => server.stop());

  // ── Root route ──────────────────────────────────────────────────────────────

  it("GET / returns 200 HTML with greeting", async () => {
    const res = await get(port, "/");
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toContain("text/html");
    const body = await res.text();
    expect(body).toContain("Hello from Conduit!");
  });

  // ── Named capture ───────────────────────────────────────────────────────────

  it("GET /hello/Adhithya returns JSON with name", async () => {
    const res = await get(port, "/hello/Adhithya");
    expect(res.status).toBe(200);
    const data = await res.json() as { message: string; app: unknown };
    expect(data.message).toBe("Hello Adhithya!");
    expect(data.app).toBe("Conduit Hello");
  });

  it("GET /hello/:name with different names", async () => {
    const names = ["Alice", "Bob", "Charlie"];
    for (const name of names) {
      const res = await get(port, `/hello/${name}`);
      const data = await res.json() as { message: string };
      expect(data.message).toBe(`Hello ${name}!`);
    }
  });

  // ── POST /echo ──────────────────────────────────────────────────────────────

  it("POST /echo echoes the JSON body", async () => {
    const payload = JSON.stringify({ ping: "pong", value: 42 });
    const res = await post(port, "/echo", payload, { "Content-Type": "application/json" });
    expect(res.status).toBe(200);
    const data = await res.json() as { ping: string; value: number };
    expect(data.ping).toBe("pong");
    expect(data.value).toBe(42);
  });

  it("POST /echo handles nested JSON", async () => {
    const payload = JSON.stringify({ user: { name: "Alice", roles: ["admin", "user"] } });
    const res = await post(port, "/echo", payload, { "Content-Type": "application/json" });
    const data = await res.json() as { user: { name: string } };
    expect(data.user.name).toBe("Alice");
  });

  // ── Redirect ────────────────────────────────────────────────────────────────

  it("GET /redirect returns 301 with Location: /", async () => {
    const res = await get(port, "/redirect", { redirect: "manual" });
    expect(res.status).toBe(301);
    expect(res.headers.get("location")).toBe("/");
  });

  // ── Halt ────────────────────────────────────────────────────────────────────

  it("GET /halt returns 403 Forbidden", async () => {
    const res = await get(port, "/halt");
    expect(res.status).toBe(403);
    const body = await res.text();
    expect(body).toContain("Forbidden");
  });

  // ── Before filter (maintenance mode) ────────────────────────────────────────

  it("GET /down returns 503 via before filter", async () => {
    const res = await get(port, "/down");
    expect(res.status).toBe(503);
    const body = await res.text();
    expect(body).toBe("Under maintenance");
  });

  // ── Error handler ────────────────────────────────────────────────────────────

  it("GET /error returns 500 via custom error handler", async () => {
    const res = await get(port, "/error");
    expect(res.status).toBe(500);
    const data = await res.json() as { error: string; detail: string };
    expect(data.error).toBe("Internal Server Error");
    expect(data.detail).toContain("Intentional error");
  });

  // ── Not-found handler ────────────────────────────────────────────────────────

  it("GET /missing returns 404 via custom not-found handler", async () => {
    const res = await get(port, "/missing");
    expect(res.status).toBe(404);
    const body = await res.text();
    expect(body).toContain("Not Found");
    expect(body).toContain("/missing");
  });

  it("GET /anything/else also returns 404", async () => {
    const res = await get(port, "/this/does/not/exist");
    expect(res.status).toBe(404);
  });

  // ── Multiple concurrent requests ─────────────────────────────────────────────

  it("handles multiple concurrent requests correctly", async () => {
    const requests = Array.from({ length: 10 }, (_, i) =>
      get(port, `/hello/User${i}`).then((r) => r.json() as Promise<{ message: string }>),
    );
    const results = await Promise.all(requests);
    for (let i = 0; i < 10; i++) {
      expect(results[i]!.message).toBe(`Hello User${i}!`);
    }
  });

  // ── Settings ─────────────────────────────────────────────────────────────────

  it("app settings are passed through to route handlers", async () => {
    const res = await get(port, "/hello/test");
    const data = await res.json() as { app: string };
    expect(data.app).toBe("Conduit Hello");
  });

  // ── Server metadata ──────────────────────────────────────────────────────────

  it("server.localPort is a valid port number", () => {
    expect(server.localPort).toBeGreaterThan(0);
    expect(server.localPort).toBeLessThanOrEqual(65535);
  });

  it("server.running is true while serving", () => {
    expect(server.running).toBe(true);
  });
});
