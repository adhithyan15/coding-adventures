import { mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const output = join(root, "dist", "worker", "forme-product-worker.cjs");
await mkdir(dirname(output), { recursive: true });
await build({
  entryPoints: [join(root, "src", "worker-main.ts")],
  outfile: output,
  bundle: true,
  platform: "node",
  target: "node20",
  format: "cjs",
  sourcemap: false,
  legalComments: "none",
  logLevel: "warning",
});
