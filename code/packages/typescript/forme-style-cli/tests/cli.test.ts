/**
 * cli.test.ts — exercise the CLI via the testable `run(argv, io)`
 * entry point.  No subprocesses; all I/O is wired to in-memory
 * buffers.
 */

import { describe, it, expect } from "vitest";
import {
  run,
  EXIT_OK, EXIT_VALIDATOR_FAIL, EXIT_IO_OR_ARG_ERROR,
  type CliIO,
} from "../src/index.js";

// ─── In-memory IO scaffolding ────────────────────────────────────────────

interface MockIO extends CliIO {
  stdoutBuffer: string;
  stderrBuffer: string;
}

function makeIO(opts: {
  files?: Record<string, string>;
  stdin?: string;
  writeFailures?: ReadonlySet<string>;
} = {}): MockIO {
  const files = new Map<string, string>(Object.entries(opts.files ?? {}));
  const writeFailures = opts.writeFailures ?? new Set<string>();
  const written: Map<string, string> = new Map();
  const io: MockIO = {
    stdoutBuffer: "",
    stderrBuffer: "",
    stdout: { write: (s: string) => { io.stdoutBuffer += s; } },
    stderr: { write: (s: string) => { io.stderrBuffer += s; } },
    readFile: async (p: string) => {
      const v = files.get(p);
      if (v === undefined) throw new Error(`ENOENT: ${p}`);
      return v;
    },
    writeFile: async (p: string, contents: string) => {
      if (writeFailures.has(p)) throw new Error(`EACCES: ${p}`);
      written.set(p, contents);
    },
    readStdin: async () => opts.stdin ?? "",
  };
  // Expose written-files for assertion.
  (io as unknown as { written: Map<string, string> }).written = written;
  return io;
}

// ─── Fixtures ────────────────────────────────────────────────────────────

const DOC = {
  kind: "StyleDocument",
  tokens: {
    colors: { text: { kind: "rgb", r: 31, g: 35, b: 40 } },
    typography: {
      families: { body: ["Inter"] },
      scale:    { md: { unit: "pt", value: 12 } },
      weights:  { regular: 400 },
      leading:  { normal: 1.5 },
      tracking: { normal: { unit: "em", value: 0 } },
    },
    space:   {},
    radii:   {},
    shadows: {},
  },
  rules: [
    {
      id: "body",
      selector: { kind: "node-type", type: "paragraph" },
      properties: [
        { kind: "color", value: { kind: "token-ref", path: "colors.text" } },
      ],
    },
  ],
  contexts: [],
  theme: null,
};

const DOC_JSON = JSON.stringify(DOC);

// ─── Tests ───────────────────────────────────────────────────────────────

describe("run — help", () => {
  it("--help prints usage and exits 0", async () => {
    const io = makeIO();
    const code = await run(["--help"], io);
    expect(code).toBe(EXIT_OK);
    expect(io.stdoutBuffer).toContain("forme-style");
    expect(io.stdoutBuffer).toContain("--target");
  });

  it("no args prints usage and exits 0 (help is the default)", async () => {
    const io = makeIO();
    const code = await run([], io);
    expect(code).toBe(EXIT_OK);
    expect(io.stdoutBuffer).toContain("Usage:");
  });

  it("-h is an alias for --help", async () => {
    const io = makeIO();
    const code = await run(["-h"], io);
    expect(code).toBe(EXIT_OK);
    expect(io.stdoutBuffer).toContain("Usage:");
  });
});

describe("run — happy path (each target)", () => {
  it("CSS dispatch writes to stdout", async () => {
    const io = makeIO({ files: { "/doc.json": DOC_JSON } });
    const code = await run(["/doc.json", "--target", "css"], io);
    expect(code).toBe(EXIT_OK);
    expect(io.stdoutBuffer).toContain("paragraph {");
    expect(io.stdoutBuffer).toContain("color: rgb(31 35 40)");
  });

  it("LaTeX dispatch writes to stdout", async () => {
    const io = makeIO({ files: { "/doc.json": DOC_JSON } });
    const code = await run(["/doc.json", "--target", "latex"], io);
    expect(code).toBe(EXIT_OK);
    expect(io.stdoutBuffer).toContain("\\newcommand{\\formeNodeParagraph}");
    expect(io.stdoutBuffer).toContain("\\color{RGB}{31,35,40}");
  });

  it("terminal dispatch writes to stdout", async () => {
    const io = makeIO({ files: { "/doc.json": DOC_JSON } });
    const code = await run(["/doc.json", "--target", "terminal"], io);
    expect(code).toBe(EXIT_OK);
    expect(io.stdoutBuffer).toContain("formeStyles");
    expect(io.stdoutBuffer).toContain("38;2;31;35;40");
  });
});

describe("run — stdin / stdout streaming", () => {
  it("reads from stdin when input is '-'", async () => {
    const io = makeIO({ stdin: DOC_JSON });
    const code = await run(["-", "--target", "css"], io);
    expect(code).toBe(EXIT_OK);
    expect(io.stdoutBuffer).toContain("color: rgb(31 35 40)");
  });

  it("writes to file when --out is given", async () => {
    const io = makeIO({ files: { "/doc.json": DOC_JSON } });
    const code = await run(["/doc.json", "--target", "css", "--out", "/out.css"], io);
    expect(code).toBe(EXIT_OK);
    const written = (io as unknown as { written: Map<string, string> }).written;
    expect(written.get("/out.css")).toContain("color: rgb(31 35 40)");
    // stdout receives nothing when --out is used.
    expect(io.stdoutBuffer).toBe("");
  });
});

describe("run — themes", () => {
  it("loads themes file and applies named theme", async () => {
    const themesJson = JSON.stringify({
      themes: [
        { name: "dark", tokens: { colors: { text: { kind: "named", name: "white" } } } },
      ],
    });
    const io = makeIO({
      files: { "/doc.json": DOC_JSON, "/themes.json": themesJson },
    });
    const code = await run([
      "/doc.json", "--target", "css",
      "--themes", "/themes.json", "--theme", "dark",
    ], io);
    expect(code).toBe(EXIT_OK);
    expect(io.stdoutBuffer).toContain("color: white");
  });

  it("warns when theme name missing from registry; continues with base", async () => {
    const themesJson = JSON.stringify({ themes: [] });
    const io = makeIO({
      files: { "/doc.json": DOC_JSON, "/themes.json": themesJson },
    });
    const code = await run([
      "/doc.json", "--target", "css",
      "--themes", "/themes.json", "--theme", "nonexistent",
    ], io);
    expect(code).toBe(EXIT_OK);
    expect(io.stderrBuffer).toContain("THEME_NOT_FOUND");
    expect(io.stdoutBuffer).toContain("color: rgb(31 35 40)");
  });

  it("rejects --theme without --themes (arg parse error)", async () => {
    const io = makeIO({ files: { "/doc.json": DOC_JSON } });
    const code = await run([
      "/doc.json", "--target", "css", "--theme", "dark",
    ], io);
    expect(code).toBe(EXIT_IO_OR_ARG_ERROR);
    expect(io.stderrBuffer).toContain("--theme");
    expect(io.stderrBuffer).toContain("--themes");
  });

  it("rejects malformed themes file", async () => {
    const io = makeIO({
      files: { "/doc.json": DOC_JSON, "/themes.json": "not json at all" },
    });
    const code = await run([
      "/doc.json", "--target", "css",
      "--themes", "/themes.json", "--theme", "dark",
    ], io);
    expect(code).toBe(EXIT_IO_OR_ARG_ERROR);
    expect(io.stderrBuffer).toContain("--themes");
  });

  it("rejects themes file with non-object top level", async () => {
    const themesJson = JSON.stringify([{ name: "dark" }]); // array, not object
    const io = makeIO({
      files: { "/doc.json": DOC_JSON, "/themes.json": themesJson },
    });
    const code = await run([
      "/doc.json", "--target", "css",
      "--themes", "/themes.json", "--theme", "dark",
    ], io);
    expect(code).toBe(EXIT_IO_OR_ARG_ERROR);
  });

  it("rejects themes file missing the `themes` array", async () => {
    const themesJson = JSON.stringify({ wat: [] });
    const io = makeIO({
      files: { "/doc.json": DOC_JSON, "/themes.json": themesJson },
    });
    const code = await run([
      "/doc.json", "--target", "css",
      "--themes", "/themes.json", "--theme", "dark",
    ], io);
    expect(code).toBe(EXIT_IO_OR_ARG_ERROR);
  });

  it("skips malformed theme entries silently and continues with valid ones", async () => {
    const themesJson = JSON.stringify({
      themes: [
        null,                          // skipped (non-object)
        { /* no name */ },             // skipped (no name)
        { name: "" },                  // skipped (empty name)
        { name: "__proto__" },         // skipped (forbidden name)
        { name: "dark", tokens: { colors: { text: { kind: "named", name: "white" } } } },
      ],
    });
    const io = makeIO({
      files: { "/doc.json": DOC_JSON, "/themes.json": themesJson },
    });
    const code = await run([
      "/doc.json", "--target", "css",
      "--themes", "/themes.json", "--theme", "dark",
    ], io);
    expect(code).toBe(EXIT_OK);
    expect(io.stdoutBuffer).toContain("color: white");
  });
});

describe("run — context / used / scope pass-through", () => {
  it("--active filters context-tagged rules", async () => {
    const docWithCtx = {
      ...DOC,
      rules: [
        ...DOC.rules,
        {
          id: "print-only",
          selector: { kind: "node-type", type: "paragraph" },
          properties: [{ kind: "color", value: { kind: "named", name: "black" } }],
          context: "print",
        },
      ],
    };
    const io = makeIO({ files: { "/doc.json": JSON.stringify(docWithCtx) } });
    const screenCode = await run(["/doc.json", "--target", "css", "--active", "screen"], io);
    expect(screenCode).toBe(EXIT_OK);
    expect(io.stdoutBuffer).not.toContain("@media print");

    const io2 = makeIO({ files: { "/doc.json": JSON.stringify(docWithCtx) } });
    const printCode = await run(["/doc.json", "--target", "css", "--active", "print"], io2);
    expect(printCode).toBe(EXIT_OK);
    expect(io2.stdoutBuffer).toContain("@media print");
  });

  it("--used slices output", async () => {
    const docWithTwo = {
      ...DOC,
      rules: [
        ...DOC.rules,
        {
          id: "extra",
          selector: { kind: "node-type", type: "heading" },
          properties: [{ kind: "color", value: { kind: "named", name: "black" } }],
        },
      ],
    };
    const io = makeIO({ files: { "/doc.json": JSON.stringify(docWithTwo) } });
    const code = await run(["/doc.json", "--target", "css", "--used", "extra"], io);
    expect(code).toBe(EXIT_OK);
    expect(io.stdoutBuffer).toContain("heading");
    expect(io.stdoutBuffer).not.toContain("paragraph");
  });

  it("--active with empty / whitespace entries ignores them", async () => {
    // `--active ',,,'` should parse as empty.
    const io = makeIO({ files: { "/doc.json": DOC_JSON } });
    const code = await run(["/doc.json", "--target", "css", "--active", ",, ,"], io);
    expect(code).toBe(EXIT_OK);
  });

  it("--scope is passed through to the CSS translator", async () => {
    const io = makeIO({ files: { "/doc.json": DOC_JSON } });
    const code = await run(["/doc.json", "--target", "css", "--scope", ".page"], io);
    expect(code).toBe(EXIT_OK);
    expect(io.stdoutBuffer).toContain(".page paragraph");
  });
});

describe("run — exit codes", () => {
  it("validator rejection exits 1 with errors on stderr", async () => {
    const io = makeIO({ files: { "/bad.json": "null" } });
    const code = await run(["/bad.json", "--target", "css"], io);
    expect(code).toBe(EXIT_VALIDATOR_FAIL);
    expect(io.stderrBuffer).toContain("validator rejected");
  });

  it("missing input file exits 2", async () => {
    const io = makeIO();
    const code = await run(["/nope.json", "--target", "css"], io);
    expect(code).toBe(EXIT_IO_OR_ARG_ERROR);
    expect(io.stderrBuffer).toContain("failed to read input");
  });

  it("invalid JSON input exits 2", async () => {
    const io = makeIO({ files: { "/bad.json": "{ not json" } });
    const code = await run(["/bad.json", "--target", "css"], io);
    expect(code).toBe(EXIT_IO_OR_ARG_ERROR);
    expect(io.stderrBuffer).toContain("valid JSON");
  });

  it("missing --target exits 2", async () => {
    const io = makeIO({ files: { "/doc.json": DOC_JSON } });
    const code = await run(["/doc.json"], io);
    expect(code).toBe(EXIT_IO_OR_ARG_ERROR);
    expect(io.stderrBuffer).toContain("--target");
  });

  it("unknown --target exits 2 with the expected error message", async () => {
    const io = makeIO({ files: { "/doc.json": DOC_JSON } });
    const code = await run(["/doc.json", "--target", "pdf"], io);
    expect(code).toBe(EXIT_IO_OR_ARG_ERROR);
    expect(io.stderrBuffer).toContain("pdf");
  });

  it("unknown flag exits 2", async () => {
    const io = makeIO({ files: { "/doc.json": DOC_JSON } });
    const code = await run(["/doc.json", "--target", "css", "--wat", "x"], io);
    expect(code).toBe(EXIT_IO_OR_ARG_ERROR);
    expect(io.stderrBuffer).toContain("--wat");
  });

  it("too many positional args exits 2", async () => {
    const io = makeIO({ files: { "/doc.json": DOC_JSON, "/extra.json": DOC_JSON } });
    const code = await run(["/doc.json", "/extra.json", "--target", "css"], io);
    expect(code).toBe(EXIT_IO_OR_ARG_ERROR);
    expect(io.stderrBuffer).toContain("positional");
  });

  it("flag without value exits 2", async () => {
    const io = makeIO({ files: { "/doc.json": DOC_JSON } });
    const code = await run(["/doc.json", "--target"], io);
    expect(code).toBe(EXIT_IO_OR_ARG_ERROR);
    expect(io.stderrBuffer).toContain("requires a value");
  });

  it("flag followed by another flag (value missing) exits 2", async () => {
    const io = makeIO({ files: { "/doc.json": DOC_JSON } });
    const code = await run(["/doc.json", "--target", "--scope", ".p"], io);
    expect(code).toBe(EXIT_IO_OR_ARG_ERROR);
    expect(io.stderrBuffer).toContain("requires a value");
  });

  it("file write failure exits 2", async () => {
    const io = makeIO({
      files: { "/doc.json": DOC_JSON },
      writeFailures: new Set(["/out.css"]),
    });
    const code = await run(["/doc.json", "--target", "css", "--out", "/out.css"], io);
    expect(code).toBe(EXIT_IO_OR_ARG_ERROR);
    expect(io.stderrBuffer).toContain("failed to write");
  });
});

describe("run — warnings are reported on stderr without failing the run", () => {
  it("unresolved token-ref warns but exits 0", async () => {
    const docWithUnresolved = {
      ...DOC,
      rules: [
        {
          id: "bad",
          selector: { kind: "node-type", type: "p" },
          properties: [
            { kind: "color", value: { kind: "token-ref", path: "colors.nope" } },
          ],
        },
      ],
    };
    const io = makeIO({ files: { "/doc.json": JSON.stringify(docWithUnresolved) } });
    const code = await run(["/doc.json", "--target", "css"], io);
    expect(code).toBe(EXIT_OK);
    expect(io.stderrBuffer).toContain("warning");
  });
});
