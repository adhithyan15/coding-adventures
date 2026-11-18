import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const cacheDir = path.join(packageRoot, "node_modules", ".cache");
const compiler = path.join(packageRoot, "node_modules", "typescript", "bin", "tsc");
const config = path.join(packageRoot, "tsconfig.json");
const registrySource = path.join(
  packageRoot,
  "src",
  "language-source-input-registry.json",
);

/**
 * Build the production CLI once outside each fixture's process-tree budget.
 * Each Vitest run owns a fresh ignored output directory; placing it beneath
 * node_modules lets emitted createRequire(import.meta.url) calls find the
 * package's normal dependencies without a custom resolver.
 */
export default function setupCli() {
  mkdirSync(cacheDir, { recursive: true });
  const outputDir = mkdtempSync(path.join(cacheDir, "build-tool-cli-test-"));
  const registryOutput = path.join(
    outputDir,
    "language-source-input-registry.json",
  );
  process.env.BUILD_TOOL_TEST_CLI_DIR = outputDir;

  try {
    execFileSync(
      process.execPath,
      [
        compiler,
        "--project",
        config,
        "--outDir",
        outputDir,
        "--noEmitOnError",
        "--declaration",
        "false",
        "--sourceMap",
        "false",
      ],
      { cwd: packageRoot, stdio: "inherit", timeout: 120_000 },
    );
    copyFileSync(registrySource, registryOutput);
    writeFileSync(
      path.join(outputDir, "package.json"),
      '{"type":"module"}\n',
    );
    if (!readFileSync(registrySource).equals(readFileSync(registryOutput))) {
      throw new Error("emitted CLI registry differs from the source registry bytes");
    }
    createRequire(pathToFileURL(path.join(outputDir, "starlark-evaluator.js")))
      .resolve("typescript");
  } catch (error) {
    rmSync(outputDir, { recursive: true, force: true });
    delete process.env.BUILD_TOOL_TEST_CLI_DIR;
    throw error;
  }

  return () => {
    rmSync(outputDir, { recursive: true, force: true });
    delete process.env.BUILD_TOOL_TEST_CLI_DIR;
  };
}
