import { resolve } from "node:path";
import process from "node:process";
import {
  CONFORMANCE_STAGE,
  runRunnerConformance,
} from "../../../typescript/forme-plugin-runner-conformance/dist/index.js";

const executable = process.argv[2];
if (!executable) throw new Error("expected the Python interpreter path");

await runRunnerConformance({
  executable: resolve(executable),
  args: [
    "-m", "coverage", "run", "--parallel-mode", "--source=forme_plugin_runner",
    "tests/fixture.py", CONFORMANCE_STAGE.id, CONFORMANCE_STAGE.configSchemaHash,
  ],
  cwd: process.cwd(),
  modeArgument: true,
  expectedRunner: "forme-plugin-runner-py",
  expectedRunnerVersion: "0.1.0",
  timeoutMs: 10_000,
});
