import { spawn } from "node:child_process";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createPluginHost,
  type PluginProcessFactory,
} from "@coding-adventures/forme-plugin-host";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import type { StorageApi } from "@coding-adventures/forme-stage";
import { createOrchestrator } from "../src/index.js";

const pluginSource = String.raw`
import process from "node:process";
let buffer = Buffer.alloc(0);
let nextId = -1;
const pending = new Map();
function send(message) {
  const payload = Buffer.from(JSON.stringify(message));
  process.stdout.write(Buffer.concat([
    Buffer.from("Content-Length: " + payload.length + "\r\n\r\n"), payload,
  ]));
}
function respond(id, result) { send({ jsonrpc: "2.0", id, result }); }
function request(method, params) {
  const id = nextId--;
  send({ jsonrpc: "2.0", id, method, params });
  return new Promise(resolve => pending.set(id, resolve));
}
async function handle(message) {
  if (Object.hasOwn(message, "id") && !message.method) {
    pending.get(message.id)?.(message); pending.delete(message.id); return;
  }
  if (message.method === "handshake") {
    respond(message.id, {
      pluginName: message.params.pluginName, pluginVersion: message.params.pluginVersion,
      apiVersion: message.params.apiVersion, protocolVersion: message.params.protocolVersion,
      runner: "orchestrator-e2e", runnerVersion: "1.0.0",
    }); return;
  }
  if (message.method === "announce") {
    respond(message.id, { stage: {
      id: "source", consumes: "Void", produces: "DeployArtifact",
      capabilities: ["storage:read"], configSchemaHash: null,
    } }); return;
  }
  if (message.method === "stage.init") { respond(message.id, null); return; }
  if (message.method === "stage.run") {
    const read = await request("ctx.storage.read", {
      path: "marker.txt", streamId: message.params.streamId,
    });
    respond(message.id, { kind: "single", value: {
      variant: { kind: "dist-tree" }, files: {},
      manifest: { routes: [], assets: [], buildTime: "", buildId: "blake2b:00" },
      marker: read.result.bytes,
    } }); return;
  }
  if (message.method === "stage.dispose") {
    respond(message.id, null); setTimeout(() => process.exit(0), 5);
  }
}
process.stdin.on("data", chunk => {
  buffer = Buffer.concat([buffer, chunk]);
  while (true) {
    const end = buffer.indexOf("\r\n\r\n");
    if (end < 0) return;
    const length = Number(/^Content-Length: ([0-9]+)$/i.exec(buffer.subarray(0, end).toString())[1]);
    const frameEnd = end + 4 + length;
    if (buffer.length < frameEnd) return;
    const payload = buffer.subarray(end + 4, frameEnd);
    buffer = buffer.subarray(frameEnd);
    void handle(JSON.parse(payload.toString()));
  }
});
`;

const roots: string[] = [];
afterEach(async () => {
  await Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true })));
});

async function fixtureRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "forme-orchestrator-plugin-"));
  roots.push(root);
  const plugin = join(root, "e2e");
  await mkdir(plugin);
  await writeFile(join(plugin, "plugin.mjs"), pluginSource);
  await writeFile(join(plugin, "plugin.toml"), `manifestVersion = 1
[plugin]
name = "@example/e2e"
version = "1.0.0"
apiVersion = 1
[runtime]
kind = "node"
entry = "./plugin.mjs"
[[capabilities.required]]
realm = "storage"
scope = "read"
reason = "Read the integration marker"
[[contributes.stages]]
id = "source"
consumes = "Void"
produces = "DeployArtifact"
`);
  return root;
}

describe("subprocess plugin orchestrator integration", () => {
  it("runs a capability-mediated StageRef twice across session disposal", async () => {
    const launches = vi.fn();
    const factory: PluginProcessFactory = {
      async launch(request) {
        launches();
        const entry = join(request.workingDirectory, "plugin.mjs");
        await writeFile(entry, request.plugin.entryBytes);
        const child = spawn(process.execPath, [entry], {
          cwd: request.workingDirectory,
          env: { PATH: process.env.PATH ?? "" },
          stdio: ["pipe", "pipe", "pipe"],
        });
        return {
          isolation: "sandboxed",
          isolationProvider: "orchestrator-e2e-fixture",
          launchedManifestHash: request.plugin.manifestHash,
          launchedConfigSchemaHash: request.configSchema?.hash ?? null,
          stdin: child.stdin, stdout: child.stdout, stderr: child.stderr,
          exited: new Promise(resolve => child.once("exit", (code, signal) => resolve({ code, signal }))),
          signal: signal => { child.kill(signal); },
        };
      },
    };
    const marker = new TextEncoder().encode("integrated");
    const storage: StorageApi = {
      async read() { return marker; },
      async readBounded(_path, maxBytes) {
        if (marker.byteLength > maxBytes) throw new Error("bounded read overflow");
        return marker;
      },
      async write() {}, async exists() { return true; },
      async *list() {}, async *watch() {}, async remove() {},
      async stat() { return { size: marker.byteLength, mtimeMs: 0, type: "file" }; },
    };
    const host = await createPluginHost({
      roots: [await fixtureRoot()],
      grants: { "@example/e2e": ["storage:read"] },
      capabilityApis: { storage },
      processFactory: factory,
      handshakeTimeoutMs: 1_000,
      requestTimeoutMs: 1_000,
      disposeGracePeriodMs: 100,
      killGracePeriodMs: 100,
    });
    const orchestrator = createOrchestrator({ pluginHost: host });
    const config: PipelineConfig = {
      name: "plugin-e2e",
      settings: {
        storageRoot: ".", cacheDir: null, reproducibleBuild: true,
        maxConcurrency: 1, logLevel: "error", bestEffort: false, deadlineMs: null,
      },
      stages: [{
        id: "source",
        stage: { kind: "stage-ref", packageName: "@example/e2e", export: "source" },
        capabilities: ["storage:read"],
      }],
    };
    const pipeline = await orchestrator.buildPipeline(config);
    const first = await orchestrator.runOnce(pipeline, { useCache: false });
    const second = await orchestrator.runOnce(pipeline, { useCache: false });
    expect(first.outcome).toBe("success");
    expect(second.outcome).toBe("success");
    expect(first.outputs.source).toMatchObject({
      marker: Buffer.from(marker).toString("base64"),
    });
    expect(launches).toHaveBeenCalledTimes(2);
    await orchestrator.dispose();
  });
});
