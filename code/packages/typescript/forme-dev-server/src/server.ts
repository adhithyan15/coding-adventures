import { createServer, type ServerResponse } from "node:http";
import type { AddressInfo } from "node:net";
import type { DeployArtifact } from "@coding-adventures/forme-types";

const CLIENT_PATH = "/__forme/client.js";
const EVENTS_PATH = "/__forme/events";
const STATUS_PATH = "/__forme/status";
const CLIENT_TAG = `<script type="module" src="${CLIENT_PATH}"></script>`;

const CLIENT_SOURCE = `const overlayId = "__forme_build_error";
const events = new EventSource("${EVENTS_PATH}");
events.addEventListener("reload", () => location.reload());
events.addEventListener("build-error", event => {
  const payload = JSON.parse(event.data);
  let overlay = document.getElementById(overlayId);
  if (!overlay) {
    overlay = document.createElement("pre");
    overlay.id = overlayId;
    overlay.style.cssText = "position:fixed;z-index:2147483647;inset:auto 1rem 1rem 1rem;max-height:45vh;overflow:auto;padding:1rem;border:2px solid #b42318;border-radius:.5rem;background:#fff1f0;color:#7a271a;box-shadow:0 8px 30px #0005;font:13px/1.5 ui-monospace,monospace;white-space:pre-wrap";
    document.body.append(overlay);
  }
  overlay.textContent = "Forme rebuild failed — showing the last good output\\n\\n" + payload.message;
});
`;

export interface PreviewSnapshot {
  readonly buildId: string;
  readonly files: ReadonlyMap<string, Uint8Array>;
}

export interface BuildFailure {
  readonly message: string;
}

export interface DevServerOptions {
  readonly host?: string;
  readonly port?: number;
}

export interface DevServerAddress {
  readonly host: string;
  readonly port: number;
  readonly url: string;
}

export interface DevServer {
  readonly address: DevServerAddress;
  publish(snapshot: PreviewSnapshot): void;
  publishFailure(failure: BuildFailure): void;
  close(): Promise<void>;
}

export function snapshotFromOutputs(
  buildId: string,
  outputs: Readonly<Record<string, unknown>>,
): PreviewSnapshot {
  const files = new Map<string, Uint8Array>();
  for (const outputName of Object.keys(outputs).sort()) {
    const artifact = asDistArtifact(outputs[outputName], outputName);
    for (const path of Object.keys(artifact.files).sort()) {
      const normalized = normalizeArtifactPath(path);
      if (files.has(normalized)) {
        throw new Error(`preview output collision at ${JSON.stringify(normalized)}`);
      }
      files.set(normalized, artifact.files[path]!);
    }
  }
  if (files.size === 0) throw new Error("preview build produced no static files");
  return { buildId, files };
}

export async function startDevServer(options: DevServerOptions = {}): Promise<DevServer> {
  const host = options.host ?? "127.0.0.1";
  const port = options.port ?? 3000;
  let snapshot: PreviewSnapshot | null = null;
  let failure: BuildFailure | null = null;
  const clients = new Set<ServerResponse>();

  const server = createServer((request, response) => {
    const method = request.method ?? "GET";
    if (method !== "GET" && method !== "HEAD") {
      response.writeHead(405, { Allow: "GET, HEAD", "Content-Type": "text/plain; charset=utf-8" });
      response.end("Method Not Allowed\n");
      return;
    }

    const pathname = requestPath(request.url ?? "/");
    if (pathname === null) {
      response.writeHead(400, { "Content-Type": "text/plain; charset=utf-8" });
      response.end("Bad Request\n");
      return;
    }
    if (pathname === EVENTS_PATH) {
      response.writeHead(200, {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache, no-store, must-revalidate",
        Connection: "keep-alive",
        "X-Accel-Buffering": "no",
      });
      response.write("retry: 500\n\n");
      clients.add(response);
      request.once("close", () => clients.delete(response));
      if (failure !== null) sendEvent(response, "build-error", failure);
      return;
    }
    if (pathname === STATUS_PATH) {
      sendBytes(response, method, 200, "application/json; charset=utf-8", new TextEncoder().encode(
        `${JSON.stringify({
          state: failure === null ? (snapshot === null ? "building" : "ready") : "failed",
          buildId: snapshot?.buildId ?? null,
          error: failure?.message ?? null,
        })}\n`,
      ));
      return;
    }
    if (pathname === CLIENT_PATH) {
      sendBytes(response, method, 200, "text/javascript; charset=utf-8", new TextEncoder().encode(CLIENT_SOURCE));
      return;
    }
    if (snapshot === null) {
      const detail = failure?.message ?? "The first preview build is still running.";
      sendHtml(response, method, 503, errorPage("Forme preview unavailable", detail));
      return;
    }

    const filePath = resolveFile(pathname, snapshot.files);
    if (filePath === null) {
      sendHtml(response, method, 404, errorPage("Not found", pathname));
      return;
    }
    const bytes = snapshot.files.get(filePath)!;
    const contentType = mimeType(filePath);
    if (contentType.startsWith("text/html")) {
      const html = injectClient(new TextDecoder().decode(bytes));
      sendHtml(response, method, 200, html);
      return;
    }
    sendBytes(response, method, 200, contentType, bytes);
  });

  await new Promise<void>((resolve, reject) => {
    const fail = (error: Error) => reject(error);
    server.once("error", fail);
    server.listen(port, host, () => {
      server.removeListener("error", fail);
      resolve();
    });
  });
  const bound = server.address() as AddressInfo;
  const address = { host, port: bound.port, url: `http://${displayHost(host)}:${bound.port}` };

  return {
    address,
    publish(next) {
      snapshot = next;
      failure = null;
      broadcast(clients, "reload", { buildId: next.buildId });
    },
    publishFailure(next) {
      failure = next;
      broadcast(clients, "build-error", next);
    },
    async close() {
      for (const client of clients) client.end();
      clients.clear();
      if (!server.listening) return;
      await new Promise<void>((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
    },
  };
}

function asDistArtifact(value: unknown, outputName: string): DeployArtifact {
  if (typeof value !== "object" || value === null) {
    throw new Error(`preview output ${JSON.stringify(outputName)} is not a DeployArtifact`);
  }
  const candidate = value as Partial<DeployArtifact>;
  if (candidate.variant?.kind !== "dist-tree" || typeof candidate.files !== "object" || candidate.files === null) {
    throw new Error(`preview output ${JSON.stringify(outputName)} is not a static dist tree`);
  }
  for (const [path, bytes] of Object.entries(candidate.files)) {
    if (!(bytes instanceof Uint8Array)) {
      throw new Error(`preview file ${JSON.stringify(path)} in ${JSON.stringify(outputName)} is not bytes`);
    }
  }
  return candidate as DeployArtifact;
}

function normalizeArtifactPath(path: string): string {
  const normalized = path.replaceAll("\\", "/").replace(/^\/+/, "");
  if (normalized.length === 0 || normalized.split("/").some(part => part === "" || part === "." || part === "..")) {
    throw new Error(`unsafe preview artifact path ${JSON.stringify(path)}`);
  }
  return normalized;
}

function requestPath(rawUrl: string): string | null {
  try {
    const url = new URL(rawUrl, "http://forme.local");
    const decoded = decodeURIComponent(url.pathname);
    if (decoded.includes("\\") || decoded.includes("\0")) return null;
    return decoded;
  } catch {
    return null;
  }
}

function resolveFile(pathname: string, files: ReadonlyMap<string, Uint8Array>): string | null {
  const segments = pathname.replace(/^\/+|\/+$/g, "").split("/").filter(Boolean);
  if (segments.length === 0 && files.has("index.html")) return "index.html";
  // A production artifact can intentionally author URLs beneath a project
  // deployment prefix (for example /coding-adventures/assets/...). Preview
  // has no deploy adapter, so try the exact path first and then remove leading
  // path segments until a unique in-memory artifact path is reached.
  for (let start = 0; start < segments.length; start++) {
    const plain = segments.slice(start).join("/");
    const candidates = pathname.endsWith("/")
      ? [`${plain}/index.html`]
      : [plain, `${plain}.html`, `${plain}/index.html`];
    for (const candidate of candidates) if (files.has(candidate)) return candidate;
  }
  return null;
}

function injectClient(html: string): string {
  if (html.includes(CLIENT_PATH)) return html;
  const body = html.lastIndexOf("</body>");
  return body === -1 ? `${html}${CLIENT_TAG}` : `${html.slice(0, body)}${CLIENT_TAG}${html.slice(body)}`;
}

function errorPage(title: string, detail: string): string {
  return `<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>${escapeHtml(title)}</title><style>body{max-width:52rem;margin:10vh auto;padding:0 1.25rem;font:16px/1.6 system-ui;color:#321}pre{padding:1rem;border-left:4px solid #b42318;background:#fff1f0;white-space:pre-wrap}</style><h1>${escapeHtml(title)}</h1><pre>${escapeHtml(detail)}</pre>${CLIENT_TAG}`;
}

function escapeHtml(value: string): string {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
}

function sendHtml(response: ServerResponse, method: string, status: number, html: string): void {
  sendBytes(response, method, status, "text/html; charset=utf-8", new TextEncoder().encode(html));
}

function sendBytes(
  response: ServerResponse,
  method: string,
  status: number,
  contentType: string,
  bytes: Uint8Array,
): void {
  response.writeHead(status, {
    "Content-Type": contentType,
    "Content-Length": bytes.byteLength,
    "Cache-Control": "no-cache, no-store, must-revalidate",
    "X-Content-Type-Options": "nosniff",
  });
  response.end(method === "HEAD" ? undefined : bytes);
}

function broadcast(clients: ReadonlySet<ServerResponse>, event: string, data: unknown): void {
  for (const client of clients) sendEvent(client, event, data);
}

function sendEvent(response: ServerResponse, event: string, data: unknown): void {
  response.write(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`);
}

function mimeType(path: string): string {
  const extension = path.slice(path.lastIndexOf(".")).toLowerCase();
  return ({
    ".css": "text/css; charset=utf-8",
    ".gif": "image/gif",
    ".html": "text/html; charset=utf-8",
    ".ico": "image/x-icon",
    ".jpeg": "image/jpeg",
    ".jpg": "image/jpeg",
    ".js": "text/javascript; charset=utf-8",
    ".json": "application/json; charset=utf-8",
    ".png": "image/png",
    ".svg": "image/svg+xml",
    ".txt": "text/plain; charset=utf-8",
    ".webp": "image/webp",
    ".xml": "application/xml; charset=utf-8",
  } as Record<string, string>)[extension] ?? "application/octet-stream";
}

function displayHost(host: string): string {
  return host.includes(":") ? `[${host}]` : host;
}
