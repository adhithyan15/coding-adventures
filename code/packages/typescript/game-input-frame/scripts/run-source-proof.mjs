import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repositoryRoot = resolve(packageRoot, "../../../..");
const rustWorkspace = resolve(repositoryRoot, "code/packages/rust");
const sourcePath = resolve(packageRoot, "src/digital.ts");
const expectedPath = relative(repositoryRoot, sourcePath).split(sep).join("/");
const digest = createHash("sha256").update(readFileSync(sourcePath)).digest("hex");
const args = [
  "run",
  "--locked",
  "-q",
  "-p",
  "typescript-warrant-ir",
  "--example",
  "source-proof",
  "--",
  "--source",
  sourcePath,
  "--expected-path",
  expectedPath,
  "--sha256",
  digest,
  "--query",
  "healthy:Proved",
];

const result = spawnSync("cargo", args, { cwd: rustWorkspace, encoding: "utf8" });
if (result.stdout !== null) process.stdout.write(result.stdout);
if (result.stderr !== null) process.stderr.write(result.stderr);
if (result.error !== undefined) {
  process.stderr.write(`${result.error.message}\n`);
  process.exitCode = 1;
} else if (result.status !== 0) {
  process.exitCode = result.status ?? 1;
}
