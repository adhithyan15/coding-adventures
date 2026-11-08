#!/usr/bin/env node
import * as fs from "node:fs";
import * as path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { parseArgs, USAGE_EXIT, parseFile, buildSelfCorpusSuite } from "../dist/index.js";

async function main() {
  let options;
  try {
    options = parseArgs(process.argv.slice(2));
  } catch (err) {
    console.error(`usage error: ${err instanceof Error ? err.message : String(err)}`);
    process.exit(USAGE_EXIT);
  }

  const __dirname = path.dirname(fileURLToPath(import.meta.url));
  const candidatePaths = [
    process.env.TF_CORPUS,
    path.resolve(__dirname, "../../../../specs/fixtures/test-bootstrap-v1/cases.tfv"),
    path.resolve(process.cwd(), "specs/fixtures/test-bootstrap-v1/cases.tfv"),
    path.resolve(process.cwd(), "code/specs/fixtures/test-bootstrap-v1/cases.tfv"),
  ];

  const foundPath = candidatePaths.find((p) => p !== undefined && fs.existsSync(p));

  if (!foundPath) {
    console.error("test-bootstrap-v1/cases.tfv could not be found");
    process.exit(1);
  }

  const content = fs.readFileSync(foundPath, "utf-8");
  const records = parseFile(content);
  const suite = buildSelfCorpusSuite(records);

  const summary = await suite.run(options);
  process.exit(summary.exitCode);
}

main().catch((err) => {
  console.error("unexpected fatal error:", err);
  process.exit(1);
});
