import { startTransition, useMemo, useRef, useState } from "react";
import {
  Intel4004Simulator,
  type Intel4004State,
  type Intel4004Trace,
} from "@coding-adventures/intel4004-simulator";
import { EXAMPLES, type ExampleProgram } from "./examples.js";
import {
  compileNibToHex,
  loadHexForSimulation,
  WebPipelineError,
  type WebCompileResult,
} from "./pipeline.js";

type CompilePhase = "idle" | "success" | "error";

interface CompileSnapshot {
  readonly phase: CompilePhase;
  readonly result: WebCompileResult | null;
  readonly stage: string | null;
  readonly message: string | null;
}

interface MachineSnapshot {
  readonly loaded: boolean;
  readonly state: Intel4004State;
  readonly traces: readonly Intel4004Trace[];
  readonly note: string;
}

const EMPTY_MACHINE_STATE: Intel4004State = Object.freeze({
  accumulator: 0,
  registers: Object.freeze(new Array(16).fill(0)),
  carry: false,
  pc: 0,
  halted: false,
  ram: Object.freeze([]),
  hwStack: Object.freeze([0, 0, 0]),
  stackPointer: 0,
});

const EMPTY_COMPILE: CompileSnapshot = {
  phase: "idle",
  result: null,
  stage: null,
  message: null,
};

const EMPTY_MACHINE: MachineSnapshot = {
  loaded: false,
  state: EMPTY_MACHINE_STATE,
  traces: [],
  note: "Compile a program to load the simulated Intel 4004.",
};

const MAX_TRACE_ROWS = 40;
const MAX_RUN_STEPS = 10_000;

function toCompileFailure(error: unknown): CompileSnapshot {
  if (error instanceof WebPipelineError) {
    return {
      phase: "error",
      result: null,
      stage: error.stage,
      message: error.message,
    };
  }

  return {
    phase: "error",
    result: null,
    stage: "unknown",
    message: error instanceof Error ? error.message : String(error),
  };
}

function toMachineSnapshot(
  simulator: Intel4004Simulator,
  traces: readonly Intel4004Trace[],
  note: string,
): MachineSnapshot {
  return {
    loaded: true,
    state: simulator.getState(),
    traces: traces.slice(-MAX_TRACE_ROWS),
    note,
  };
}

function formatNibble(value: number): string {
  return `0x${value.toString(16).toUpperCase()}`;
}

function formatAddress(value: number): string {
  return `0x${value.toString(16).toUpperCase().padStart(3, "0")}`;
}

function formatBinary(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((byte) => byte.toString(16).toUpperCase().padStart(2, "0"))
    .join(" ");
}

function formatIr(result: WebCompileResult): string {
  return JSON.stringify(result.optimizedIr, null, 2);
}

export function App() {
  const compilerRef = useRef({ compile: compileNibToHex });
  const simulatorRef = useRef(new Intel4004Simulator());
  const traceBufferRef = useRef<Intel4004Trace[]>([]);

  const [selectedExample, setSelectedExample] = useState(EXAMPLES[0]!.id);
  const [source, setSource] = useState(EXAMPLES[0]!.source);
  const [compileSnapshot, setCompileSnapshot] = useState<CompileSnapshot>(EMPTY_COMPILE);
  const [machineSnapshot, setMachineSnapshot] = useState<MachineSnapshot>(EMPTY_MACHINE);

  const currentExample = useMemo(
    () => EXAMPLES.find((example) => example.id === selectedExample) ?? EXAMPLES[0]!,
    [selectedExample],
  );

  function loadIntoSimulator(hexText: string, note: string): MachineSnapshot {
    const simulator = simulatorRef.current;
    simulator.reset();
    simulator.load(loadHexForSimulation(hexText));
    traceBufferRef.current = [];
    return toMachineSnapshot(simulator, traceBufferRef.current, note);
  }

  function handleExampleChange(example: ExampleProgram): void {
    setSelectedExample(example.id);
    setSource(example.source);
  }

  function handleCompile(): void {
    try {
      const result = compilerRef.current.compile(source);
      const nextMachine = loadIntoSimulator(
        result.hexText,
        `Program loaded at origin ${formatAddress(result.origin)}.`,
      );

      startTransition(() => {
        setCompileSnapshot({
          phase: "success",
          result,
          stage: null,
          message: null,
        });
        setMachineSnapshot(nextMachine);
      });
    } catch (error) {
      startTransition(() => {
        setCompileSnapshot(toCompileFailure(error));
        setMachineSnapshot(EMPTY_MACHINE);
      });
    }
  }

  function handleResetMachine(): void {
    const result = compileSnapshot.result;
    if (result === null) return;
    setMachineSnapshot(loadIntoSimulator(result.hexText, "Simulator reset and ready to step."));
  }

  function handleStep(): void {
    if (compileSnapshot.result === null || machineSnapshot.state.halted) {
      return;
    }

    const simulator = simulatorRef.current;

    try {
      const trace = simulator.step();
      traceBufferRef.current = [...traceBufferRef.current, trace].slice(-MAX_TRACE_ROWS);
      setMachineSnapshot(
        toMachineSnapshot(
          simulator,
          traceBufferRef.current,
          simulator.halted ? "Program halted." : `Stepped ${trace.mnemonic}.`,
        ),
      );
    } catch (error) {
      setMachineSnapshot(
        toMachineSnapshot(
          simulator,
          traceBufferRef.current,
          error instanceof Error ? error.message : String(error),
        ),
      );
    }
  }

  function handleRun(limit: number, label: string): void {
    if (compileSnapshot.result === null || machineSnapshot.state.halted) {
      return;
    }

    const simulator = simulatorRef.current;
    let steps = 0;

    try {
      while (!simulator.halted && steps < limit) {
        const trace = simulator.step();
        traceBufferRef.current = [...traceBufferRef.current, trace].slice(-MAX_TRACE_ROWS);
        steps += 1;
      }

      const note = simulator.halted
        ? `Program halted after ${steps} step${steps === 1 ? "" : "s"}.`
        : `${label} paused after ${steps} step${steps === 1 ? "" : "s"}.`;

      setMachineSnapshot(toMachineSnapshot(simulator, traceBufferRef.current, note));
    } catch (error) {
      setMachineSnapshot(
        toMachineSnapshot(
          simulator,
          traceBufferRef.current,
          error instanceof Error ? error.message : String(error),
        ),
      );
    }
  }

  const result = compileSnapshot.result;
  const registers = machineSnapshot.state.registers;

  return (
    <div className="page-shell">
      <div aria-hidden="true" className="page-shell__glow page-shell__glow--left" />
      <div aria-hidden="true" className="page-shell__glow page-shell__glow--right" />
      <div aria-hidden="true" className="page-shell__grid" />
      <header className="hero">
        <div className="hero__copy">
          <p className="eyebrow">Nib on the Web</p>
          <h1>Compile to the Intel 4004. Inspect every step.</h1>
          <div className="hero__signals" aria-label="Playground highlights">
            <span className="signal-chip">Parser + type checker</span>
            <span className="signal-chip">Intel HEX output</span>
            <span className="signal-chip">Cycle-by-cycle simulation</span>
          </div>
          <p className="hero__lede">
            This playground runs the real Nib parser, type checker, compiler, assembler,
            and Intel 4004 simulator in the browser. Write code, compile it to Intel HEX,
            then watch the machine state move.
          </p>
          <div className="hero__rail" aria-label="Compiler stages">
            <span>Source</span>
            <span>IR</span>
            <span>Assembly</span>
            <span>HEX</span>
            <span>Machine</span>
          </div>
        </div>

        <div className="hero__meta">
          <div className="stat-card stat-card--target">
            <span className="stat-card__label">Target</span>
            <strong>Intel 4004</strong>
          </div>
          <div className="stat-card stat-card--path">
            <span className="stat-card__label">Compiler Path</span>
            <strong>Nib → IR → 4004 → HEX</strong>
          </div>
          <div className="stat-card stat-card--runtime">
            <span className="stat-card__label">Runtime</span>
            <strong>{machineSnapshot.loaded ? "Simulator Loaded" : "Awaiting Compile"}</strong>
          </div>
        </div>
      </header>

      <main className="workspace">
        <section className="panel panel--editor">
          <div className="panel__header">
            <div>
              <p className="panel__eyebrow">Source</p>
              <h2>Nib Playground</h2>
            </div>
            <div className="example-picker">
              <label htmlFor="example-select">Example</label>
              <select
                id="example-select"
                value={currentExample.id}
                onChange={(event) => {
                  const example = EXAMPLES.find((item) => item.id === event.target.value);
                  if (example !== undefined) {
                    handleExampleChange(example);
                  }
                }}
              >
                {EXAMPLES.map((example) => (
                  <option key={example.id} value={example.id}>
                    {example.name}
                  </option>
                ))}
              </select>
            </div>
          </div>

          <p className="example-summary">{currentExample.summary}</p>

          <label className="sr-only" htmlFor="nib-source">Nib Source</label>
          <textarea
            id="nib-source"
            className="editor"
            aria-label="Nib Source"
            spellCheck={false}
            value={source}
            onChange={(event) => setSource(event.target.value)}
          />

          <div className="action-row">
            <button className="button button--primary" onClick={handleCompile}>
              Compile to Intel 4004
            </button>
            <button
              className="button"
              onClick={handleResetMachine}
              disabled={compileSnapshot.result === null}
            >
              Reset Machine
            </button>
            <button
              className="button"
              onClick={handleStep}
              disabled={compileSnapshot.result === null || machineSnapshot.state.halted}
            >
              Step
            </button>
            <button
              className="button"
              onClick={() => handleRun(25, "Burst run")}
              disabled={compileSnapshot.result === null || machineSnapshot.state.halted}
            >
              Run 25
            </button>
            <button
              className="button"
              onClick={() => handleRun(MAX_RUN_STEPS, "Full run")}
              disabled={compileSnapshot.result === null || machineSnapshot.state.halted}
            >
              Run to Halt
            </button>
          </div>
        </section>

        <section className="panel panel--results">
          <div className="panel__header">
            <div>
              <p className="panel__eyebrow">Compiler</p>
              <h2>Outputs</h2>
            </div>
            <div className={`status-pill status-pill--${compileSnapshot.phase}`}>
              {compileSnapshot.phase === "success" && "Compilation succeeded"}
              {compileSnapshot.phase === "error" && "Compilation failed"}
              {compileSnapshot.phase === "idle" && "Ready to compile"}
            </div>
          </div>

          <section className="card">
            <h3>Diagnostics</h3>
            <div aria-label="Diagnostics panel" className="diagnostics">
              {compileSnapshot.phase === "idle" && (
                <p>Compile a Nib program to see frontend or backend diagnostics.</p>
              )}
              {compileSnapshot.phase === "error" && (
                <>
                  <p className="diagnostics__stage">{compileSnapshot.stage}</p>
                  <pre>{compileSnapshot.message}</pre>
                </>
              )}
              {compileSnapshot.phase === "success" && (
                <p>No errors. Compiler pipeline completed through Intel HEX packaging.</p>
              )}
            </div>
          </section>

          <div className="artifact-grid">
            <section className="card card--hex">
              <h3>Intel HEX</h3>
              <pre aria-label="Intel HEX output" className="artifact-block">
                {result?.hexText ?? "HEX output will appear here after a successful compile."}
              </pre>
            </section>

            <section className="card card--assembly">
              <h3>Assembly</h3>
              <pre className="artifact-block">
                {result?.assembly ?? "Intel 4004 assembly will appear here."}
              </pre>
            </section>

            <section className="card card--ir">
              <h3>Optimized IR</h3>
              <pre className="artifact-block">
                {result !== null ? formatIr(result) : "Optimized IR will appear here."}
              </pre>
            </section>

            <section className="card card--binary">
              <h3>Binary Bytes</h3>
              <pre className="artifact-block">
                {result !== null ? formatBinary(result.binary) : "Compiled program bytes will appear here."}
              </pre>
            </section>
          </div>
        </section>

        <section className="panel panel--machine">
          <div className="panel__header">
            <div>
              <p className="panel__eyebrow">Simulator</p>
              <h2>Intel 4004 State</h2>
            </div>
            <div className={`machine-badge ${machineSnapshot.state.halted ? "machine-badge--halted" : "machine-badge--running"}`}>
              {machineSnapshot.state.halted ? "HALTED" : machineSnapshot.loaded ? "READY" : "IDLE"}
            </div>
          </div>

          <p className="machine-note">{machineSnapshot.note}</p>

          <div className="machine-summary">
            <div className="chip">
              <span>A</span>
              <strong>{formatNibble(machineSnapshot.state.accumulator)}</strong>
            </div>
            <div className="chip">
              <span>Carry</span>
              <strong>{machineSnapshot.state.carry ? "1" : "0"}</strong>
            </div>
            <div className="chip">
              <span>PC</span>
              <strong>{formatAddress(machineSnapshot.state.pc)}</strong>
            </div>
            <div className="chip">
              <span>Stack Ptr</span>
              <strong>{machineSnapshot.state.stackPointer}</strong>
            </div>
            <div className="chip">
              <span>Program Size</span>
              <strong>{result?.binary.length ?? 0} bytes</strong>
            </div>
          </div>

          <div className="machine-grid">
            <section className="card">
              <h3>Registers</h3>
              <div className="register-grid">
                {registers.map((value, index) => (
                  <div key={index} className={`register-cell register-cell--${index}`}>
                    <span>{`R${index}`}</span>
                    <strong>{formatNibble(value)}</strong>
                  </div>
                ))}
              </div>
            </section>

            <section className="card">
              <h3>Hardware Stack</h3>
              <div className="stack-grid">
                {machineSnapshot.state.hwStack.map((value, index) => (
                  <div key={index} className="register-cell">
                    <span>{`L${index}`}</span>
                    <strong>{formatAddress(value)}</strong>
                  </div>
                ))}
              </div>
            </section>
          </div>

          <section className="card">
            <h3>Execution Trace</h3>
            <div aria-label="Execution trace" className="trace-list">
              {machineSnapshot.traces.length === 0 && (
                <p className="trace-empty">No instructions executed yet.</p>
              )}
              {[...machineSnapshot.traces].reverse().map((trace) => (
                <div key={`${trace.address}-${trace.mnemonic}-${trace.raw}`} className="trace-row">
                  <span className="trace-row__address">{formatAddress(trace.address)}</span>
                  <span className="trace-row__mnemonic">{trace.mnemonic}</span>
                  <span className="trace-row__delta">
                    A {formatNibble(trace.accumulatorBefore)} → {formatNibble(trace.accumulatorAfter)}
                  </span>
                </div>
              ))}
            </div>
          </section>
        </section>
      </main>
    </div>
  );
}
