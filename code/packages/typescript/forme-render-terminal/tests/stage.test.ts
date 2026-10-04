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
import { createOutputProvenance } from "@coding-adventures/forme-identity";
import renderTerminal, { packageTerminal } from "../src/index.js";

const ID = "00000000-0000-7000-8000-000000000001" as ContentNode["identity"];
const REV = ("blake2b:" + "1".repeat(64)) as ContentNode["revision"];
const PROVENANCE = createOutputProvenance([{ identity: ID, revision: REV }]);

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
    const hostilePath = { ...unrouted, sourcePath: "bad\u001b]52;c;x\u0007\u202E.md" };
    let message = "";
    try { await consume(hostilePath); } catch (error) { message = (error as Error).message; }
    expect(message).toContain("bad\\u{1b}]52;c;x\\u{7}\\u{202e}.md");
    expect(message).not.toMatch(/[\u001b\u0007\u202e]/);
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
    const symbolConfig = {};
    Object.defineProperty(symbolConfig, Symbol("hidden"), { get: () => { throw new Error("must not run"); } });
    await expect(drain(symbolConfig)).resolves.toBeUndefined();
    const sparse = new Array(1);
    await expect(drain({ activeStyleContexts: sparse })).rejects.toThrow(/data element/);
    const arrayAccessor: unknown[] = [];
    Object.defineProperty(arrayAccessor, "0", { get: () => "screen" });
    Object.defineProperty(arrayAccessor, "length", { value: 1 });
    await expect(drain({ activeStyleContexts: arrayAccessor })).rejects.toThrow(/data element/);
    await expect(drain({ activeStyleContexts: ["screen"] })).resolves.toBeUndefined();
    const cycle: any = {};
    cycle.self = cycle;
    await expect(drain({ style: cycle })).rejects.toThrow(/cycles/);
    await expect(drain({ style: new Date() })).rejects.toThrow(/plain objects/);
    await expect(drain({ ["x".repeat(257)]: true })).rejects.toThrow(/unsafe field name/);
    const namedArray = ["screen"];
    Object.defineProperty(namedArray, "x".repeat(33), { enumerable: true, value: true });
    await expect(drain({ activeStyleContexts: namedArray })).rejects.toThrow(/unsafe array property/);
  });

  it("rejects malformed ContentNode and Document AST shapes at the boundary", async () => {
    const drain = async (input: unknown) => {
      for await (const _ of renderTerminal.run(values([input]) as never, {} as never, context()) as AsyncIterable<unknown>) { /* drain */ }
    };
    const invalid: unknown[] = [
      { ...node(), identity: "bad" },
      { ...node(), revision: "bad" },
      { ...node(), route: 3 },
      { ...node(), sourcePath: 3 },
      { ...node(), frontmatter: [] },
      { ...node(), frontmatter: { value: NaN } },
      { ...node(), assetRefs: [{ ...node().assetRefs[0], id: "bad" }] },
      { ...node(), assetRefs: [{ ...node().assetRefs[0], nodePath: [-1] }] },
      { ...node(), assetRefs: [{ ...node().assetRefs[0], role: "bad" }] },
      { ...node(), assetRefs: [{ ...node().assetRefs[0], sourcePath: 4 }] },
      { ...node(), document: { type: "unknown" } },
      { ...node(), document: { type: "heading", level: 9, children: [] } },
      { ...node(), document: { type: "table", align: ["diagonal"], children: [] } },
      { ...node(), document: { type: "autolink", destination: "x", isEmail: "yes" } },
    ];
    for (const value of invalid) await expect(drain(value)).rejects.toThrow(/forme-render-terminal/);
    await expect(drain({ ...node(), frontmatter: { ["x".repeat(257)]: true } })).rejects.toThrow(/unsafe field name/);
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

  it("does not count removed script or style elements as island fallbacks", async () => {
    const scripted = { ...node(), document: { type: "document", children: [
      { type: "raw_block", format: "html", value: "<script id=\"fallback\">bad()</script>" },
    ] } } as ContentNode;
    const drain = async () => {
      for await (const _ of renderTerminal.run(values([scripted]) as never, { interactivity } as never, context()) as AsyncIterable<unknown>) { /* drain */ }
    };
    await expect(drain()).rejects.toThrow(/fallback id.*0 matches/);
  });

  it("snapshots hostile content without invoking accessors and strips presentation controls", async () => {
    const getter = { ...node() } as Record<string, unknown>;
    Object.defineProperty(getter, "route", { enumerable: true, get: () => { throw new Error("must not run"); } });
    const drain = async (input: unknown) => {
      for await (const _ of renderTerminal.run(values([input]) as never, {} as never, context()) as AsyncIterable<unknown>) { /* drain */ }
    };
    await expect(drain(getter)).rejects.toThrow(/accessor/);
    await expect(drain(new Proxy(node(), {}))).rejects.toThrow(/descriptor-safe JSON/);

    const controlled = { ...node(), assetRefs: [], document: { type: "document", children: [
      { type: "text", value: "before\roverwrite\u202Eafter\u2066" },
    ] } } as ContentNode;
    const results: TerminalBuffer[] = [];
    for await (const item of renderTerminal.run(values([controlled]) as never, {} as never, context()) as AsyncIterable<TerminalBuffer>) results.push(item);
    expect(results[0]!.text).toBe("before\noverwriteafter\n");
    const rawFormat = { ...node(), assetRefs: [], document: { type: "document", children: [
      { type: "raw_block", format: "ht\u001bml\u202E", value: "ignored" },
    ] } } as ContentNode;
    const rawResults: TerminalBuffer[] = [];
    for await (const item of renderTerminal.run(values([rawFormat]) as never, {} as never, context()) as AsyncIterable<TerminalBuffer>) rawResults.push(item);
    expect(rawResults[0]!.degradations[0]).toMatchObject({ format: "html", message: expect.not.stringMatching(/[\u001b\u202e]/) });
  });

  it("binds output provenance to style and terminal bytes", async () => {
    const plain: TerminalBuffer[] = [];
    const styled: TerminalBuffer[] = [];
    for await (const item of renderTerminal.run(values([node()]) as never, {} as never, context()) as AsyncIterable<TerminalBuffer>) plain.push(item);
    for await (const item of renderTerminal.run(values([node()]) as never, { style: theme } as never, context()) as AsyncIterable<TerminalBuffer>) styled.push(item);
    expect(styled[0]!.text).not.toBe(plain[0]!.text);
    expect(styled[0]!.revision).not.toBe(plain[0]!.revision);
    expect(styled[0]!.provenance).toEqual(plain[0]!.provenance);
    const plainArtifact = await packageTerminal.run(values(plain) as never, {} as never, context()) as any;
    const styledArtifact = await packageTerminal.run(values(styled) as never, {} as never, context()) as any;
    expect(styledArtifact.manifest.buildId).not.toBe(plainArtifact.manifest.buildId);
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
      revision: REV, provenance: PROVENANCE,
    };
    await expect(packageTerminal.run(values([buffer, buffer]) as never, {} as never, context())).rejects.toThrow(/duplicate route/);
    await expect(packageTerminal.run(values([buffer]) as never, { root: "../escape" } as never, context())).rejects.toThrow(/root/);
    await expect(packageTerminal.run(values([{ ...buffer, route: "/../escape" }]) as never, {} as never, context())).rejects.toThrow(/unsafe/);
    await expect(packageTerminal.run(values([{ ...buffer, route: "/CON" }]) as never, {} as never, context())).rejects.toThrow(/unsafe portable path/);
    await expect(packageTerminal.run(values([{ ...buffer, route: "/trail." }]) as never, {} as never, context())).rejects.toThrow(/unsafe portable path/);
    await expect(packageTerminal.run(values([{ ...buffer, route: `/${"x".repeat(240)}` }]) as never, {} as never, context())).rejects.toThrow(/unsafe portable path/);
    await expect(packageTerminal.run(values([{ ...buffer, route: "/Same" }, { ...buffer, route: "/same" }]) as never, {} as never, context())).rejects.toThrow(/portable path collision/);
    await expect(packageTerminal.run(values([{ ...buffer, route: "/foo" }, { ...buffer, route: "/foo.ansi/bar" }]) as never, {} as never, context())).rejects.toThrow(/portable path collision/);
    await expect(packageTerminal.run(values([buffer]) as never, { root: "NUL" } as never, context())).rejects.toThrow(/unsafe portable path/);
    const longRoot = Array(10).fill("r".repeat(203)).join("/");
    await expect(packageTerminal.run(values([buffer]) as never, { root: longRoot } as never, context())).rejects.toThrow(/2048-character/);
    await expect(packageTerminal.run(values([buffer]) as never, { extra: true } as never, context())).rejects.toThrow(/unknown/);
  });

  it("snapshots hostile buffers and enforces per-file limits before packaging", async () => {
    const buffer: TerminalBuffer = {
      route: "/safe", text: "x", usedStyle: [], usedAssets: [], degradations: [],
      revision: REV, provenance: PROVENANCE,
    };
    const hostile: any = { ...buffer, degradations: [{ code: "raw-node-dropped", format: "html", nodePath: [], message: "x" }] };
    Object.defineProperty(hostile.degradations[0], "toJSON", { enumerable: true, value: () => { throw new Error("must not run"); } });
    await expect(packageTerminal.run(values([hostile]) as never, {} as never, context())).rejects.toThrow(/descriptor-safe JSON/);
    await expect(packageTerminal.run(values([{ ...buffer, text: "x".repeat(8_388_609) }]) as never, {} as never, context())).rejects.toThrow(/8 MiB limit/);
    await expect(packageTerminal.run(values([{ ...buffer, usedStyle: [3] }]) as never, {} as never, context())).rejects.toThrow(/usedStyle/);
    await expect(packageTerminal.run(values([{ ...buffer, usedAssets: ["bad"] }]) as never, {} as never, context())).rejects.toThrow(/usedAssets/);
    await expect(packageTerminal.run(values([{ ...buffer, degradations: [{ code: "unknown", message: "x" }] }]) as never, {} as never, context())).rejects.toThrow(/code is unknown/);
    await expect(packageTerminal.run(values([{ ...buffer, provenance: { ...buffer.provenance, revision: "bad" } }]) as never, {} as never, context())).rejects.toThrow(/revision is invalid/);
    await expect(packageTerminal.run(values([{ ...buffer, provenance: { contributors: PROVENANCE.contributors, revision: REV } }]) as never, {} as never, context())).rejects.toThrow(/not canonical/);
    await expect(packageTerminal.run(values([{ ...buffer, text: "safe\u001b]52;c;bad\u0007" }]) as never, {} as never, context())).rejects.toThrow(/non-SGR/);
    await expect(packageTerminal.run(values([{ ...buffer, text: "safe\rspoof" }]) as never, {} as never, context())).rejects.toThrow(/presentation controls/);
    await expect(packageTerminal.run(values([{ ...buffer, text: "\ud800" }]) as never, {} as never, context())).rejects.toThrow(/malformed Unicode/);
    let diagnostic = "";
    try {
      await packageTerminal.run(values([{ ...buffer, route: `/${"x".repeat(5_000)}\u001b\u202e` }]) as never, {} as never, context());
    } catch (error) {
      diagnostic = (error as Error).message;
    }
    expect(diagnostic.length).toBeLessThan(512);
    expect(diagnostic).toMatch(/unsafe route/);
    expect(diagnostic).not.toMatch(/[\u001b\u202e]/);
  });

  it("supports an empty default-root artifact", async () => {
    const artifact = await packageTerminal.run(values([]) as never, {} as never, context()) as any;
    expect(artifact.files).toEqual({});
    expect(artifact.manifest.routes).toEqual([]);
  });
});
