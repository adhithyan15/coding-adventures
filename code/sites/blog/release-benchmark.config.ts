/**
 * The release benchmark must exercise the live product rather than a smaller
 * look-alike pipeline.  Deriving this configuration from `forme.config.ts`
 * keeps all thirteen stages, wires, outputs, themes, and backend choices exact;
 * only mutable filesystem locations and the diagnostic name move under a
 * disposable benchmark root.
 */
import liveConfig from "./forme.config.js";
import type {
  PipelineConfig,
  StageInstanceSpec,
} from "@coding-adventures/forme-pipeline-config";
import { lstatSync, realpathSync } from "node:fs";
import { dirname, isAbsolute, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const suppliedRoot = process.env.FORME_RELEASE_BENCHMARK_ROOT;
if (suppliedRoot === undefined || !isAbsolute(suppliedRoot)) {
  throw new Error("FORME_RELEASE_BENCHMARK_ROOT must name an absolute private temporary directory");
}
const suppliedRootInfo = lstatSync(suppliedRoot);
if (!suppliedRootInfo.isDirectory() || suppliedRootInfo.isSymbolicLink()) {
  throw new Error("FORME_RELEASE_BENCHMARK_ROOT must be a real directory");
}
const BENCHMARK_ROOT = realpathSync(resolve(suppliedRoot));
const projectRoot = realpathSync(dirname(fileURLToPath(import.meta.url)));
const relativeToProject = relative(projectRoot, BENCHMARK_ROOT);
if (relativeToProject === "" || isAbsolute(relativeToProject)
    || relativeToProject === ".." || relativeToProject.startsWith(`..${sep}`)
    || !relativeToProject.split(sep)[0]?.startsWith(".forme-release-benchmark-")) {
  throw new Error(
    "FORME_RELEASE_BENCHMARK_ROOT must be a private Forme benchmark directory inside the blog project",
  );
}

function relocate(stage: StageInstanceSpec): StageInstanceSpec {
  const config = typeof stage.config === "object" && stage.config !== null
    ? stage.config as Readonly<Record<string, unknown>>
    : {};

  switch (stage.id) {
    case "source":
    case "resolve-assets":
    case "load-assets":
      return { ...stage, config: { ...config, root: `${BENCHMARK_ROOT}/data` } };
    case "emit-articles":
    case "emit-surface":
      return { ...stage, config: { ...config, outDir: `${BENCHMARK_ROOT}/dist` } };
    case "render-pages":
      return {
        ...stage,
        config: {
          ...config,
          interactivity: [],
          islandModules: [],
          allowExecutableAssets: false,
        },
      };
    case "render-terminal":
      return { ...stage, config: { ...config, interactivity: [] } };
    default:
      return stage;
  }
}

const config: PipelineConfig = {
  ...liveConfig,
  name: "coding-adventures-blog-release-benchmark",
  settings: {
    ...liveConfig.settings,
    cacheDir: `${BENCHMARK_ROOT}/cache`,
    reproducibleBuild: false,
  },
  stages: liveConfig.stages.map(relocate),
};

export default config;
