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
  symlinkSync,
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
  // macOS may spell the same temporary directory as /var/... or
  // /private/var/...; compare canonical paths rather than those aliases.
  const realParent = realpathSync(parent);
  const realCandidate = realpathSync(candidate);
  const relative = path.relative(realParent, realCandidate);
  return relative === "" || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative));
}

function assertSafeRemovalTarget(target) {
  const lexicalTarget = path.resolve(target);
  assert.equal(path.basename(lexicalTarget).startsWith("ca-rank-statistics-smoke-"), true);
  const stat = lstatSync(lexicalTarget);
  assert.equal(stat.isDirectory(), true);
  assert.equal(stat.isSymbolicLink(), false);
  assert.equal(isWithin(tempParent, lexicalTarget), true);
  assert.equal(readFileSync(path.join(lexicalTarget, ".owned-smoke-root"), "utf8"), marker);
}

function createDirectoryAlias(target, alias) {
  symlinkSync(target, alias, process.platform === "win32" ? "junction" : "dir");
}

function removeOwnedDirectoryAlias(alias, expectedTarget) {
  const lexicalAlias = path.resolve(alias);
  const relative = path.relative(path.resolve(tempRoot), lexicalAlias);
  assert.equal(relative === "" || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative)), true);
  assert.equal(lstatSync(lexicalAlias).isSymbolicLink(), true);
  assert.equal(realpathSync(lexicalAlias), realpathSync(expectedTarget));
  rmSync(lexicalAlias, { force: false });
  assert.equal(existsSync(lexicalAlias), false);
  assert.equal(existsSync(expectedTarget), true, "removing the alias must not remove its target");
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

  const realPackageRoot = realpathSync(packageRoot);
  const realConsumerDir = realpathSync(consumerDir);
  const expectedInstalledPath = path.join(realConsumerDir, "node_modules", "@coding-adventures", "rank-statistics");
  const expectedInstalledRoot = realpathSync(expectedInstalledPath);
  const installedRoot = realpathSync(path.join(consumerDir, "node_modules", "@coding-adventures", "rank-statistics"));
  assert.equal(installedRoot, expectedInstalledRoot, "installed package must resolve at the canonical consumer location");
  assert.equal(isWithin(realConsumerDir, installedRoot), true, "resolved package must stay inside the external consumer");
  assert.equal(isWithin(realPackageRoot, installedRoot), false, "installed package must not resolve into the source tree");
  assert.equal(isWithin(realPackageRoot, realConsumerDir), false, "consumer directory must be outside the package source tree");
  assert.equal(isWithin(realConsumerDir, artifactDir), false, "the sibling artifact directory is outside the consumer");

  // Exercise real directory aliases and ensure cleanup unlinks only the alias.
  const consumerAlias = path.join(tempRoot, "consumer-alias");
  createDirectoryAlias(consumerDir, consumerAlias);
  assert.equal(realpathSync(consumerAlias), realConsumerDir);
  assert.equal(isWithin(consumerAlias, installedRoot), true, "consumer aliases must canonicalize to their target");
  assert.equal(isWithin(consumerAlias, artifactDir), false, "an outside sibling must not be treated as consumer content");
  removeOwnedDirectoryAlias(consumerAlias, consumerDir);

  const sourceAlias = path.join(tempRoot, "source-alias");
  createDirectoryAlias(packageRoot, sourceAlias);
  assert.equal(realpathSync(sourceAlias), realPackageRoot);
  assert.equal(isWithin(realConsumerDir, sourceAlias), false, "a source-tree alias must remain outside the consumer");
  assert.equal(isWithin(sourceAlias, installedRoot), false, "the installed package must remain outside source aliases");
  removeOwnedDirectoryAlias(sourceAlias, packageRoot);
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
      assert.equal(existsSync(target), true, `${mapFile} source is not installed`);
      const realTarget = realpathSync(target);
      assert.equal(isWithin(installedRoot, realTarget), true, `${mapFile} target escapes the installed package`);
      const packageRelative = path.relative(installedRoot, realTarget).replaceAll("\\", "/");
      assert.ok(listed.has(packageRelative), `${mapFile} references unpacked source ${packageRelative}`);
      assert.equal(existsSync(path.join(installedRoot, packageRelative)), true, `${mapFile} source is not installed: ${packageRelative}`);
    }
    mapTargets[mapFile] = targets.map((target) => path.relative(installedRoot, realpathSync(target)).replaceAll("\\", "/"));
  }
  for (const file of ["LICENSE", "testdata/microbenchmark-pairs-v1.json"]) {
    const sourceBytes = readFileSync(path.join(packageRoot, file));
    const installedBytes = readFileSync(path.join(installedRoot, file));
    assert.equal(sha256(installedBytes), sha256(sourceBytes), `${file} differs from the package source`);
  }

  const runtimePath = path.join(consumerDir, "runtime-smoke.mjs");
  writeFileSync(runtimePath, `import assert from "node:assert/strict";\nimport { median, medianInterval } from "@coding-adventures/rank-statistics";\nassert.equal(median([1, 2, 3]), 2);\nassert.deepEqual(medianInterval([0.97, 1.01, 1.04, 0.99, 1.02], 70), { low: 0.97, high: 1.04 });\nassert.equal(medianInterval([1, 2, 3], 99), null);\nconst resolved = await import.meta.resolve("@coding-adventures/rank-statistics");\nconsole.log(JSON.stringify({ resolved }));\n`);
  const runtimeOutput = run(process.execPath, [runtimePath], consumerDir).trim();
  const runtimeResult = JSON.parse(runtimeOutput);
  const resolvedEntry = realpathSync(fileURLToPath(runtimeResult.resolved));
  const expectedInstalledEntry = realpathSync(path.join(expectedInstalledRoot, "dist", "index.js"));
  assert.equal(resolvedEntry, expectedInstalledEntry, "runtime import must resolve to the canonical installed entry");
  assert.equal(isWithin(realConsumerDir, resolvedEntry), true, "runtime entry must remain inside the external consumer");
  assert.equal(isWithin(realPackageRoot, resolvedEntry), false, "runtime entry must not resolve into the source tree");

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
    pathContainmentChecks: {
      directoryAlias: process.platform === "win32" ? "junction PASS" : "symlink PASS",
      outsideConsumerSibling: "rejected",
      sourceTreeAlias: "rejected",
      aliasCleanupDidNotRemoveTargets: "PASS",
    },
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
