import { useMemo, useState } from "react";
import { MacsymaSession, type EvalResult, toDisplayString } from "@coding-adventures/macsyma-runtime";

const examples = [
  { label: "Arithmetic", source: "1 + 2 * 3;\n(2 + 3)^2;" },
  { label: "Bindings", source: "x : 5$\ny : x^2 + 1;\ny + 10;" },
  { label: "History", source: "2 + 3;\n% * 2;\n%i1;\n%o2;" },
  { label: "Functions", source: "f(x) := x^2 + 1;\nf(4);" },
] as const;

interface TranscriptEntry {
  readonly inputIndex: number;
  readonly outputIndex: number;
  readonly input: string;
  readonly output: string;
  readonly display: boolean;
}

export function App(): JSX.Element {
  const session = useMemo(() => new MacsymaSession(), []);
  const [source, setSource] = useState(examples[0].source);
  const [transcript, setTranscript] = useState<TranscriptEntry[]>([]);
  const [error, setError] = useState<string | null>(null);

  const visibleCount = transcript.filter((entry) => entry.display).length;

  function runSource(): void {
    try {
      const results = session.evalSource(source);
      setTranscript((current) => [...current, ...results.map(toEntry)]);
      setError(null);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }

  function reset(): void {
    session.resetHistory();
    setTranscript([]);
    setError(null);
  }

  return (
    <main className="shell">
      <section className="workbench" aria-label="MACSYMA browser REPL">
        <header className="topbar">
          <div>
            <h1>MACSYMA</h1>
            <p>Pure TypeScript symbolic session</p>
          </div>
          <dl className="counters" aria-label="Session counters">
            <div>
              <dt>Inputs</dt>
              <dd>{session.history().nextInputIndex() - 1}</dd>
            </div>
            <div>
              <dt>Visible</dt>
              <dd>{visibleCount}</dd>
            </div>
          </dl>
        </header>

        <div className="mainGrid">
          <section className="editorPane" aria-label="Input editor">
            <div className="exampleBar" aria-label="Examples">
              {examples.map((example) => (
                <button
                  key={example.label}
                  className="exampleButton"
                  type="button"
                  onClick={() => setSource(example.source)}
                >
                  {example.label}
                </button>
              ))}
            </div>
            <textarea
              aria-label="MACSYMA source"
              spellCheck={false}
              value={source}
              onChange={(event) => setSource(event.target.value)}
            />
            <div className="actions">
              <button className="primary" type="button" onClick={runSource}>
                Run
              </button>
              <button type="button" onClick={reset}>
                Reset
              </button>
            </div>
            {error !== null ? (
              <output className="error" role="alert">
                {error}
              </output>
            ) : null}
          </section>

          <section className="transcriptPane" aria-label="Transcript">
            {transcript.length === 0 ? (
              <div className="emptyState">(%i1)</div>
            ) : (
              <ol>
                {transcript.map((entry) => (
                  <li key={`${entry.inputIndex}:${entry.outputIndex}`} className={entry.display ? "" : "suppressed"}>
                    <div className="prompt">%i{entry.inputIndex}</div>
                    <pre>{entry.input}</pre>
                    <div className="prompt">%o{entry.outputIndex}</div>
                    <pre>{entry.display ? entry.output : "$"}</pre>
                  </li>
                ))}
              </ol>
            )}
          </section>
        </div>
      </section>
    </main>
  );
}

function toEntry(result: EvalResult): TranscriptEntry {
  return {
    inputIndex: result.inputIndex,
    outputIndex: result.outputIndex,
    input: toDisplayString(result.input),
    output: toDisplayString(result.output),
    display: result.display,
  };
}
