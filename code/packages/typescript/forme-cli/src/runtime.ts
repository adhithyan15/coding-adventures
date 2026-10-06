import { resolve } from "node:path";
import { lstat, realpath } from "node:fs/promises";
import { filesystemCache, type CacheBackend } from "@coding-adventures/forme-cache";
import {
  createOrchestrator,
  type Orchestrator,
  type OrchestratorOptions,
} from "@coding-adventures/forme-orchestrator";
import {
  createPluginHost,
  type PluginHost,
  type PluginHostOptions,
  type PluginProcessFactory,
} from "@coding-adventures/forme-plugin-host";
import { isStageRef, type PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import { createLinuxSandboxFactory } from "@coding-adventures/forme-sandbox-linux";
import { createMacosSandboxFactory } from "@coding-adventures/forme-sandbox-macos";
import {
  createWindowsInstallAclVerifier,
  createWindowsSandboxFactory,
  type WindowsInstallAclVerifier,
} from "@coding-adventures/forme-sandbox-windows";
import { silentLogger, type StageContext } from "@coding-adventures/forme-stage";
import { createProductCapabilityApis } from "./capability-apis.js";

type ConfigurableRuntime = "node" | "deno" | "bun" | "python";

export interface ProductRuntimeOptions {
  readonly config: PipelineConfig;
  readonly projectRoot: string;
  readonly cacheRoot: string | null;
  readonly platform?: NodeJS.Platform;
  readonly runtimeExecutables?: Partial<Readonly<Record<ConfigurableRuntime, string>>>;
  readonly runtimeRoots?: Partial<Readonly<Record<ConfigurableRuntime, string>>>;
  /** Cooperative cancellation for installed-plugin discovery and grant loading. */
  readonly signal?: AbortSignal;
}

interface SandboxFactoryOptions {
  readonly runtimeExecutables?: ProductRuntimeOptions["runtimeExecutables"];
  readonly runtimeRoots?: ProductRuntimeOptions["runtimeRoots"];
}

export interface ProductRuntimeDependencies {
  createPluginHost(options: PluginHostOptions): Promise<PluginHost>;
  createOrchestrator(options: OrchestratorOptions): Orchestrator;
  createCache(path: string): CacheBackend;
  createLinuxSandboxFactory(options: SandboxFactoryOptions): PluginProcessFactory;
  createMacosSandboxFactory(options: SandboxFactoryOptions): PluginProcessFactory;
  createWindowsSandboxFactory(options: SandboxFactoryOptions): PluginProcessFactory;
  verifyWindowsAcl: WindowsInstallAclVerifier;
  pluginRootIdentity(path: string): Promise<string>;
  createCapabilityApis(storageRoot: string, reservedRoots: readonly string[]): Partial<Pick<
    StageContext,
    "storage" | "network" | "env" | "filesystem"
  >>;
}

const defaultDependencies: ProductRuntimeDependencies = {
  createPluginHost,
  createOrchestrator,
  createCache: path => filesystemCache(path),
  createLinuxSandboxFactory,
  createMacosSandboxFactory,
  createWindowsSandboxFactory,
  verifyWindowsAcl: createWindowsInstallAclVerifier(),
  pluginRootIdentity: async path => {
    const canonical = await realpath(path);
    if (canonical.toLowerCase() !== resolve(path).toLowerCase()) {
      throw new Error("Windows plugin install root is not canonical");
    }
    const stat = await lstat(canonical, { bigint: true });
    if (!stat.isDirectory() || stat.isSymbolicLink()) {
      throw new Error("Windows plugin install root is not a real directory");
    }
    return `${stat.dev}:${stat.ino}`;
  },
  createCapabilityApis: createProductCapabilityApis,
};

/** Compose the installed-plugin boundary only when the pipeline references it. */
export async function createProductOrchestrator(
  options: ProductRuntimeOptions,
  dependencies: ProductRuntimeDependencies = defaultDependencies,
): Promise<Orchestrator> {
  const base: OrchestratorOptions = {
    ...(options.cacheRoot === null ? {} : { cache: dependencies.createCache(options.cacheRoot) }),
    logger: silentLogger(),
  };
  if (!options.config.stages.some(stage => isStageRef(stage.stage))) {
    return dependencies.createOrchestrator(base);
  }

  const sandboxOptions: SandboxFactoryOptions = {
    ...runtimeConfiguration(options),
  };
  const pluginRoot = resolve(options.projectRoot, "forme-plugins");
  const platform = options.platform ?? process.platform;
  let verifiedRootIdentity: string | null = null;
  if (platform === "win32") {
    verifiedRootIdentity = await dependencies.pluginRootIdentity(pluginRoot);
    if (!await dependencies.verifyWindowsAcl(pluginRoot, "existing-target-tree")) {
      throw new Error("unsafe Windows plugin install-root ACL");
    }
  }
  const processFactory = selectSandboxFactory(
    platform,
    sandboxOptions,
    dependencies,
  );
  const pluginHost = await dependencies.createPluginHost({
    roots: [pluginRoot],
    loadPersistentGrants: true,
    processFactory,
    storageRoot: resolve(options.projectRoot, options.config.settings.storageRoot),
    cacheDirectory: options.cacheRoot,
    logger: silentLogger(),
    capabilityApis: dependencies.createCapabilityApis(
      resolve(options.projectRoot, options.config.settings.storageRoot),
      [pluginRoot, ...(options.cacheRoot === null ? [] : [options.cacheRoot])],
    ),
    ...(options.signal === undefined ? {} : { signal: options.signal }),
  });
  try {
    if (verifiedRootIdentity !== null
        && await dependencies.pluginRootIdentity(pluginRoot) !== verifiedRootIdentity) {
      throw new Error("Windows plugin install root changed identity after plugin discovery");
    }
    return dependencies.createOrchestrator({ ...base, pluginHost });
  } catch (error) {
    await pluginHost.dispose();
    throw error;
  }
}

function runtimeConfiguration(options: ProductRuntimeOptions): SandboxFactoryOptions {
  const configured = options.config.settings.pluginRuntimes ?? {};
  const runtimeExecutables: Partial<Record<ConfigurableRuntime, string>> = {};
  const runtimeRoots: Partial<Record<ConfigurableRuntime, string>> = {};
  for (const kind of ["deno", "bun", "python"] as const) {
    const runtime = configured[kind];
    if (runtime === undefined) continue;
    runtimeExecutables[kind] = runtime.executable;
    runtimeRoots[kind] = runtime.root;
  }
  Object.assign(runtimeExecutables, options.runtimeExecutables);
  Object.assign(runtimeRoots, options.runtimeRoots);
  return {
    ...(Object.keys(runtimeExecutables).length === 0 ? {} : { runtimeExecutables }),
    ...(Object.keys(runtimeRoots).length === 0 ? {} : { runtimeRoots }),
  };
}

function selectSandboxFactory(
  platform: NodeJS.Platform,
  options: SandboxFactoryOptions,
  dependencies: ProductRuntimeDependencies,
): PluginProcessFactory {
  if (platform === "linux") return dependencies.createLinuxSandboxFactory(options);
  if (platform === "darwin") return dependencies.createMacosSandboxFactory(options);
  if (platform === "win32") return dependencies.createWindowsSandboxFactory(options);
  throw new Error(`unsupported plugin sandbox platform: ${platform}`);
}
