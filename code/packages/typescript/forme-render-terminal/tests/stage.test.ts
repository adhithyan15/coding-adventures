import { describe, expect, it } from "vitest";
import {
  Kinds,
  streamOf,
  type ContentNode,
  type TerminalBuffer,
} from "@coding-adventures/forme-types";
import {
  createCancellationTokenSource,
  deniedEnvApi,
  deniedFilesystemApi,
  deniedNetworkApi,
  deniedShellApi,
  deniedStorageApi,
  inMemoryCache,
  inMemoryEventBus,
  noOpTelemetryEmitter,
  silentLogger,
  systemClock,
  type StageContext,
} from "@coding-adventures/forme-stage";
import { emptyStyleDocument, sel, styleRuleId } from "@coding-adventures/forme-style-ir";
import renderTerminal, { packageTerminal } from "../src/index.js";

const ID = "00000000-0000-7000-8000-000000000001" as ContentNode["identity"];
const REV = ("blake2b:" + "1".repeat(64)) as ContentNode["revision"];

function context(): StageContext {
  return {
    logger: silentLogger(), cancellation: createCancellationTokenSource().token,
    time: systemClock(), cache: inMemoryCache(), telemetry: noOpTelemetryEmitter(),
    storage: deniedStorageApi(), network: deniedNetworkApi(), env: deniedEnvApi(),
    filesystem: deniedFilesystemApi(), shell: deniedShellApi(), events: inMemoryEventBus(),
  };
}

async function* values<T>(items: readonly T[]): AsyncGenerator<T> { for (const item of items) yield item; }

function node(): ContentNode {
  return {
    identity: ID, revision: REV, route: "/blog/hello.html", sourcePath: "hello.md",
    frontmatter: {},
    assetRefs: [{ id: ID, nodePath: [3], role: "image", sourcePath: "hero.png" }],
    document: {
      type: "document",
      children: [
        { type: "heading", level: 1, children: [{ type: "text", value: "Hello\u001b[31m" }] },
        { type: "paragraph", children: [
          { type: "text", value: "Fallback " },
          { type: "strong", children: [{ type: "text", value: "works" }] },
        ] },
        { type: "raw_block", format: "html", value: "<script>bad()</script>" },
        { type: "image", destination: "hero.png", title: null, alt: "Hero" },
      ],
    },
  } as ContentNode;
}

const theme = {
  ...emptyStyleDocument(),
  rules: [
    { id: styleRuleId("heading"), selector: sel.heading(1), properties: [
      { kind: "font-weight" as const, value: 700 },
      { kind: "padding" as const, value: { top: { unit: "pt" as const, value: 1 }, right: { unit: "pt" as const, value: 1 }, bottom: { unit: "pt" as const, value: 1 }, left: { unit: "pt" as const, value: 1 } } },
    ] },
    { id: styleRuleId("strong"), selector: sel.type("strong"), properties: [{ kind: "font-weight" as const, value: 700 }] },
  ],
};

const interactivity = [{ route: "/blog/hello.html", document: {
  kind: "Interactivity", version: 1, state: [], bindings: [], handlers: [],
  islands: [{
    id: "explorer", packageName: "@example/explorer", export: "enhance",
    target: { kind: "element", id: "fallback" }, fallback: { kind: "element", id: "fallback" },
    activation: "load", config: {},
  }],
} }];

describe("render-terminal stage", () => {
  it("declares the backend boundary and no authority", () => {
    expect(renderTerminal.consumes).toEqual(streamOf(Kinds.ContentNode));
    expect(renderTerminal.produces).toEqual(streamOf(Kinds.TerminalBuffer));
    expect(renderTerminal.capabilities).toEqual([]);
    expect(packageTerminal.consumes).toEqual(streamOf(Kinds.TerminalBuffer));
    expect(packageTerminal.produces).toEqual(Kinds.DeployArtifact);
  });

  it("renders safe ANSI while preserving fallback and reporting every degradation", async () => {
    const result: TerminalBuffer[] = [];
    const output = renderTerminal.run(values([node()]) as never, { style: theme, interactivity } as never, context()) as AsyncIterable<TerminalBuffer>;
    for await (const item of output) result.push(item);
    expect(result).toHaveLength(1);
    expect(result[0]!.text).toContain("\u001b[1mHello[31m\u001b[0m");
    expect(result[0]!.text).toContain("Fallback");
    expect(result[0]!.text).not.toContain("<script>");
    expect(result[0]!.usedStyle).toEqual(["heading", "strong"]);
    expect(result[0]!.degradations.map(item => item.code)).toEqual([
      "raw-node-dropped", "asset-reference-dropped", "style-property-dropped", "interactivity-dropped",
    ]);
  });

  it("rejects unrouted content and unresolved themes", async () => {
    const unrouted = { ...node(), route: null };
    const consume = async (input: ContentNode, style = theme) => {
      for await (const _ of renderTerminal.run(values([input]) as never, { style } as never, context()) as AsyncIterable<unknown>) { /* drain */ }
    };
    await expect(consume(unrouted)).rejects.toThrow(/no route/);
    await expect(consume(node(), { ...theme, theme: "missing" })).rejects.toThrow(/unresolved/);
  });
});

describe("package-terminal stage", () => {
  it("packages ANSI and canonical degradation evidence without I/O", async () => {
    const rendered: TerminalBuffer[] = [];
    for await (const item of renderTerminal.run(values([node()]) as never, { style: theme, interactivity } as never, context()) as AsyncIterable<TerminalBuffer>) rendered.push(item);
    const artifact = await packageTerminal.run(values(rendered) as never, { root: "terminal" } as never, context()) as any;
    expect(Object.keys(artifact.files)).toEqual([
      "terminal/blog/hello.html.ansi",
      "terminal/blog/hello.html.degradations.json",
    ]);
    expect(new TextDecoder().decode(artifact.files["terminal/blog/hello.html.ansi"])).toBe(rendered[0]!.text);
    expect(JSON.parse(new TextDecoder().decode(artifact.files["terminal/blog/hello.html.degradations.json"]))).toEqual(rendered[0]!.degradations);
    expect(artifact.manifest.routes[0].target.path).toBe("terminal/blog/hello.html.ansi");
  });

  it("rejects duplicate routes and unsafe roots", async () => {
    const buffer: TerminalBuffer = {
      route: "/same", text: "x", usedStyle: [], usedAssets: [], degradations: [],
      provenance: { contributors: [{ identity: ID, revision: REV }], revision: REV },
    };
    await expect(packageTerminal.run(values([buffer, buffer]) as never, {} as never, context())).rejects.toThrow(/duplicate route/);
    await expect(packageTerminal.run(values([buffer]) as never, { root: "../escape" } as never, context())).rejects.toThrow(/root/);
  });
});
