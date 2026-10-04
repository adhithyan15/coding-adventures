import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { validateAuthoringProject } from "@coding-adventures/forme-authoring-core";
import type { AuthoringProject, AuthoringSession } from "@coding-adventures/forme-authoring-core";
import { createAuthoringPreview } from "@coding-adventures/forme-authoring-preview";
import type { AuthoringPreviewAttempt, PublishedAuthoringPreview } from "@coding-adventures/forme-authoring-preview";
import emitFs from "@coding-adventures/forme-emit-fs";
import { computeRevisionId } from "@coding-adventures/forme-identity";
import { createOrchestrator } from "@coding-adventures/forme-orchestrator";
import type { RunResult } from "@coding-adventures/forme-orchestrator";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import renderStatic from "@coding-adventures/forme-render-static";
import router from "@coding-adventures/forme-router";
import { defineStage, silentLogger } from "@coding-adventures/forme-stage";
import classlessTheme from "@coding-adventures/forme-theme-classless";
import { Kinds, streamOf } from "@coding-adventures/forme-types";
import type { ContentNode, JsonValue, LogicalId } from "@coding-adventures/forme-types";

export interface BuildAuthoringProductInput {
  readonly project: AuthoringProject;
  readonly revision: string;
  readonly output: string;
}

export interface PreviewAuthoringProductInput {
  readonly project: AuthoringProject;
  readonly revision: string;
}

export interface PreviewAuthoringProductResult {
  readonly attempt: AuthoringPreviewAttempt;
  readonly snapshot: PublishedAuthoringPreview;
}

/** Build one exact persisted authoring revision through the shipping pipeline. */
export async function buildAuthoringProduct(input: BuildAuthoringProductInput): Promise<RunResult> {
  const project = checkedProductInput(input.project, input.revision);
  if (typeof input.output !== "string" || input.output.length === 0) {
    throw new TypeError("the product output directory is invalid");
  }
  const config = productConfig(project, input.revision, input.output);

  const orchestrator = createOrchestrator({ logger: silentLogger() });
  try {
    const pipeline = await orchestrator.buildPipeline(config);
    return await orchestrator.runOnce(pipeline);
  } finally {
    await orchestrator.dispose();
  }
}

/** Exercise the exact FM03 watch and FM07 snapshot path used by desktop preview. */
export async function previewAuthoringProduct(
  input: PreviewAuthoringProductInput,
): Promise<PreviewAuthoringProductResult> {
  const project = checkedProductInput(input.project, input.revision);
  const orchestrator = createOrchestrator({ logger: silentLogger() });
  let snapshot: PublishedAuthoringPreview | null = null;
  const coordinator = createAuthoringPreview({
    debounceMs: 0,
    orchestrator,
    materializer: {
      async prepare(previewInput) {
        const output = await mkdtemp(join(tmpdir(), "forme-desktop-preview-"));
        try {
          const pipeline = await orchestrator.buildPipeline(
            productConfig(previewInput.project, previewInput.revision, output),
          );
          return {
            pipeline,
            async release() { await rm(output, { recursive: true, force: true }); },
          };
        } catch (error) {
          await rm(output, { recursive: true, force: true });
          throw error;
        }
      },
    },
    publisher: {
      publish(value, commit) { commit(() => { snapshot = value; }); },
      publishFailure(_failure, commit) { commit(() => {}); },
    },
  });
  const session = readOnlySession(project, input.revision);
  try {
    const attempt = await coordinator.request(session);
    const published = snapshot as PublishedAuthoringPreview | null;
    if (attempt.outcome !== "ready" || published === null) {
      throw new Error("the exact product preview did not publish a snapshot");
    }
    return { attempt, snapshot: published };
  } finally {
    await coordinator.dispose();
    await orchestrator.dispose();
  }
}

function checkedProductInput(projectInput: AuthoringProject, revision: string): AuthoringProject {
  const project = validateAuthoringProject(projectInput);
  if (project.site.themeId !== "forme-classless") {
    throw new TypeError("the authoring project does not select a reviewed desktop theme");
  }
  if (typeof revision !== "string" || revision.length === 0) {
    throw new TypeError("the authoring revision is invalid");
  }
  return project;
}

function productConfig(project: AuthoringProject, revision: string, output: string): PipelineConfig {
  const source = authoringSource(project, revision);
  return {
    name: "forme-shell-desktop-product",
    settings: {
      storageRoot: output,
      cacheDir: null,
      reproducibleBuild: true,
      maxConcurrency: 1,
      logLevel: "error",
      bestEffort: false,
      deadlineMs: 30_000,
    },
    stages: [
      { id: "authoring", stage: source, config: {} },
      { id: "route", stage: router, config: { routeTemplate: "/{slug}.html" } },
      {
        id: "render",
        stage: renderStatic,
        config: {
          siteTitle: project.title,
          style: classlessTheme,
          activeStyleContexts: ["dark", "narrow", "high-contrast"],
        },
      },
      { id: "site", stage: emitFs, config: { outDir: output } },
    ],
    wires: [
      { from: { id: "authoring" }, to: { id: "route" } },
      { from: { id: "route" }, to: { id: "render" } },
      { from: { id: "render" }, to: { id: "site" } },
    ],
  };
}

function authoringSource(project: AuthoringProject, authoringRevision: string) {
  return defineStage({
    name: "forme-shell-desktop-authoring-source",
    version: "0.1.0",
    apiVersion: 1,
    description: "Yield a validated authoring snapshot as Content IR.",
    consumes: Kinds.Void,
    produces: streamOf(Kinds.ContentNode),
    capabilities: [],
    configSchema: { type: "object", properties: {} },
    async *run(_input, _config, context) {
      for (const document of project.documents) {
        context.cancellation.throwIfCancelled();
        const node: ContentNode = {
          identity: document.id as LogicalId,
          revision: computeRevisionId({
            authoringRevision,
            documentId: document.id,
            document: document.body,
          } as unknown as JsonValue),
          document: document.body,
          frontmatter: { title: document.title, slug: document.slug },
          route: null,
          assetRefs: [],
          sourcePath: `${document.slug}.md`,
        };
        yield node as never;
      }
    },
  });
}

function readOnlySession(project: AuthoringProject, revision: string): AuthoringSession {
  const immutable = async (): Promise<never> => {
    throw new Error("the desktop product preview session is immutable");
  };
  return Object.freeze({
    project,
    storageRevision: revision,
    canUndo: false,
    canRedo: false,
    dispatch: immutable,
    dispatchAtRevision: immutable,
    undo: immutable,
    redo: immutable,
  });
}
