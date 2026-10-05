import { execFile } from "node:child_process";
import { cp, mkdir, mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { afterAll, describe, expect, it, vi } from "vitest";
import type { Orchestrator } from "@coding-adventures/forme-orchestrator";
import type { PluginHost, PluginHostOptions, PluginProcessFactory } from "@coding-adventures/forme-plugin-host";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import { discoverPlugins, formatGrantsFile } from "@coding-adventures/forme-plugin-host";
import {
  createProductOrchestrator,
  type ProductRuntimeDependencies,
} from "../src/runtime.js";

const PROJECT_ROOT = "/project";
const CACHE_ROOT = join(PROJECT_ROOT, ".forme", "cache");
const pluginFixture = fileURLToPath(new URL("../../forme-plugin-host/tests/fixtures/echo-plugin", import.meta.url));
const windowsLauncherPath = fileURLToPath(new URL(
  "../../forme-sandbox-windows/native/forme-sandbox-windows.exe",
  import.meta.url,
));
const execFileAsync = promisify(execFile);
const suiteRoots: string[] = [];

afterAll(async () => Promise.all(suiteRoots.splice(0).map(root => rm(root, {
  recursive: true,
  force: true,
  maxRetries: 5,
  retryDelay: 100,
}))));

let pythonDistributionPromise: Promise<{ readonly executable: string; readonly root: string }> | undefined;

function escapedDiagnosticPath(path: string): string {
  return path.split("\\").join("\\\\");
}

async function verifyWindowsRuntimeRoot(path: string): Promise<{
  readonly accepted: boolean;
  readonly stderr: string;
}> {
  return new Promise(resolveVerification => {
    execFile(windowsLauncherPath, [`--verify-runtime-root=${path}`], {
      windowsHide: true,
    }, (error, _stdout, stderr) => resolveVerification({
      accepted: error === null,
      stderr,
    }));
  });
}

async function pythonDistribution(): Promise<{ readonly executable: string; readonly root: string }> {
  pythonDistributionPromise ??= (async () => {
    // setup-python prepends a separately packaged CPython to PATH whenever this
    // repository's Python packages are in the CI change set. The Linux native
    // sandbox acceptance fixture intentionally exercises the distro runtime
    // whose complete system-library closure the launcher exposes.
    const discoveryExecutable = process.platform === "win32"
      ? "python"
      : process.platform === "linux" ? "/usr/bin/python3" : "python3";
    const { stdout } = await execFileAsync(discoveryExecutable, [
      "-c",
      "import os,sys; app=os.path.join(sys.prefix,'Resources','Python.app','Contents','MacOS','Python'); print(os.path.realpath(app if os.path.isfile(app) else sys.executable)); print(os.path.realpath(sys.prefix))",
    ]);
    const [sourceExecutable, sourceRoot] = stdout.trim().split(/\r?\n/);
    if (!sourceExecutable || !sourceRoot) {
      throw new Error("Python runtime discovery returned an incomplete distribution");
    }
    if (process.platform !== "win32") {
      return { executable: sourceExecutable, root: sourceRoot };
    }

    const executableRelativePath = relative(sourceRoot, sourceExecutable);
    if (executableRelativePath.startsWith("..") || isAbsolute(executableRelativePath)) {
      throw new Error("Python executable is outside its reported distribution root");
    }
    if (sourceRoot.toLowerCase().includes("\\hostedtoolcache\\")) {
      const sourceVerification = await verifyWindowsRuntimeRoot(sourceRoot);
      expect(sourceVerification.accepted).toBe(false);
      expect(sourceVerification.stderr).toContain("untrusted writer");
      expect(sourceVerification.stderr).toContain("sid=S-1-5-11");
      expect(sourceVerification.stderr).toContain(escapedDiagnosticPath(sourceRoot));
    }

    const root = await mkdtemp(join(homedir(), "forme-cli-python-runtime-"));
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
    const destinationVerification = await verifyWindowsRuntimeRoot(root);
    expect(destinationVerification.accepted, destinationVerification.stderr).toBe(true);
    return { executable: join(root, executableRelativePath), root };
  })();
  return pythonDistributionPromise;
}

function productTestRoot(prefix: string): string {
  // The Windows product boundary intentionally rejects shared ancestors such
  // as the hosted runner's temp directory. Use the private user profile for
  // positive end-to-end fixtures on that platform.
  return join(process.platform === "win32" ? homedir() : tmpdir(), prefix);
}

function config(withPlugin = false): PipelineConfig {
  return {
    name: "runtime-fixture",
    settings: {
      storageRoot: "content",
      cacheDir: ".forme/cache",
      reproducibleBuild: false,
      maxConcurrency: null,
      logLevel: "info",
      bestEffort: false,
      deadlineMs: null,
    },
    stages: withPlugin ? [{
      id: "plugin",
      stage: { kind: "stage-ref", packageName: "@example/plugin" },
    }] : [],
  } as PipelineConfig;
}

function fixtureDependencies() {
  const host = { dispose: vi.fn(), plugins: new Map(), loadStage: vi.fn() } as unknown as PluginHost;
  const orchestrator = { dispose: vi.fn() } as unknown as Orchestrator;
  const factory = { launch: vi.fn() } as unknown as PluginProcessFactory;
  const hostOptions: PluginHostOptions[] = [];
  const orchestratorOptions: unknown[] = [];
  const selected: string[] = [];
  const verified: Array<readonly [string, "install-root" | "existing-target-tree"]> = [];
  const capabilityRequests: Array<readonly [string, readonly string[]]> = [];
  const identities = ["stable", "stable"];
  const dependencies: ProductRuntimeDependencies = {
    createPluginHost: async options => {
      hostOptions.push(options);
      return host;
    },
    createOrchestrator: options => {
      orchestratorOptions.push(options);
      return orchestrator;
    },
    createCache: path => ({ path }) as never,
    createLinuxSandboxFactory: options => {
      selected.push(`linux:${JSON.stringify(options)}`);
      return factory;
    },
    createMacosSandboxFactory: options => {
      selected.push(`darwin:${JSON.stringify(options)}`);
      return factory;
    },
    createWindowsSandboxFactory: options => {
      selected.push(`win32:${JSON.stringify(options)}`);
      return factory;
    },
    verifyWindowsAcl: async (path, scope) => {
      verified.push([path, scope]);
      return true;
    },
    pluginRootIdentity: async () => identities.shift() ?? "stable",
    createCapabilityApis: (storageRoot, reservedRoots) => {
      capabilityRequests.push([storageRoot, reservedRoots]);
      return { storage: { fixture: true } } as never;
    },
  };
  return {
    dependencies, host, orchestrator, factory, hostOptions, orchestratorOptions,
    selected, verified, identities, capabilityRequests,
  };
}

describe("installed plugin runtime composition", () => {
  it("keeps direct-only pipelines on the process-free path", async () => {
    const fixture = fixtureDependencies();
    const result = await createProductOrchestrator({
      config: config(false),
      projectRoot: PROJECT_ROOT,
      cacheRoot: CACHE_ROOT,
      platform: "linux",
    }, fixture.dependencies);

    expect(result).toBe(fixture.orchestrator);
    expect(fixture.hostOptions).toEqual([]);
    expect(fixture.selected).toEqual([]);
    expect(fixture.orchestratorOptions).toEqual([{ cache: { path: CACHE_ROOT }, logger: expect.anything() }]);
  });

  it.each(["linux", "darwin", "win32"] as const)(
    "loads persistent grants and selects the %s native sandbox",
    async platform => {
      const fixture = fixtureDependencies();
      const controller = new AbortController();
      const result = await createProductOrchestrator({
        config: config(true),
        projectRoot: PROJECT_ROOT,
        cacheRoot: CACHE_ROOT,
        platform,
        runtimeExecutables: { python: "/runtime/python" },
        runtimeRoots: { python: "/runtime" },
        signal: controller.signal,
      }, fixture.dependencies);

      expect(result).toBe(fixture.orchestrator);
      expect(fixture.selected).toEqual([
        `${platform}:${JSON.stringify({
          runtimeExecutables: { python: "/runtime/python" },
          runtimeRoots: { python: "/runtime" },
        })}`,
      ]);
      expect(fixture.hostOptions).toEqual([expect.objectContaining({
        roots: [resolve(PROJECT_ROOT, "forme-plugins")],
        loadPersistentGrants: true,
        processFactory: fixture.factory,
        storageRoot: resolve(PROJECT_ROOT, "content"),
        cacheDirectory: CACHE_ROOT,
        signal: controller.signal,
        capabilityApis: { storage: { fixture: true } },
      })]);
      expect(fixture.orchestratorOptions).toEqual([expect.objectContaining({
        cache: { path: CACHE_ROOT },
        pluginHost: fixture.host,
      })]);
      expect(fixture.capabilityRequests).toEqual([[
        resolve(PROJECT_ROOT, "content"),
        [resolve(PROJECT_ROOT, "forme-plugins"), CACHE_ROOT],
      ]]);
      expect(fixture.verified).toEqual(platform === "win32"
        ? [[resolve(PROJECT_ROOT, "forme-plugins"), "existing-target-tree"]]
        : []);
    },
  );

  it("maps trusted non-default runtime distributions from project config", async () => {
    const fixture = fixtureDependencies();
    const base = config(true);
    const configured: PipelineConfig = {
      ...base,
      settings: {
        ...base.settings,
        pluginRuntimes: {
          python: { executable: "/trusted/python/bin/python3", root: "/trusted/python" },
        },
      },
    };
    await createProductOrchestrator({
      config: configured,
      projectRoot: PROJECT_ROOT,
      cacheRoot: null,
      platform: "linux",
    }, fixture.dependencies);
    expect(fixture.selected).toEqual([
      `linux:${JSON.stringify({
        runtimeExecutables: { python: "/trusted/python/bin/python3" },
        runtimeRoots: { python: "/trusted/python" },
      })}`,
    ]);
  });

  it("fails closed on an unsupported platform before plugin discovery", async () => {
    const fixture = fixtureDependencies();
    await expect(createProductOrchestrator({
      config: config(true),
      projectRoot: PROJECT_ROOT,
      cacheRoot: null,
      platform: "aix",
    }, fixture.dependencies)).rejects.toThrow("unsupported plugin sandbox platform: aix");
    expect(fixture.hostOptions).toEqual([]);
    expect(fixture.orchestratorOptions).toEqual([]);
  });

  it("disposes a newly created host when orchestrator construction fails", async () => {
    const fixture = fixtureDependencies();
    fixture.dependencies.createOrchestrator = () => { throw new Error("construction failed"); };
    await expect(createProductOrchestrator({
      config: config(true),
      projectRoot: PROJECT_ROOT,
      cacheRoot: null,
      platform: "linux",
    }, fixture.dependencies)).rejects.toThrow("construction failed");
    expect(fixture.host.dispose).toHaveBeenCalledOnce();
  });

  it("fails before Windows discovery when the installed tree ACL is unsafe", async () => {
    const fixture = fixtureDependencies();
    fixture.dependencies.verifyWindowsAcl = async () => false;
    await expect(createProductOrchestrator({
      config: config(true),
      projectRoot: PROJECT_ROOT,
      cacheRoot: null,
      platform: "win32",
    }, fixture.dependencies)).rejects.toThrow("unsafe Windows plugin install-root ACL");
    expect(fixture.hostOptions).toEqual([]);
  });

  it("disposes the Windows host when the verified install root changes identity", async () => {
    const fixture = fixtureDependencies();
    fixture.identities.splice(0, fixture.identities.length, "before", "after");
    await expect(createProductOrchestrator({
      config: config(true),
      projectRoot: PROJECT_ROOT,
      cacheRoot: null,
      platform: "win32",
    }, fixture.dependencies)).rejects.toThrow("changed identity after plugin discovery");
    expect(fixture.host.dispose).toHaveBeenCalledOnce();
    expect(fixture.orchestratorOptions).toEqual([]);
  });

  it.skipIf(!["linux", "darwin", "win32"].includes(process.platform))(
    "runs a mixed direct/plugin pipeline through the native platform sandbox",
    async () => {
      const projectRoot = await mkdtemp(productTestRoot("forme-product-runtime-"));
      try {
        const installed = join(projectRoot, "forme-plugins", "plugin-echo");
        await mkdir(dirname(installed), { recursive: true });
        await cp(pluginFixture, installed, { recursive: true });
        const plugins = await discoverPlugins([join(projectRoot, "forme-plugins")]);
        const plugin = plugins.get("@example/echo");
        expect(plugin).toBeDefined();
        await writeFile(join(installed, "grants.toml"), formatGrantsFile({
          manifestHash: plugin!.manifestHash,
          granted: [{ capability: "storage:read", grantedAt: "2026-10-02T00:00:00Z" }],
        }));
        await mkdir(join(projectRoot, "posts"));
        await writeFile(join(projectRoot, "posts", "input.md"), "host-mediated");

        const contentNode = { name: "ContentNode", version: "1.0" } as const;
        const source = {
          name: "@example/source",
          version: "1.0.0",
          apiVersion: 2,
          description: "direct first-party source",
          consumes: { name: "Void", version: "1.0" },
          produces: contentNode,
          capabilities: [],
          configSchema: null,
          async run() {
            return { path: "index.md", body: "hello", readPath: "posts/input.md" };
          },
        };
        const mixed = {
          ...config(true),
          settings: { ...config(true).settings, storageRoot: ".", cacheDir: null },
          stages: [
            { id: "source", stage: source },
            {
              id: "plugin",
              stage: { kind: "stage-ref", packageName: "@example/echo", export: "echo" },
              config: {},
            },
          ],
          wires: [{ from: { id: "source" }, to: { id: "plugin" } }],
          outputs: [{ fromInstance: "plugin", name: "result" }],
        } as PipelineConfig;
        const orchestrator = await createProductOrchestrator({
          config: mixed,
          projectRoot,
          cacheRoot: null,
        });
        try {
          const pipeline = await orchestrator.buildPipeline(mixed);
          const result = await orchestrator.runOnce(pipeline);
          expect(result.errors).toEqual([]);
          expect(result.outcome).toBe("success");
          expect(result.outputs.result).toMatchObject({
            path: "index.md",
            body: "hello",
            bytes: Buffer.from("host-mediated").toString("base64"),
          });
        } finally {
          await orchestrator.dispose();
        }
      } finally {
        await rm(projectRoot, { recursive: true, force: true });
      }
    },
    20_000,
  );

  it.skipIf(!["linux", "darwin", "win32"].includes(process.platform))(
    "runs a configured Python plugin through the product sandbox",
    async () => {
      const python = await pythonDistribution();
      const projectRoot = await mkdtemp(productTestRoot("forme-python-product-"));
      try {
        const installed = join(projectRoot, "forme-plugins", "python-echo");
        await mkdir(installed, { recursive: true });
        await writeFile(join(installed, "plugin.toml"), pythonManifest());
        await writeFile(join(installed, "plugin.py"), pythonPlugin());
        const contentNode = { name: "ContentNode", version: "1.0" } as const;
        const source = {
          name: "@example/python-source",
          version: "1.0.0",
          apiVersion: 2,
          description: "direct source for Python product composition",
          consumes: { name: "Void", version: "1.0" },
          produces: contentNode,
          capabilities: [],
          configSchema: null,
          async run() { return { value: "python-product" }; },
        };
        const pipelineConfig = {
          ...config(true),
          settings: {
            ...config(true).settings,
            storageRoot: ".",
            cacheDir: null,
            pluginRuntimes: {
              python: { executable: await realpath(python.executable), root: await realpath(python.root) },
            },
          },
          stages: [
            { id: "source", stage: source },
            { id: "plugin", stage: { kind: "stage-ref", packageName: "@example/python-product" } },
          ],
          wires: [{ from: { id: "source" }, to: { id: "plugin" } }],
          outputs: [{ fromInstance: "plugin", name: "result" }],
        } as PipelineConfig;
        const orchestrator = await createProductOrchestrator({
          config: pipelineConfig,
          projectRoot,
          cacheRoot: null,
        });
        try {
          const pipeline = await orchestrator.buildPipeline(pipelineConfig);
          const result = await orchestrator.runOnce(pipeline);
          expect(result.outcome, JSON.stringify(result.errors)).toBe("success");
          expect(result.outputs.result).toEqual({ value: "python-product" });
        } finally {
          await orchestrator.dispose();
        }
      } finally {
        await rm(projectRoot, { recursive: true, force: true });
      }
    },
    90_000,
  );
});

function pythonManifest(): string {
  return `manifestVersion = 1
[plugin]
name = "@example/python-product"
version = "1.0.0"
apiVersion = 2
[runtime]
kind = "python"
entry = "./plugin.py"
[[contributes.stages]]
id = "echo"
consumes = "ContentNode"
produces = "ContentNode"
`;
}

function pythonPlugin(): string {
  return String.raw`import json, sys

def send(message):
    payload = json.dumps(message, separators=(",", ":")).encode("utf-8")
    sys.stdout.buffer.write(b"Content-Length: " + str(len(payload)).encode("ascii") + b"\r\n\r\n" + payload)
    sys.stdout.buffer.flush()

while True:
    header = sys.stdin.buffer.readline()
    if not header:
        break
    length = int(header.decode("ascii").split(":", 1)[1].strip())
    if sys.stdin.buffer.readline() != b"\r\n":
        raise RuntimeError("malformed frame")
    message = json.loads(sys.stdin.buffer.read(length))
    method = message.get("method")
    if method == "handshake":
        params = message["params"]
        send({"jsonrpc":"2.0","id":message["id"],"result":{
            "pluginName":params["pluginName"],"pluginVersion":params["pluginVersion"],
            "apiVersion":params["apiVersion"],"protocolVersion":params["protocolVersion"],
            "runner":"forme-python-product-fixture","runnerVersion":"1.0.0"}})
    elif method == "announce":
        send({"jsonrpc":"2.0","id":message["id"],"result":{"stage":{
            "id":"echo","consumes":"ContentNode","produces":"ContentNode",
            "capabilities":[],"configSchemaHash":None}}})
    elif method == "stage.init":
        send({"jsonrpc":"2.0","id":message["id"],"result":None})
    elif method == "stage.run":
        send({"jsonrpc":"2.0","id":message["id"],"result":{
            "kind":"single","value":message["params"]["input"]}})
    elif method == "stage.dispose":
        send({"jsonrpc":"2.0","id":message["id"],"result":None})
        break
`;
}
