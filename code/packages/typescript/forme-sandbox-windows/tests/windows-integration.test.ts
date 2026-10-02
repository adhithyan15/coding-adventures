import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { copyFile, mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { dirname, isAbsolute, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { afterAll, afterEach, describe, expect, it } from "vitest";
import { computeManifestHash, type Manifest } from "@coding-adventures/forme-manifest";
import type { SandboxLaunchRequest } from "@coding-adventures/forme-sandbox-core";
import { createWindowsInstallAclVerifier, createWindowsSandboxFactory } from "../src/index.js";

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const launcherPath = join(packageRoot, "native", "forme-sandbox-windows.exe");
const probePath = join(packageRoot, "native", "forme-sandbox-probe.exe");
const roots: string[] = [];
const suiteRoots: string[] = [];
const execFileAsync = promisify(execFile);
afterEach(async () => Promise.all(roots.splice(0).map(root => rm(root, {
  recursive: true,
  force: true,
  maxRetries: 5,
  retryDelay: 100,
}))));
afterAll(async () => Promise.all(suiteRoots.splice(0).map(root => rm(root, {
  recursive: true,
  force: true,
  maxRetries: 5,
  retryDelay: 100,
}))));

let trustedPythonPromise: Promise<{ readonly executable: string; readonly root: string }> | undefined;

function escapedDiagnosticPath(path: string): string {
  return path.split("\\").join("\\\\");
}

async function trustedPythonDistribution(): Promise<{ readonly executable: string; readonly root: string }> {
  trustedPythonPromise ??= (async () => {
    const { stdout } = await execFileAsync("python", [
      "-c",
      "import os,sys; print(os.path.realpath(sys.executable)); print(os.path.realpath(sys.prefix))",
    ]);
    const [sourceExecutable, sourceRoot] = stdout.trim().split(/\r?\n/);
    if (!sourceExecutable || !sourceRoot) {
      throw new Error("Python runtime discovery returned an incomplete distribution");
    }
    const executableRelativePath = relative(sourceRoot, sourceExecutable);
    if (executableRelativePath.startsWith("..") || isAbsolute(executableRelativePath)) {
      throw new Error("Python executable is outside its reported distribution root");
    }
    if (sourceRoot.toLowerCase().includes("\\hostedtoolcache\\")) {
      const sourceVerification = await verifyRuntimeRoot(sourceRoot);
      expect(sourceVerification.accepted).toBe(false);
      expect(sourceVerification.stderr).toContain("untrusted writer");
      expect(sourceVerification.stderr).toContain("sid=S-1-5-11");
      expect(sourceVerification.stderr).toContain(escapedDiagnosticPath(sourceRoot));
    }

    // The hosted toolcache grants Authenticated Users write authority and is
    // therefore correctly rejected by the production trust verifier.  Copy
    // its runner-controlled bytes into this user-owned tree so the acceptance
    // test exercises a genuinely trusted distribution without weakening or
    // mutating the shared toolcache ACL.
    const root = await mkdtemp(join(homedir(), "forme-windows-python-runtime-"));
    suiteRoots.push(root);
    await execFileAsync(sourceExecutable, [
      "-c",
      String.raw`import os, shutil, sys
source, destination = sys.argv[1:]
source_lib = os.path.normcase(os.path.abspath(os.path.join(source, "Lib")))
is_junction = getattr(os.path, "isjunction", lambda _path: False)
def ignored(directory, names):
    rejected = {name for name in names if os.path.islink(os.path.join(directory, name)) or is_junction(os.path.join(directory, name))}
    rejected.update(name for name in names if name == "__pycache__")
    if os.path.normcase(os.path.abspath(directory)) == source_lib:
        rejected.update(name for name in names if name == "site-packages")
    return rejected
shutil.copytree(source, destination, dirs_exist_ok=True, symlinks=True, ignore=ignored)
`,
      sourceRoot,
      root,
    ], { timeout: 60_000, maxBuffer: 16_384 });
    const destinationVerification = await verifyRuntimeRoot(root);
    expect(destinationVerification.accepted, destinationVerification.stderr).toBe(true);
    return { executable: join(root, executableRelativePath), root };
  })();
  return trustedPythonPromise;
}

async function request(probe: string): Promise<SandboxLaunchRequest> {
  const workingDirectory = await mkdtemp(join(tmpdir(), "forme-windows-integration-"));
  roots.push(workingDirectory);
  const entryBytes = new Uint8Array(await readFile(probePath));
  const manifest: Manifest = {
    manifestVersion: 1,
    plugin: { name: "@example/windows-probe", version: "1.0.0", apiVersion: 1 },
    runtime: { kind: "binary", entry: "probe.exe" },
    capabilities: { required: [], optional: [] },
    contributes: { stages: [{ id: probe, consumes: "ContentSource", produces: "ContentNode" }], kinds: [] },
    resources: { maxMemoryMb: 64, maxWallClockMs: 2_000, maxFileDescriptors: 256 },
  };
  const schemaBytes = new TextEncoder().encode('{"type":"object"}\n');
  return {
    plugin: { manifest, manifestHash: computeManifestHash(manifest, entryBytes), entryBytes },
    stage: manifest.contributes.stages[0]!,
    instanceId: `probe/${probe}`,
    workingDirectory,
    resources: manifest.resources,
    configSchema: probe === "snapshot" ? {
      relativePath: "plugin-config-schema.json",
      bytes: schemaBytes,
      hash: `sha256:${createHash("sha256").update(schemaBytes).digest("hex")}`,
    } : null,
  };
}

async function nodeRequest(source = String.raw`let input = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", chunk => { input += chunk; });
process.stdin.on("end", () => process.stdout.write(input, () => process.exit(0)));
`): Promise<SandboxLaunchRequest> {
  const workingDirectory = await mkdtemp(join(tmpdir(), "forme-windows-node-"));
  roots.push(workingDirectory);
  const entryBytes = new TextEncoder().encode(source);
  const manifest: Manifest = {
    manifestVersion: 1,
    plugin: { name: "@example/windows-node", version: "1.0.0", apiVersion: 1 },
    runtime: { kind: "node", entry: "entry.mjs" },
    capabilities: { required: [], optional: [] },
    contributes: { stages: [{ id: "main", consumes: "ContentSource", produces: "ContentNode" }], kinds: [] },
    resources: { maxMemoryMb: 64, maxWallClockMs: 2_000, maxFileDescriptors: 256 },
  };
  return {
    plugin: { manifest, manifestHash: computeManifestHash(manifest, entryBytes), entryBytes },
    stage: manifest.contributes.stages[0]!, instanceId: "node/main", workingDirectory,
    resources: manifest.resources, configSchema: null,
  };
}

async function pythonRequest(): Promise<{
  readonly request: SandboxLaunchRequest;
  readonly executable: string;
  readonly root: string;
}> {
  const { executable, root } = await trustedPythonDistribution();
  const workingDirectory = await mkdtemp(join(tmpdir(), "forme-windows-python-"));
  roots.push(workingDirectory);
  const entryBytes = new TextEncoder().encode(String.raw`import sys
data = sys.stdin.buffer.read()
sys.stdout.buffer.write(data)
sys.stdout.buffer.flush()
`);
  const manifest: Manifest = {
    manifestVersion: 1,
    plugin: { name: "@example/windows-python", version: "1.0.0", apiVersion: 1 },
    runtime: { kind: "python", entry: "entry.py" },
    capabilities: { required: [], optional: [] },
    contributes: { stages: [{ id: "main", consumes: "ContentSource", produces: "ContentNode" }], kinds: [] },
    resources: { maxMemoryMb: 64, maxWallClockMs: 5_000, maxFileDescriptors: 256 },
  };
  return {
    executable,
    root,
    request: {
      plugin: { manifest, manifestHash: computeManifestHash(manifest, entryBytes), entryBytes },
      stage: manifest.contributes.stages[0]!, instanceId: "python/main", workingDirectory,
      resources: manifest.resources, configSchema: null,
    },
  };
}

async function exitWithStderr(child: Awaited<ReturnType<ReturnType<typeof createWindowsSandboxFactory>["launch"]>>): Promise<{
  readonly exit: Awaited<typeof child.exited>;
  readonly stderr: string;
}> {
  const chunks: Buffer[] = [];
  child.stderr.on("data", chunk => chunks.push(Buffer.from(chunk)));
  const exit = await child.exited;
  return { exit, stderr: Buffer.concat(chunks).toString("utf8") };
}

async function verifyRuntimeRoot(path: string): Promise<{
  readonly accepted: boolean;
  readonly stderr: string;
}> {
  return new Promise(resolveResult => {
    execFile(launcherPath, [`--verify-runtime-root=${path}`], {
      windowsHide: true,
      timeout: 5_000,
      maxBuffer: 16_384,
    }, (error, _stdout, stderr) => resolveResult({
      accepted: error === null,
      stderr,
    }));
  });
}

describe.skipIf(process.platform !== "win32")("Windows native sandbox", () => {
  it("accepts an owned tree and rejects reparse points or untrusted writers", async () => {
    // Hosted-runner temp roots are intentionally shared and therefore fail
    // the verifier's ancestor replacement-authority check. The user profile
    // gives this positive fixture the same trusted ancestry required in use.
    const root = await mkdtemp(join(homedir(), "forme-windows-acl-"));
    roots.push(root);
    await mkdir(join(root, "plugin"));
    await writeFile(join(root, "plugin", "entry.mjs"), "process.exit(0);\n");
    const verify = createWindowsInstallAclVerifier({ launcherPath });
    expect(await verify(root, "install-root")).toBe(true);
    expect(await verify(join(root, "plugin"), "existing-target-tree")).toBe(true);

    const transactionRoot = join(root, "transaction-root");
    await mkdir(transactionRoot);
    await execFileAsync("icacls.exe", [transactionRoot, "/grant", "*S-1-1-0:(OI)(CI)(IO)M", "/Q"]);
    expect(await verify(transactionRoot, "install-root")).toBe(false);
    expect(await verify(transactionRoot, "existing-target-tree")).toBe(true);
    await execFileAsync("icacls.exe", [transactionRoot, "/remove:g", "*S-1-1-0", "/Q"]);
    expect(await verify(transactionRoot, "install-root")).toBe(true);

    await execFileAsync("icacls.exe", [root, "/grant", "*S-1-1-0:(NP)M", "/Q"]);
    expect(await verify(join(root, "plugin"), "install-root")).toBe(false);
    await execFileAsync("icacls.exe", [root, "/remove:g", "*S-1-1-0", "/Q"]);

    const outside = await mkdtemp(join(tmpdir(), "forme-windows-acl-outside-"));
    roots.push(outside);
    await symlink(outside, join(root, "plugin", "link"), "junction");
    expect(await verify(join(root, "plugin"), "existing-target-tree")).toBe(false);
    await rm(join(root, "plugin", "link"), { force: true });

    await execFileAsync("icacls.exe", [root, "/grant", "*S-1-1-0:(OI)(CI)M", "/T", "/Q"]);
    expect(await verify(root, "existing-target-tree")).toBe(false);
  });

  it("round-trips protocol bytes through the trusted Node runtime", async () => {
    const child = await createWindowsSandboxFactory({ launcherPath, readinessTimeoutMs: 2_000 })
      .launch(await nodeRequest());
    const stdout: Buffer[] = [];
    child.stdout.on("data", chunk => stdout.push(Buffer.from(chunk)));
    child.stdin.end("sandbox protocol probe");
    const result = await exitWithStderr(child);
    expect(result.exit, result.stderr).toEqual({ code: 0, signal: null });
    expect(Buffer.concat(stdout).toString("utf8")).toBe("sandbox protocol probe");
  });

  it("accepts only contained runtime-root reparse targets", async () => {
    const root = await mkdtemp(join(homedir(), "forme-windows-runtime-root-"));
    roots.push(root);
    const contained = join(root, "contained");
    await mkdir(contained);
    const alias = join(root, "alias");
    await symlink(contained, alias, "junction");
    let verification = await verifyRuntimeRoot(root);
    expect(verification.accepted, verification.stderr).toBe(true);
    await rm(alias, { force: true });

    await execFileAsync("icacls.exe", [root, "/grant", "*S-1-1-0:(OI)(CI)(IO)M", "/Q"]);
    verification = await verifyRuntimeRoot(root);
    expect(verification.accepted).toBe(false);
    expect(verification.stderr).toContain("trust verification rejected");
    expect(verification.stderr).toContain("untrusted writer");
    expect(verification.stderr).toContain(escapedDiagnosticPath(root));
    await execFileAsync("icacls.exe", [root, "/remove:g", "*S-1-1-0", "/Q"]);
    verification = await verifyRuntimeRoot(root);
    expect(verification.accepted, verification.stderr).toBe(true);

    const outside = await mkdtemp(join(homedir(), "forme-windows-runtime-outside-"));
    roots.push(outside);
    await symlink(outside, alias, "junction");
    verification = await verifyRuntimeRoot(root);
    expect(verification.accepted).toBe(false);
    expect(verification.stderr).toContain("trust verification rejected");
    expect(verification.stderr).toContain("reparse target escapes the runtime root");
    expect(verification.stderr).toContain(escapedDiagnosticPath(alias));
    await rm(alias, { force: true });

    verification = await verifyRuntimeRoot(join(root, "missing"));
    expect(verification.accepted).toBe(false);
    expect(verification.stderr).toContain("[win32=2]");

    const hostileDiagnosticPath = `${root}\n\u001b\u202e`;
    verification = await verifyRuntimeRoot(hostileDiagnosticPath);
    expect(verification.accepted).toBe(false);
    expect(verification.stderr).toContain("\\u000A");
    expect(verification.stderr).toContain("\\u001B");
    expect(verification.stderr).toContain("\\u202E");
    expect(verification.stderr).not.toContain("\u001b");
    expect(verification.stderr).not.toContain("\u202e");
    expect(verification.stderr.match(/\r?\n/g)).toHaveLength(1);
  });

  it("round-trips protocol bytes through a trusted Python distribution", async () => {
    const python = await pythonRequest();
    const verification = await verifyRuntimeRoot(python.root);
    expect(verification.accepted, verification.stderr).toBe(true);
    const child = await createWindowsSandboxFactory({
      launcherPath,
      readinessTimeoutMs: 5_000,
      runtimeExecutables: { python: python.executable },
      runtimeRoots: { python: python.root },
    }).launch(python.request);
    const stdout: Buffer[] = [];
    child.stdout.on("data", chunk => stdout.push(Buffer.from(chunk)));
    child.stdin.end("sandbox python protocol probe");
    const result = await exitWithStderr(child);
    expect(result.exit, result.stderr).toEqual({ code: 0, signal: null });
    expect(Buffer.concat(stdout).toString("utf8")).toBe("sandbox python protocol probe");
  }, 75_000);

  it("does not grant Node access to sibling runtime-root files", async () => {
    const runtimeRoot = await mkdtemp(join(homedir(), "forme-windows-node-runtime-"));
    roots.push(runtimeRoot);
    const runtime = join(runtimeRoot, "node.exe");
    const secret = join(runtimeRoot, "must-stay-private.txt");
    await copyFile(process.execPath, runtime);
    await writeFile(secret, "private runtime sibling");
    const child = await createWindowsSandboxFactory({
      launcherPath,
      readinessTimeoutMs: 5_000,
      runtimeExecutables: { node: runtime },
      runtimeRoots: { node: runtimeRoot },
    }).launch(await nodeRequest(String.raw`import { readFileSync } from "node:fs";
try {
  readFileSync(${JSON.stringify(secret)});
  process.exit(91);
} catch (error) {
  process.exit(error?.code === "EACCES" || error?.code === "EPERM" ? 0 : 92);
}
`));
    const result = await exitWithStderr(child);
    expect(result.exit, result.stderr).toEqual({ code: 0, signal: null });
  }, 20_000);

  it("serializes overlapping Python runtime-root grants and revocations", async () => {
    const [firstRequest, secondRequest] = await Promise.all([pythonRequest(), pythonRequest()]);
    expect(secondRequest.root.toLowerCase()).toBe(firstRequest.root.toLowerCase());
    const factory = createWindowsSandboxFactory({
      launcherPath,
      readinessTimeoutMs: 5_000,
      runtimeExecutables: { python: firstRequest.executable },
      runtimeRoots: { python: firstRequest.root },
    });
    const [first, second] = await Promise.all([
      factory.launch(firstRequest.request),
      factory.launch(secondRequest.request),
    ]);
    first.stdin.end();
    second.stdin.end();
    const results = await Promise.all([exitWithStderr(first), exitWithStderr(second)]);
    expect(results.map(result => result.exit), results.map(result => result.stderr).join("\n")).toEqual([
      { code: 0, signal: null },
      { code: 0, signal: null },
    ]);
  }, 30_000);

  it("serializes overlapping trusted-runtime ACL grants and revocations", async () => {
    const [first, second] = await Promise.all([
      createWindowsSandboxFactory({ launcherPath, readinessTimeoutMs: 2_000 }).launch(await nodeRequest()),
      createWindowsSandboxFactory({ launcherPath, readinessTimeoutMs: 2_000 }).launch(await nodeRequest()),
    ]);
    first.stdin.end();
    second.stdin.end();
    const results = await Promise.all([exitWithStderr(first), exitWithStderr(second)]);
    expect(results.map(result => result.exit), results.map(result => result.stderr).join("\n")).toEqual([
      { code: 0, signal: null },
      { code: 0, signal: null },
    ]);
  }, 15_000);

  it.each(["filesystem", "network", "process", "snapshot", "memory", "descriptors", "environment", "cpu", "wall-clock"])(
    "blocks unauthorized %s access",
    async probe => {
      process.env.FORME_SANDBOX_AMBIENT_SENTINEL = "must-not-cross";
      try {
        const child = await createWindowsSandboxFactory({ launcherPath, readinessTimeoutMs: 2_000 })
          .launch(await request(probe));
        const exit = await child.exited;
        if (["descriptors", "cpu", "wall-clock"].includes(probe)) expect(exit.code).not.toBe(0);
        else expect(exit).toEqual({ code: 0, signal: null });
      } finally {
        delete process.env.FORME_SANDBOX_AMBIENT_SENTINEL;
      }
    },
  );
});
