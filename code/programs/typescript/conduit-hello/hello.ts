/**
 * hello.ts — conduit-hello: full Express-style demo built on Conduit.
 *
 * Exercises every feature of the WEB05 TypeScript DSL:
 *
 *   GET  /                  → HTML greeting
 *   GET  /hello/:name       → JSON with captured name
 *   POST /echo              → echoes JSON body
 *   GET  /redirect          → 301 to /
 *   GET  /halt              → 403 via halt()
 *   GET  /down              → 503 via before filter (maintenance mode)
 *   GET  /error             → triggers custom error handler (500)
 *   GET  /missing           → custom not-found handler (404)
 *
 * Run:
 *   npx tsx hello.ts
 *   # or after compiling:
 *   node dist/hello.js
 *
 * Then:
 *   curl http://localhost:3000/
 *   curl http://localhost:3000/hello/Adhithya
 *   curl -X POST http://localhost:3000/echo -H 'Content-Type: application/json' -d '{"ping":"pong"}'
 *   curl -L http://localhost:3000/redirect
 *   curl http://localhost:3000/halt
 *   curl http://localhost:3000/down
 *   curl http://localhost:3000/error
 *   curl http://localhost:3000/missing
 */

// Adjust the path to match the local monorepo layout.
// In production this would be: import { ... } from "coding-adventures-conduit";
import {
  Application,
  Server,
  html,
  json,
  halt,
  redirect,
} from "../../../../packages/typescript/conduit/src/index.js";

// ── Application setup ─────────────────────────────────────────────────────────

const app = new Application();

// Store a setting — accessible anywhere that has the app reference.
app.set("appName", "Conduit Hello");
app.set("version", "0.1.0");

// ── Before filter: maintenance mode ───────────────────────────────────────────
//
// Runs before every route lookup.  If the path is "/down" we halt immediately
// with 503 so the route handler is never reached.
app.before((req) => {
  if (req.path === "/down") halt(503, "Under maintenance");
});

// ── After filter: access log ───────────────────────────────────────────────────
//
// Runs after every successful route handler.  Cannot modify the response yet,
// but is ideal for logging and metrics.
app.after((req) => {
  process.stdout.write(`[after] ${req.method} ${req.path}\n`);
});

// ── Routes ────────────────────────────────────────────────────────────────────

// Root — plain HTML greeting.
app.get("/", () =>
  html("<h1>Hello from Conduit!</h1><p>Try <a href='/hello/Adhithya'>/hello/Adhithya</a></p>"),
);

// Named capture — extracts :name from the URL path.
app.get("/hello/:name", (req) =>
  json({
    message: `Hello ${req.params["name"] ?? "world"}!`,
    app: app.getSetting("appName"),
  }),
);

// JSON body echo — parse the request body as JSON and send it back.
app.post("/echo", (req) => {
  const body = req.json<Record<string, unknown>>();
  return json(body);
});

// Redirect — 301 Moved Permanently to /.
app.get("/redirect", () => redirect("/", 301));

// Halt — immediately return 403 Forbidden.
app.get("/halt", () => halt(403, "Forbidden — this route always halts"));

// /down — the before filter intercepts this before it reaches here.
// This handler is unreachable when the maintenance filter is active.
app.get("/down", () => html("Maintenance mode is off — we're up!"));

// Error trigger — throws an unhandled exception to exercise the error handler.
app.get("/error", (_req) => {
  throw new Error("Intentional error for demo purposes");
});

// ── Custom not-found handler ──────────────────────────────────────────────────
//
// Called when no route pattern matches the incoming request.
// Returns 404 with a friendly HTML page.
app.notFound((req) =>
  html(
    `<h1>404 Not Found</h1><p>No route matches <code>${req.path}</code></p>`,
    404,
  ),
);

// ── Custom error handler ──────────────────────────────────────────────────────
//
// Called when a route handler throws an exception that is not a HaltError.
// Returns 500 with a JSON body containing the error message.
app.onError((_req, msg) =>
  json({ error: "Internal Server Error", detail: msg }, 500),
);

// ── Server ────────────────────────────────────────────────────────────────────

const HOST = "127.0.0.1";
const PORT = 3000;

const server = new Server(app, { host: HOST, port: PORT });

console.log(`${String(app.getSetting("appName"))} v${String(app.getSetting("version"))}`);
console.log(`Listening on http://${HOST}:${PORT}`);
console.log("");
console.log("Routes:");
console.log("  GET  /                 → HTML greeting");
console.log("  GET  /hello/:name      → JSON with captured name");
console.log("  POST /echo             → echo JSON body");
console.log("  GET  /redirect         → 301 to /");
console.log("  GET  /halt             → 403 Forbidden");
console.log("  GET  /down             → 503 (before filter)");
console.log("  GET  /error            → 500 via custom error handler");
console.log("  GET  /missing          → 404 via custom not-found handler");
console.log("");
console.log("Press Ctrl-C to stop.");

// ── Signal handling ───────────────────────────────────────────────────────────
//
// On SIGINT (Ctrl-C) or SIGTERM (Docker / k8s), call server.stop() which
// signals the Rust background thread to shut down.  The event loop drains
// naturally once all TsFns are released.
process.on("SIGINT", () => {
  console.log("\nShutting down...");
  server.stop();
  process.exit(0);
});
process.on("SIGTERM", () => {
  server.stop();
  process.exit(0);
});

server.serve();
