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
import { BENCHMARK_ROOT } from "./release-benchmark.js";

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
