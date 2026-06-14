import { describe, expect, it } from "vitest";
import { MacsymaWasmRuntime, loadMacsymaWasmRuntime, parseEvalResponse, type MacsymaWasmGeneratedModule } from "../src/index";

describe("MacsymaWasmRuntime", () => {
  it("loads generated wasm-bindgen modules and runs default init once", async () => {
    const calls: string[] = [];
    const runtime = await loadMacsymaWasmRuntime(
      () => ({
        default: (input?: unknown) => {
          calls.push(`init:${String(input)}`);
        },
        WasmMacsymaSession: fakeSessionClass("7"),
        evalSource: () => fakeEvalJson("fresh"),
      }),
      { initInput: "/pkg/macsyma_runtime_wasm_bg.wasm" },
    );

    expect(calls).toEqual(["init:/pkg/macsyma_runtime_wasm_bg.wasm"]);
    expect(runtime.eval("1 + 2 * 3;").visibleOutputs).toEqual(["7"]);
  });

  it("keeps stateful session calls separate from fresh eval calls", () => {
    const runtime = new MacsymaWasmRuntime(fakeModule("6", "fresh"));

    expect(runtime.eval("x : 5$\nx + 1;").results[0].outputMacsyma).toBe("6");
    expect(runtime.evalFresh("2 + 2;").results[0].outputMacsyma).toBe("fresh");
  });

  it("normalizes snake_case Rust JSON into camelCase TypeScript values", () => {
    const response = parseEvalResponse(fakeEvalJson("x + 1"));

    expect(response.ok).toBe(true);
    expect(response.results[0]).toMatchObject({
      inputIndex: 1,
      outputIndex: 1,
      inputMacsyma: "x",
      outputMacsyma: "x + 1",
      outputLisp: "(Add x 1)",
    });
    expect(response.results[0].outputIr).toEqual({
      kind: "apply",
      head: { kind: "symbol", name: "Add" },
      args: [
        { kind: "symbol", name: "x" },
        { kind: "integer", value: 1 },
      ],
    });
    expect(response.history.lastOutputMacsyma).toBe("x + 1");
  });

  it("surfaces compile errors without losing history counters", () => {
    const response = parseEvalResponse(
      JSON.stringify({
        ok: false,
        results: [],
        visible_outputs: [],
        history: {
          input_count: 2,
          output_count: 2,
          next_input_index: 3,
          last_output_macsyma: "7",
          last_output_lisp: "7",
        },
        error: { kind: "compile", message: "parse error" },
      }),
    );

    expect(response.ok).toBe(false);
    expect(response.error?.message).toBe("parse error");
    expect(response.history.nextInputIndex).toBe(3);
  });

  it("rejects malformed bridge payloads early", () => {
    expect(() => parseEvalResponse(JSON.stringify({ ok: true }))).toThrow("Expected results to be an array");
  });
});

function fakeModule(sessionOutput: string, freshOutput: string): MacsymaWasmGeneratedModule {
  return {
    WasmMacsymaSession: fakeSessionClass(sessionOutput),
    evalSource: () => fakeEvalJson(freshOutput),
  };
}

function fakeSessionClass(output: string): new () => { eval(source: string): string; historyJson(): string; resetHistory(): void } {
  return class {
    eval(_source: string): string {
      return fakeEvalJson(output);
    }

    historyJson(): string {
      return JSON.stringify({
        ok: true,
        history: {
          input_count: 1,
          output_count: 1,
          next_input_index: 2,
          last_output_macsyma: output,
          last_output_lisp: output,
        },
      });
    }

    resetHistory(): void {}
  };
}

function fakeEvalJson(outputMacsyma: string): string {
  return JSON.stringify({
    ok: true,
    results: [
      {
        input_index: 1,
        output_index: 1,
        display: true,
        input_macsyma: "x",
        output_macsyma: outputMacsyma,
        input_lisp: "x",
        output_lisp: "(Add x 1)",
        input_ir: { kind: "symbol", name: "x" },
        output_ir: {
          kind: "apply",
          head: { kind: "symbol", name: "Add" },
          args: [
            { kind: "symbol", name: "x" },
            { kind: "integer", value: 1 },
          ],
        },
      },
    ],
    visible_outputs: [outputMacsyma],
    history: {
      input_count: 1,
      output_count: 1,
      next_input_index: 2,
      last_output_macsyma: outputMacsyma,
      last_output_lisp: "(Add x 1)",
    },
  });
}
