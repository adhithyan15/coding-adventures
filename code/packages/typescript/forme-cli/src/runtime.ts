import { resolve } from "node:path";
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
import { silentLogger } from "@coding-adventures/forme-stage";

type ConfigurableRuntime = "node" | "deno" | "bun" | "python";

export interface ProductRuntimeOptions {
  readonly config: PipelineConfig;
  readonly projectRoot: string;
  readonly cacheRoot: string | null;
  readonly platform?: NodeJS.Platform;
  readonly runtimeExecutables?: Partial<Readonly<Record<ConfigurableRuntime, string>>>;
  readonly runtimeRoots?: Partial<Readonly<Record<ConfigurableRuntime, string>>>;
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
}

const defaultDependencies: ProductRuntimeDependencies = {
  createPluginHost,
  createOrchestrator,
  createCache: path => filesystemCache(path),
  createLinuxSandboxFactory,
  createMacosSandboxFactory,
  createWindowsSandboxFactory,
  verifyWindowsAcl: createWindowsInstallAclVerifier(),
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
    ...(options.runtimeExecutables === undefined ? {} : { runtimeExecutables: options.runtimeExecutables }),
    ...(options.runtimeRoots === undefined ? {} : { runtimeRoots: options.runtimeRoots }),
  };
  const pluginRoot = resolve(options.projectRoot, "forme-plugins");
  const platform = options.platform ?? process.platform;
  if (platform === "win32"
      && !await dependencies.verifyWindowsAcl(pluginRoot, "existing-target-tree")) {
    throw new Error("unsafe Windows plugin install-root ACL");
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
  });
  try {
    return dependencies.createOrchestrator({ ...base, pluginHost });
  } catch (error) {
    await pluginHost.dispose();
    throw error;
  }
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
