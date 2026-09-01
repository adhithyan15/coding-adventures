import { get, type IncomingMessage } from "node:http";
import { afterEach, describe, expect, it } from "vitest";
import {
  snapshotFromOutputs,
  startDevServer,
  type DevServer,
} from "../src/index.js";

const encoder = new TextEncoder();
const servers: DevServer[] = [];

afterEach(async () => {
  await Promise.all(servers.splice(0).map(server => server.close()));
});

function artifact(files: Record<string, string | Uint8Array>): unknown {
  return {
    variant: { kind: "dist-tree" },
    files: Object.fromEntries(Object.entries(files).map(([path, value]) => [
      path,
      typeof value === "string" ? encoder.encode(value) : value,
    ])),
    manifest: { routes: [], assets: [], buildTime: "fixed", buildId: "blake2b:test" },
  };
}

async function server(): Promise<DevServer> {
  const value = await startDevServer({ port: 0 });
  servers.push(value);
  return value;
}

describe("snapshotFromOutputs", () => {
  it("merges named dist trees in deterministic order", () => {
    const snapshot = snapshotFromOutputs("build-1", {
      surface: artifact({ "index.html": "home" }),
      articles: artifact({ "blog/post.html": "post" }),
    });
    expect(snapshot.buildId).toBe("build-1");
    expect([...snapshot.files.keys()]).toEqual(["blog/post.html", "index.html"]);
  });

  it("rejects collisions, unsafe paths, non-static outputs, and empty output", () => {
    expect(() => snapshotFromOutputs("x", {
      a: artifact({ "same.html": "a" }),
      b: artifact({ "same.html": "b" }),
    })).toThrow(/collision/);
    expect(() => snapshotFromOutputs("x", { a: artifact({ "../escape": "x" }) })).toThrow(/unsafe/);
    expect(() => snapshotFromOutputs("x", { a: { variant: { kind: "pdf" }, files: {} } })).toThrow(/static dist tree/);
    expect(() => snapshotFromOutputs("x", {})).toThrow(/no static files/);
  });
});

describe("preview HTTP server", () => {
  it("serves injected HTML, typed assets, extensionless routes, and HEAD", async () => {
    const preview = await server();
    preview.publish(snapshotFromOutputs("build-1", {
      site: artifact({
        "index.html": "<!doctype html><body>home</body>",
        "about.html": "<!doctype html><body>about</body>",
        "assets/site.css": "body{}",
      }),
    }));

    const home = await fetch(`${preview.address.url}/`);
    expect(home.status).toBe(200);
    expect(await home.text()).toContain('<script type="module" src="/__forme/client.js"></script></body>');
    const about = await fetch(`${preview.address.url}/about`);
    expect(await about.text()).toContain("about");
    const prefixedAbout = await fetch(`${preview.address.url}/coding-adventures/about.html`);
    expect(await prefixedAbout.text()).toContain("about");
    const css = await fetch(`${preview.address.url}/assets/site.css`);
    expect(css.headers.get("content-type")).toBe("text/css; charset=utf-8");
    expect(css.headers.get("cache-control")).toContain("no-store");
    expect(await css.text()).toBe("body{}");
    expect(await (await fetch(`${preview.address.url}/coding-adventures/assets/site.css`)).text()).toBe("body{}");
    const head = await fetch(`${preview.address.url}/assets/site.css`, { method: "HEAD" });
    expect(head.headers.get("content-length")).toBe("6");
    expect(await head.text()).toBe("");
  });

  it("keeps the last good snapshot and reports a failed rebuild", async () => {
    const preview = await server();
    preview.publish(snapshotFromOutputs("good-build", {
      site: artifact({ "index.html": "<!doctype html><body>good</body>" }),
    }));
    preview.publishFailure({ message: "parse failed <unsafe>" });

    expect(await (await fetch(`${preview.address.url}/`)).text()).toContain("good");
    const status = await (await fetch(`${preview.address.url}/__forme/status`)).json();
    expect(status).toEqual({ state: "failed", buildId: "good-build", error: "parse failed <unsafe>" });
  });

  it("serves an escaped 503 error page before the first good build", async () => {
    const preview = await server();
    preview.publishFailure({ message: "bad <script>alert(1)</script>" });
    const response = await fetch(`${preview.address.url}/`);
    expect(response.status).toBe(503);
    const html = await response.text();
    expect(html).toContain("bad &lt;script&gt;");
    expect(html).not.toContain("bad <script>");
  });

  it("broadcasts reload and build-error events to connected browsers", async () => {
    const preview = await server();
    const stream = await connect(`${preview.address.url}/__forme/events`);
    const reload = waitFor(stream, "event: reload");
    preview.publish(snapshotFromOutputs("next-build", {
      site: artifact({ "index.html": "next" }),
    }));
    expect(await reload).toContain('data: {"buildId":"next-build"}');

    const failure = waitFor(stream, "event: build-error");
    preview.publishFailure({ message: "broken" });
    expect(await failure).toContain('data: {"message":"broken"}');
    stream.destroy();
  });

  it("serves the external client and rejects unsupported requests", async () => {
    const preview = await server();
    const client = await fetch(`${preview.address.url}/__forme/client.js`);
    expect(client.headers.get("content-type")).toBe("text/javascript; charset=utf-8");
    expect(await client.text()).toContain("new EventSource");
    expect((await fetch(`${preview.address.url}/missing`)).status).toBe(503);
    expect((await fetch(`${preview.address.url}/%00`)).status).toBe(400);
    expect((await fetch(`${preview.address.url}/`, { method: "POST" })).status).toBe(405);
  });
});

function connect(url: string): Promise<IncomingMessage> {
  return new Promise((resolve, reject) => {
    const call = get(url, resolve);
    call.once("error", reject);
  });
}

function waitFor(stream: IncomingMessage, pattern: string): Promise<string> {
  return new Promise((resolve, reject) => {
    let text = "";
    const onData = (chunk: Buffer) => {
      text += chunk.toString("utf8");
      if (!text.includes(pattern)) return;
      cleanup();
      resolve(text);
    };
    const onError = (error: Error) => { cleanup(); reject(error); };
    const cleanup = () => {
      stream.removeListener("data", onData);
      stream.removeListener("error", onError);
    };
    stream.on("data", onData);
    stream.on("error", onError);
  });
}
