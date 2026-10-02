import { cp, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it, vi } from "vitest";
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
  };
  return { dependencies, host, orchestrator, factory, hostOptions, orchestratorOptions, selected, verified };
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
      const result = await createProductOrchestrator({
        config: config(true),
        projectRoot: PROJECT_ROOT,
        cacheRoot: CACHE_ROOT,
        platform,
        runtimeExecutables: { python: "/runtime/python" },
        runtimeRoots: { python: "/runtime" },
      }, fixture.dependencies);

      expect(result).toBe(fixture.orchestrator);
      expect(fixture.selected).toEqual([
        `${platform}:${JSON.stringify({
          runtimeExecutables: { python: "/runtime/python" },
          runtimeRoots: { python: "/runtime" },
        })}`,
      ]);
      expect(fixture.hostOptions).toEqual([expect.objectContaining({
        roots: [join(PROJECT_ROOT, "forme-plugins")],
        loadPersistentGrants: true,
        processFactory: fixture.factory,
        storageRoot: join(PROJECT_ROOT, "content"),
        cacheDirectory: CACHE_ROOT,
      })]);
      expect(fixture.orchestratorOptions).toEqual([expect.objectContaining({
        cache: { path: CACHE_ROOT },
        pluginHost: fixture.host,
      })]);
      expect(fixture.verified).toEqual(platform === "win32"
        ? [[join(PROJECT_ROOT, "forme-plugins"), "existing-target-tree"]]
        : []);
    },
  );

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

  it.skipIf(!["linux", "darwin", "win32"].includes(process.platform))(
    "runs a mixed direct/plugin pipeline through the native platform sandbox",
    async () => {
      const projectRoot = await mkdtemp(join(tmpdir(), "forme-product-runtime-"));
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

        const contentNode = { name: "ContentNode", version: "1.0" } as const;
        const source = {
          name: "@example/source",
          version: "1.0.0",
          apiVersion: 1,
          description: "direct first-party source",
          consumes: { name: "Void", version: "1.0" },
          produces: contentNode,
          capabilities: [],
          configSchema: null,
          async run() {
            return { path: "index.md", body: "hello" };
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
          expect(result.outcome).toBe("success");
          expect(result.outputs.result).toMatchObject({ path: "index.md", body: "hello" });
        } finally {
          await orchestrator.dispose();
        }
      } finally {
        await rm(projectRoot, { recursive: true, force: true });
      }
    },
    20_000,
  );
});
