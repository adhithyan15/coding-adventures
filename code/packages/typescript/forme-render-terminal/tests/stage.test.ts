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
        { type: "raw_block", format: "html", value: "<div id=\"fallback\"><script>bad()</script>Raw fallback</div>" },
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
    expect(result[0]!.text).toMatch(/\u001b\[1mHello\[31m\n\n\u001b\[0m/);
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

  it("renders every portable Document AST family", async () => {
    const complete = {
      ...node(),
      route: "/complete",
      assetRefs: [],
      document: { type: "document", children: [
        { type: "code_block", language: "ts", value: "const x = 1;" },
        { type: "blockquote", children: [{ type: "paragraph", children: [{ type: "text", value: "quote" }] }] },
        { type: "list", ordered: true, start: 3, tight: false, children: [
          { type: "list_item", children: [{ type: "paragraph", children: [{ type: "text", value: "item" }] }] },
          { type: "task_item", checked: true, children: [{ type: "paragraph", children: [{ type: "text", value: "done" }] }] },
        ] },
        { type: "thematic_break" },
        { type: "table", align: [null], children: [
          { type: "table_row", isHeader: true, children: [{ type: "table_cell", children: [{ type: "text", value: "head" }] }] },
        ] },
        { type: "paragraph", children: [
          { type: "emphasis", children: [{ type: "text", value: "em" }] },
          { type: "strikethrough", children: [{ type: "text", value: "strike" }] },
          { type: "code_span", value: "code" },
          { type: "link", destination: "https://example.com\u001b[1m", title: null, children: [{ type: "text", value: "link" }] },
          { type: "autolink", destination: "a@example.com", isEmail: true },
          { type: "raw_inline", format: "latex", value: "\\bad" },
          { type: "hard_break" },
          { type: "soft_break" },
        ] },
      ] },
    } as ContentNode;
    const structuralTheme = {
      ...emptyStyleDocument(),
      rules: [
        { id: styleRuleId("and-not"), selector: sel.and(sel.type("strong"), sel.not(sel.type("em"))), properties: [{ kind: "font-weight" as const, value: 700 }] },
        { id: styleRuleId("child"), selector: sel.childOf(sel.type("p"), sel.type("strong")), properties: [{ kind: "font-weight" as const, value: 700 }] },
        { id: styleRuleId("descendant"), selector: sel.descendantOf(sel.type("p"), sel.type("strong")), properties: [{ kind: "font-weight" as const, value: 700 }] },
        { id: styleRuleId("never"), selector: sel.or(sel.custom("x"), sel.tag("x"), sel.id("x"), sel.role("x"), sel.nth(sel.type("p"), 0), sel.adjacent(sel.type("p"), sel.type("p"))), properties: [{ kind: "font-weight" as const, value: 700 }] },
      ],
    };
    const results: TerminalBuffer[] = [];
    for await (const item of renderTerminal.run(values([complete]) as never, { style: structuralTheme } as never, context()) as AsyncIterable<TerminalBuffer>) results.push(item);
    expect(results[0]!.text).toContain("3. item");
    expect(results[0]!.text).toContain("[x] done");
    expect(results[0]!.text).toContain("> quote");
    expect(results[0]!.text).toContain("link <https://example.com[1m>");
    expect(results[0]!.degradations).toMatchObject([{ code: "raw-node-dropped", format: "latex" }]);
  });

  it("fails closed on malformed renderer config", async () => {
    const drain = async (config: unknown) => {
      for await (const _ of renderTerminal.run(values([node()]) as never, config as never, context()) as AsyncIterable<unknown>) { /* drain */ }
    };
    await expect(drain({ unknown: true })).rejects.toThrow(/unknown/);
    await expect(drain({ activeStyleContexts: "screen" })).rejects.toThrow(/bounded array/);
    await expect(drain({ interactivity: "bad" })).rejects.toThrow(/bounded array/);
    await expect(drain({ interactivity: [{ route: "bad", document: interactivity[0]!.document }] })).rejects.toThrow(/route is invalid/);
    await expect(drain({ interactivity: [interactivity[0], interactivity[0]] })).rejects.toThrow(/duplicate interactivity route/);
    await expect(drain({ activeStyleContexts: ["x".repeat(129)] })).rejects.toThrow(/bounded string/);
    const accessor = {} as Record<string, unknown>;
    Object.defineProperty(accessor, "style", { get: () => theme });
    await expect(drain(accessor)).rejects.toThrow(/accessor/);
    const symbol = { [Symbol("hidden")]: true };
    await expect(drain(symbol)).rejects.toThrow(/symbol keys/);
    const sparse = new Array(1);
    await expect(drain({ activeStyleContexts: sparse })).rejects.toThrow(/data element/);
    const arrayAccessor: unknown[] = [];
    Object.defineProperty(arrayAccessor, "0", { get: () => "screen" });
    Object.defineProperty(arrayAccessor, "length", { value: 1 });
    await expect(drain({ activeStyleContexts: arrayAccessor })).rejects.toThrow(/data element/);
  });

  it("bounds adversarial document and raw-HTML structure", async () => {
    const drain = async (input: ContentNode) => {
      for await (const _ of renderTerminal.run(values([input]) as never, {} as never, context()) as AsyncIterable<unknown>) { /* drain */ }
    };
    let child: any = { type: "text", value: "deep" };
    for (let index = 0; index < 514; index++) child = { type: "strong", children: [child] };
    await expect(drain({ ...node(), document: { type: "document", children: [child] } } as ContentNode)).rejects.toThrow(/structural limit/);
    const hugeRaw = { ...node(), document: { type: "document", children: [
      { type: "raw_block", format: "html", value: `<p>${"x".repeat(1_048_576)}</p>` },
    ] } } as ContentNode;
    await expect(drain(hugeRaw)).rejects.toThrow(/raw HTML exceeds/);
  });

  it("requires each configured island fallback exactly once", async () => {
    const drain = async () => {
      for await (const _ of renderTerminal.run(values([{ ...node(), document: { type: "document", children: [] } }]) as never, { interactivity } as never, context()) as AsyncIterable<unknown>) { /* drain */ }
    };
    await expect(drain()).rejects.toThrow(/fallback id.*0 matches/);
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
    await expect(packageTerminal.run(values([{ ...buffer, route: "/../escape" }]) as never, {} as never, context())).rejects.toThrow(/unsafe/);
    await expect(packageTerminal.run(values([{ ...buffer, route: "/CON" }]) as never, {} as never, context())).rejects.toThrow(/unsafe portable path/);
    await expect(packageTerminal.run(values([{ ...buffer, route: "/trail." }]) as never, {} as never, context())).rejects.toThrow(/unsafe portable path/);
    await expect(packageTerminal.run(values([{ ...buffer, route: `/${"x".repeat(240)}` }]) as never, {} as never, context())).rejects.toThrow(/unsafe portable path/);
    await expect(packageTerminal.run(values([{ ...buffer, route: "/Same" }, { ...buffer, route: "/same" }]) as never, {} as never, context())).rejects.toThrow(/portable path collision/);
    await expect(packageTerminal.run(values([buffer]) as never, { root: "NUL" } as never, context())).rejects.toThrow(/unsafe portable path/);
    await expect(packageTerminal.run(values([buffer]) as never, { extra: true } as never, context())).rejects.toThrow(/unknown/);
  });

  it("supports an empty default-root artifact", async () => {
    const artifact = await packageTerminal.run(values([]) as never, {} as never, context()) as any;
    expect(artifact.files).toEqual({});
    expect(artifact.manifest.routes).toEqual([]);
  });
});
