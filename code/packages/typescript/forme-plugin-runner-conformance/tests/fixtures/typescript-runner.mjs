import { StageError } from "@coding-adventures/forme-errors";
import { defineStage } from "@coding-adventures/forme-stage";
import { KERNEL_API_VERSION, Kinds, streamOf } from "@coding-adventures/forme-types";
import { runPlugin } from "./runner/index.js";

const common = {
  name: "@forme/conformance",
  version: "1.0.0",
  apiVersion: KERNEL_API_VERSION,
  description: "language-neutral runner conformance fixture",
  capabilities: [
    "storage:read", "storage:write", "env:ALLOWED", "filesystem:user",
    "network:example.com", "system:time:wallclock", "system:shell",
  ],
  configSchema: { type: "object" },
};

const stages = {
  single: defineStage({
    ...common,
    consumes: Kinds.ContentNode,
    produces: Kinds.ContentNode,
    async run(input, _config, ctx) {
      if (input.operation === "error") {
        throw new StageError({ code: "FIXTURE", message: "fixture failed", fields: { line: 3 } });
      }
      if (input.operation === "waitForCancel") {
        while (true) {
          ctx.cancellation.throwIfCancelled();
          await new Promise(resolve => setTimeout(resolve, 2));
        }
      }
      if (input.operation === "capabilityDenied") {
        return { value: await ctx.env.get("DENIED") };
      }
      if (input.operation === "malformedResponse") {
        return { value: await ctx.env.get("MALFORMED") };
      }
      if (input.operation === "context") {
        const bytes = await ctx.storage.read("posts/a.md");
        await ctx.storage.write("out/a.md", bytes);
        const entries = [];
        for await (const entry of ctx.storage.list("posts")) entries.push(entry);
        const watched = [];
        for await (const entry of ctx.storage.watch("posts")) watched.push(entry);
        await ctx.storage.remove("out/stale.md");
        const response = await ctx.network.fetch("https://example.com/data", {
          method: "POST", body: new Uint8Array([1, 2, 3]), headers: { "x-test": "yes" },
        });
        await ctx.filesystem.writeAbsolute("/safe/out", bytes);
        const shell = await ctx.shell.run("tool", ["arg"], { stdin: bytes });
        ctx.logger.info("runner fixture", { ok: true });
        return {
          bytes,
          bounded: await ctx.storage.readBounded("posts/a.md", 16),
          exists: await ctx.storage.exists("posts/a.md"),
          stat: await ctx.storage.stat("posts/a.md"),
          entries,
          watched,
          env: await ctx.env.get("ALLOWED"),
          envRequired: await ctx.env.getOrThrow("ALLOWED"),
          nowMs: await ctx.time.nowMs(),
          nowIso: await ctx.time.nowIso(),
          home: await ctx.filesystem.homeDir(),
          temp: await ctx.filesystem.tempDir(),
          absolute: await ctx.filesystem.readAbsolute("/safe/a"),
          absoluteBounded: await ctx.filesystem.readAbsoluteBounded("/safe/a", 16),
          status: response.status,
          body: new Uint8Array(await response.arrayBuffer()),
          shell,
        };
      }
      return input.value;
    },
  }),
  stream: defineStage({
    ...common,
    consumes: streamOf(Kinds.ContentNode),
    produces: streamOf(Kinds.ContentNode),
    async *run(input) { for await (const value of input) yield value; },
  }),
  "stream-single": defineStage({
    ...common,
    consumes: streamOf(Kinds.ContentNode),
    produces: Kinds.ContentNode,
    async run(input) { const values = []; for await (const value of input) values.push(value); return { values }; },
  }),
  "single-stream": defineStage({
    ...common,
    consumes: Kinds.ContentNode,
    produces: streamOf(Kinds.ContentNode),
    async *run(input) { yield input; yield { copy: input }; },
  }),
};

const mode = process.argv[4] ?? "single";
const stage = stages[mode];
if (!stage) throw new Error(`unknown conformance fixture mode: ${mode}`);
await runPlugin(stage, {
  maxFrameBytes: 4096,
  maxHeaderBytes: 512,
  maxBufferedStreamValues: 4,
  maxBufferedStreamBytes: 1024,
});
