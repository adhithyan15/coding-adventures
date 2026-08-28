/** Seven-stage Forme DAG for the repository root landing page. */

import sourceFs from "@coding-adventures/forme-source-fs";
import resolveAssetRefsFs from "@coding-adventures/forme-resolve-asset-refs-fs";
import router from "@coding-adventures/forme-router";
import loadAssetsFs from "@coding-adventures/forme-load-assets-fs";
import emitSiteFs from "@coding-adventures/forme-emit-site-fs";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import parseLanding from "./parse-landing.ts";
import renderLanding from "./render-landing.ts";

const config: PipelineConfig = {
  name: "coding-adventures-landing",
  settings: {
    storageRoot: ".",
    cacheDir: null,
    reproducibleBuild: false,
    maxConcurrency: null,
    logLevel: "info",
    bestEffort: false,
    deadlineMs: null,
  },
  stages: [
    {
      id: "source",
      stage: sourceFs,
      config: { glob: "**/*.landing", root: "data", persistIdentities: true },
    },
    { id: "parse", stage: parseLanding, config: {} },
    {
      id: "resolve-assets",
      stage: resolveAssetRefsFs,
      config: { root: "data", persistIdentities: true },
    },
    {
      id: "route",
      stage: router,
      config: { routeTemplate: "/{slug}.html" },
    },
    {
      id: "render",
      stage: renderLanding,
      config: { cssPath: "landing.css" },
    },
    {
      id: "load-assets",
      stage: loadAssetsFs,
      config: { root: "data" },
    },
    {
      id: "emit",
      stage: emitSiteFs,
      config: {
        outDir: "dist",
        assetDir: "assets",
        publicPathPrefix: "/coding-adventures",
      },
    },
  ],
  wires: [
    { from: { id: "source" }, to: { id: "parse" } },
    { from: { id: "parse" }, to: { id: "resolve-assets" } },
    { from: { id: "resolve-assets" }, to: { id: "route" } },
    { from: { id: "route" }, to: { id: "render" } },
    { from: { id: "route" }, to: { id: "load-assets" } },
    { from: { id: "render" }, to: { id: "emit" } },
    { from: { id: "load-assets" }, to: { id: "emit", port: "assets" } },
  ],
  outputs: [{ fromInstance: "emit", name: "site" }],
};

export default config;
