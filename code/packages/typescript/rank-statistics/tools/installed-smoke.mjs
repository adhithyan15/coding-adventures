import assert from "node:assert/strict";
import { createHash, randomBytes } from "node:crypto";
import {
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const packageRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const npmCli = process.env.npm_execpath;
assert.ok(npmCli, "npm must provide npm_execpath when this smoke is run as an npm script");
const tempParent = path.resolve(os.tmpdir());
const tempRoot = mkdtempSync(path.join(tempParent, "ca-rank-statistics-smoke-"));
const marker = randomBytes(16).toString("hex");
writeFileSync(path.join(tempRoot, ".owned-smoke-root"), marker, { flag: "wx" });
const artifactDir = path.join(tempRoot, "artifact");
const consumerDir = path.join(tempRoot, "consumer");
mkdirSync(artifactDir);
mkdirSync(consumerDir);

function run(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, encoding: "utf8", windowsHide: true });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed (${result.status})\n${result.stdout}\n${result.stderr}`);
  }
  return result.stdout;
}

function runNpm(args, cwd) {
  return run(process.execPath, [npmCli, ...args], cwd);
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function isWithin(parent, candidate) {
  const relative = path.relative(parent, candidate);
  return relative === "" || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative));
}

function assertSafeRemovalTarget(target) {
  const resolvedParent = path.resolve(tempParent);
  const resolvedTarget = path.resolve(target);
  assert.equal(path.basename(resolvedTarget).startsWith("ca-rank-statistics-smoke-"), true);
  assert.equal(isWithin(resolvedParent, resolvedTarget), true);
  const stat = lstatSync(resolvedTarget);
  assert.equal(stat.isDirectory(), true);
  assert.equal(stat.isSymbolicLink(), false);
  assert.equal(readFileSync(path.join(resolvedTarget, ".owned-smoke-root"), "utf8"), marker);
}

let passed = false;
try {
  const packJson = JSON.parse(runNpm(["pack", "--json", "--pack-destination", artifactDir], packageRoot));
  assert.equal(packJson.length, 1, "npm pack must create exactly one tarball");
  const packed = packJson[0];
  const tarball = path.join(artifactDir, packed.filename);
  const tarballBytes = readFileSync(tarball);
  const listed = new Set(packed.files.map(({ path: file }) => file.replaceAll("\\", "/")));
  const required = [
    "package.json",
    "README.md",
    "CHANGELOG.md",
    "LICENSE",
    "required_capabilities.json",
    "testdata/microbenchmark-pairs-v1.json",
    "dist/index.js",
    "dist/index.d.ts",
    "dist/index.js.map",
    "dist/index.d.ts.map",
  ];
  for (const file of required) assert.ok(listed.has(file), `package tarball is missing ${file}`);
  for (const file of listed) {
    assert.equal(/^(tests|tools|node_modules|coverage|dist-test)\//.test(file), false, `test/tool output leaked into tarball: ${file}`);
    assert.equal(file.includes("package-lock.json"), false, "package lock must not be shipped");
  }
  const sourceFiles = [...listed].filter((file) => file.startsWith("src/") && file.endsWith(".ts")).sort();
  assert.ok(sourceFiles.length > 0, "production TypeScript sources must ship with the maps");
  const manifest = JSON.parse(readFileSync(path.join(packageRoot, "package.json"), "utf8"));
  assert.equal(manifest.name, "@coding-adventures/rank-statistics");
  assert.equal(manifest.version, "0.1.0");
  assert.equal(manifest.type, "module");

  writeFileSync(path.join(consumerDir, "package.json"), JSON.stringify({ private: true, type: "module" }, null, 2));
  runNpm(["install", "--offline", "--ignore-scripts", "--no-audit", "--no-fund", "--prefix", consumerDir, tarball], packageRoot);

  const installedRoot = realpathSync(path.join(consumerDir, "node_modules", "@coding-adventures", "rank-statistics"));
  const expectedInstalledRoot = realpathSync(path.join(consumerDir, "node_modules", "@coding-adventures", "rank-statistics"));
  assert.equal(installedRoot, expectedInstalledRoot);
  assert.equal(isWithin(consumerDir, installedRoot), true, "resolved package must stay inside the external consumer");
  assert.equal(isWithin(packageRoot, installedRoot), false, "installed package must not resolve into the source tree");
  assert.equal(isWithin(packageRoot, consumerDir), false, "consumer directory must be outside the package source tree");
  const installedManifest = JSON.parse(readFileSync(path.join(installedRoot, "package.json"), "utf8"));
  assert.equal(installedManifest.name, manifest.name);
  assert.equal(installedManifest.version, manifest.version);

  const mapTargets = {};
  for (const mapFile of ["dist/index.js.map", "dist/index.d.ts.map"]) {
    const mapPath = path.join(installedRoot, mapFile);
    const map = JSON.parse(readFileSync(mapPath, "utf8"));
    assert.ok(Array.isArray(map.sources) && map.sources.length > 0, `${mapFile} must name source files`);
    const targets = map.sources.map((source) => path.resolve(path.dirname(mapPath), source));
    for (const target of targets) {
      assert.equal(isWithin(installedRoot, target), true, `${mapFile} target escapes the installed package`);
      const packageRelative = path.relative(installedRoot, target).replaceAll("\\", "/");
      assert.ok(listed.has(packageRelative), `${mapFile} references unpacked source ${packageRelative}`);
      assert.equal(existsSync(target), true, `${mapFile} source is not installed: ${packageRelative}`);
    }
    mapTargets[mapFile] = targets.map((target) => path.relative(installedRoot, target).replaceAll("\\", "/"));
  }
  for (const file of ["LICENSE", "testdata/microbenchmark-pairs-v1.json"]) {
    const sourceBytes = readFileSync(path.join(packageRoot, file));
    const installedBytes = readFileSync(path.join(installedRoot, file));
    assert.equal(sha256(installedBytes), sha256(sourceBytes), `${file} differs from the package source`);
  }

  const runtimePath = path.join(consumerDir, "runtime-smoke.mjs");
  writeFileSync(runtimePath, `import assert from "node:assert/strict";\nimport { median, medianInterval } from "@coding-adventures/rank-statistics";\nassert.equal(median([1, 2, 3]), 2);\nassert.deepEqual(medianInterval([0.97, 1.01, 1.04, 0.99, 1.02], 70), { low: 0.97, high: 1.04 });\nassert.equal(medianInterval([1, 2, 3], 99), null);\nconst resolved = await import.meta.resolve("@coding-adventures/rank-statistics");\nassert.ok(resolved.startsWith(new URL("./node_modules/@coding-adventures/rank-statistics/", import.meta.url).href));\nconsole.log(JSON.stringify({ resolved }));\n`);
  const runtimeOutput = run(process.execPath, [runtimePath], consumerDir).trim();
  const runtimeResult = JSON.parse(runtimeOutput);
  assert.ok(runtimeResult.resolved.includes("node_modules/@coding-adventures/rank-statistics/dist/index.js"));

  writeFileSync(path.join(consumerDir, "consumer.ts"), `import { median, medianInterval } from "@coding-adventures/rank-statistics";\nconst center: number = median([1, 2, 3]);\nconst interval: { readonly low: number; readonly high: number } | null = medianInterval([1, 2, 3], 50);\nvoid center;\nvoid interval;\n`);
  writeFileSync(path.join(consumerDir, "tsconfig.json"), JSON.stringify({
    compilerOptions: {
      target: "ES2022",
      module: "NodeNext",
      moduleResolution: "NodeNext",
      strict: true,
      noEmit: true,
      types: [],
    },
    files: ["consumer.ts"],
  }, null, 2));
  const tsc = path.join(packageRoot, "node_modules", "typescript", "bin", "tsc");
  run(process.execPath, [tsc, "-p", path.join(consumerDir, "tsconfig.json")], consumerDir);

  passed = true;
  console.log(JSON.stringify({
    status: "PASS",
    node: process.version,
    npm: runNpm(["--version"], packageRoot).trim(),
    package: manifest.name,
    version: manifest.version,
    tarball: path.basename(tarball),
    tarballSha256: sha256(tarballBytes),
    installedRoot,
    packageFileCount: listed.size,
    productionTypeScriptSources: sourceFiles,
    sourceMapTargets: mapTargets,
    licenseSha256: sha256(readFileSync(path.join(installedRoot, "LICENSE"))),
    fixtureSha256: sha256(readFileSync(path.join(installedRoot, "testdata/microbenchmark-pairs-v1.json"))),
    runtimeResolution: runtimeResult.resolved,
    strictNodeNextConsumer: "PASS",
    offlineInstall: "PASS",
  }, null, 2));
} finally {
  if (passed && process.env.CA_RANK_STATISTICS_SMOKE_KEEP !== "1") {
    assertSafeRemovalTarget(tempRoot);
    rmSync(tempRoot, { recursive: true, force: false });
  } else {
    console.error(`Installed smoke artifacts retained at ${tempRoot}`);
  }
}
