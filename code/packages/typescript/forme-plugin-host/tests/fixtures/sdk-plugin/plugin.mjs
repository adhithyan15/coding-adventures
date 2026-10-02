import { defineStage } from "@coding-adventures/forme-stage";
import { KERNEL_API_VERSION, Kinds } from "@coding-adventures/forme-types";
import { runPlugin } from "./runner/index.js";

const stage = defineStage({
  name: "@example/sdk",
  version: "1.0.0",
  apiVersion: KERNEL_API_VERSION,
  description: "FM-B050 TypeScript runner end-to-end fixture",
  consumes: Kinds.ContentNode,
  produces: Kinds.ContentNode,
  capabilities: ["storage:read"],
  configSchema: null,
  async run(input, _config, ctx) {
    if (input?.watchPath) {
      for await (const change of ctx.storage.watch(input.watchPath)) {
        return { ...input, change };
      }
      return { ...input, ended: true };
    }
    if (input?.readPath) {
      const bytes = await ctx.storage.read(input.readPath);
      ctx.logger.info("sdk fixture", { bytes: bytes.byteLength });
      return { ...input, bytes };
    }
    return input;
  },
});

await runPlugin(stage);
