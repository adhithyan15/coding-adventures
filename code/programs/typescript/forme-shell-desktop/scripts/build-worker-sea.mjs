import { createHash } from "node:crypto";
import { copyFile, chmod, readFile, rename, rm, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const workerDirectory = join(root, "dist", "worker");
const bundle = join(workerDirectory, "forme-product-worker.cjs");
const blob = join(workerDirectory, "forme-product-worker.blob");
const configuration = join(workerDirectory, "sea-config.json");
const suffix = process.platform === "win32" ? ".exe" : "";
const output = join(workerDirectory, `forme-product-worker${suffix}`);
const temporary = `${output}.new`;
const sandboxLauncher = join(workerDirectory, "forme-sandbox-macos");
const sandboxPackage = resolve(root, "../../../packages/typescript/forme-sandbox-macos");
const postject = join(root, "node_modules", ".bin", process.platform === "win32" ? "postject.cmd" : "postject");

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === resolve(process.argv[1])) {
  await buildWorkerSea();
}

async function buildWorkerSea() {
  const seaNode = await selectSeaNode();
  await writeFile(configuration, `${JSON.stringify({
    main: bundle,
    output: blob,
    disableExperimentalSEAWarning: true,
  }, null, 2)}\n`, { encoding: "utf8", mode: 0o600 });
  run(seaNode, ["--experimental-sea-config", configuration]);
  await rm(temporary, { force: true });
  if (process.platform === "darwin") {
    const plan = planMacosSeaCopy(process.arch, run("lipo", [seaNode, "-archs"]));
    if (plan.kind === "thin") {
      run("lipo", [seaNode, "-thin", plan.architecture, "-output", temporary]);
    } else {
      await copyFile(seaNode, temporary);
    }
  } else {
    await copyFile(seaNode, temporary);
  }
  if (process.platform !== "win32") await chmod(temporary, 0o700);
  if (process.platform === "darwin") run("codesign", ["--remove-signature", temporary]);
  const injection = [temporary, "NODE_SEA_BLOB", blob,
    "--sentinel-fuse", "NODE_SEA_FUSE_fce680ab2cc467b6e072b8b5df1996b2"];
  if (process.platform === "darwin") injection.push("--macho-segment-name", "NODE_SEA");
  run(postject, injection);
  if (process.platform === "darwin") run("codesign", ["--sign", "-", temporary]);
  await rename(temporary, output);
  const digest = createHash("sha256").update(await readFile(output)).digest("hex");
  await writeFile(`${output}.sha256`, `${digest}\n`, { encoding: "utf8", mode: 0o600 });
  if (process.platform === "darwin") {
    const compiledLauncher = join(sandboxPackage, "native/forme-sandbox-macos");
    const compiledProbe = join(sandboxPackage, "native/forme-sandbox-probe");
    try {
      run(process.execPath, [join(sandboxPackage, "scripts/build-native.mjs")]);
      await copyFile(compiledLauncher, sandboxLauncher);
      await chmod(sandboxLauncher, 0o700);
      const launcherDigest = createHash("sha256")
        .update(await readFile(sandboxLauncher))
        .digest("hex");
      await writeFile(`${sandboxLauncher}.sha256`, `${launcherDigest}\n`, {
        encoding: "utf8",
        mode: 0o600,
      });
    } finally {
      await Promise.all([
        rm(compiledLauncher, { force: true }),
        rm(compiledProbe, { force: true }),
      ]);
    }
  }
  await rm(blob, { force: true });
  await rm(configuration, { force: true });
}

function run(command, arguments_) {
  const result = spawnSync(command, arguments_, { cwd: root, encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(`${command} failed: ${result.stderr || result.stdout || `status ${result.status}`}`);
  }
  return result.stdout;
}

export function planMacosSeaCopy(processArchitecture, lipoArchitectures) {
  const architecture = processArchitecture === "x64" ? "x86_64" : processArchitecture;
  const available = lipoArchitectures.trim().split(/\s+/).filter(Boolean);
  if (!available.includes(architecture)) {
    throw new Error(`selected Node.js executable does not contain architecture ${architecture}`);
  }
  return available.length === 1 ? { kind: "copy" } : { kind: "thin", architecture };
}

async function selectSeaNode() {
  const candidates = [process.env.FORME_SEA_NODE, process.execPath];
  if (process.platform === "darwin") candidates.push("/usr/local/bin/node");
  const sentinel = Buffer.from("NODE_SEA_FUSE_fce680ab2cc467b6e072b8b5df1996b2:0");
  for (const candidate of candidates) {
    if (candidate === undefined || candidate.length === 0) continue;
    try {
      if ((await readFile(candidate)).includes(sentinel)) return candidate;
    } catch {
      // Try the next explicit candidate.
    }
  }
  throw new Error("no injectible Node.js SEA executable is available; set FORME_SEA_NODE");
}
