import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";
import { scriptInventoryPlugin } from "./script-inventory-plugin.ts";

// The canonical script JSON lives outside this package, at
// code/learning/human-languages/data/scripts/, and `scriptdata.ts` imports it
// directly rather than copying it — so the pen paths here can never drift from
// the data the lessons teach from. Vite guards reads outside the project root,
// so the repo root has to be declared legal, exactly as the app's config does.
//
// This config runs in Node, so convert file URLs with Node's platform-aware
// helper. URL.pathname leaves a Windows drive path as `/C:/...`; feeding that
// string to node:path turns it into `<current drive>\C:\...` and makes the
// shard boundary fail before Vitest can start.
const repoRoot = fileURLToPath(new URL("../../../..", import.meta.url));
const curriculumRoot = fileURLToPath(
  new URL("../../../../code/learning/human-languages", import.meta.url),
);

export default defineConfig({
  plugins: [scriptInventoryPlugin({ curriculumRoot })],
  server: {
    fs: { allow: [repoRoot] },
  },
  test: {
    // jsdom, for two tests and two only: the SVG serialiser's escaping is
    // checked by handing the output to a REAL parser and asserting that a
    // hostile caption cannot break out of an attribute or smuggle in a
    // <script>. A string comparison would pass on markup no browser accepts,
    // which is exactly the bug those tests exist to catch — so the environment
    // moves with them rather than the tests being weakened to fit.
    environment: "jsdom",
    coverage: {
      provider: "v8",
      thresholds: {
        lines: 90,
      },
    },
  },
});
