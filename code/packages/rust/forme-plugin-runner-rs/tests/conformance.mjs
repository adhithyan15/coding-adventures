import { resolve } from "node:path";
import process from "node:process";
import {
  CONFORMANCE_STAGE,
  runRunnerConformance,
} from "../../../typescript/forme-plugin-runner-conformance/dist/index.js";

const executable = process.argv[2];
if (!executable) throw new Error("expected the Rust fixture executable path");

await runRunnerConformance({
  executable: resolve(executable),
  args: [CONFORMANCE_STAGE.id, CONFORMANCE_STAGE.configSchemaHash],
  cwd: process.cwd(),
  modeArgument: true,
  expectedRunner: "forme-plugin-runner-rs",
  expectedRunnerVersion: "0.1.0",
  timeoutMs: 10_000,
});
