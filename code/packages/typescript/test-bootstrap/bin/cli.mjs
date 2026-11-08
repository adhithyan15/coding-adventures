#!/usr/bin/env node
import * as fs from "node:fs";
import * as path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  testBootstrapCliSpec,
  parseFile,
  buildSelfCorpusSuite,
  USAGE_EXIT,
  Parser,
  ParseErrors,
} from "../dist/index.js";

async function main() {
  let parseResult;
  try {
    const parser = new Parser(testBootstrapCliSpec, process.argv);
    parseResult = parser.parse();
  } catch (err) {
    if (err instanceof ParseErrors) {
      for (const e of err.errors) {
        console.error(`usage error [${e.code}]: ${e.message}`);
      }
    } else {
      console.error(`usage error: ${err instanceof Error ? err.message : String(err)}`);
    }
    process.exit(USAGE_EXIT);
  }

  // Handle help
  if ("text" in parseResult) {
    process.stdout.write(parseResult.text + "\n");
    process.exit(0);
  }

  // Handle version
  if ("version" in parseResult && !("flags" in parseResult)) {
    process.stdout.write(parseResult.version + "\n");
    process.exit(0);
  }

  const { flags, arguments: args } = parseResult;
  const rawFiles = args.files;
  const files = (
    Array.isArray(rawFiles)
      ? rawFiles.flat().map(String)
      : typeof rawFiles === "string"
      ? [rawFiles]
      : []
  ).filter((f) => f.length > 0);

  const forceSelfCorpus = Boolean(flags["self-corpus"]);
  const seed = typeof flags.seed === "number" ? flags.seed : 0;
  const sabotage = typeof flags.sabotage === "string" ? flags.sabotage : "none";
  const reporter = typeof flags.reporter === "string" ? flags.reporter : "tap";
  const filter = typeof flags.filter === "string" ? flags.filter : undefined;
  const bail = Boolean(flags.bail);
  const timeout = typeof flags.timeout === "number" ? flags.timeout : 5000;

  // If specific files are given and --self-corpus is not forced, execute files
  if (files.length > 0 && !forceSelfCorpus) {
    const resolvedFiles = files.map((f) => path.resolve(process.cwd(), f));
    for (const f of resolvedFiles) {
      if (!fs.existsSync(f)) {
        console.error(`test file not found: ${f}`);
        process.exit(1);
      }
    }

    const nodeArgs = [
      "--experimental-strip-types",
      "--test",
      ...resolvedFiles,
    ];

    const cleanEnv = { ...process.env };
    delete cleanEnv.NODE_TEST_CONTEXT;

    const child = spawnSync(process.execPath, nodeArgs, {
      stdio: "pipe",
      encoding: "utf-8",
      env: {
        ...cleanEnv,
        TF_SEED: String(seed),
        TF_SABOTAGE: sabotage,
      },
    });

    if (child.stdout) {
      process.stdout.write(child.stdout);
    }
    if (child.stderr) {
      process.stderr.write(child.stderr);
    }
    process.exit(child.status ?? (child.signal ? 1 : 0));
  }

  // Otherwise, run self-corpus
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

  const summary = await suite.run({
    seed: BigInt(seed),
    sabotage: sabotage !== "none",
  });

  process.exit(summary.exitCode);
}

main().catch((err) => {
  console.error("unexpected fatal error:", err);
  process.exit(1);
});
