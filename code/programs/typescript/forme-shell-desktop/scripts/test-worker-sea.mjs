import { spawnSync } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const suffix = process.platform === "win32" ? ".exe" : "";
const worker = new URL(`../dist/worker/forme-product-worker${suffix}`, import.meta.url);
const output = await mkdtemp(join(tmpdir(), "forme-worker-sea-"));
try {
  const request = JSON.stringify({
    schemaVersion: 1,
    project: {
      schemaVersion: 1,
      projectId: "01952c0d-7e63-7000-8000-000000000068",
      title: "SEA smoke test",
      site: { baseUrl: null, themeId: "forme-classless" },
      workflow: { lastPublication: null },
      activeDocumentId: null,
      documents: [],
    },
    revision: "sea-smoke-revision",
    output,
  });
  const result = spawnSync(fileURLToPath(worker), [], {
    input: request,
    encoding: "utf8",
    maxBuffer: 9 * 1024 * 1024,
  });
  if (result.status !== 0 || result.stderr !== "") {
    throw new Error(`the bundled worker failed its smoke test with status ${result.status}`);
  }
  const response = JSON.parse(result.stdout);
  if (response.schemaVersion !== 1 || response.revision !== "sea-smoke-revision") {
    throw new Error("the bundled worker returned the wrong snapshot metadata");
  }
} finally {
  await rm(output, { recursive: true, force: true });
}
