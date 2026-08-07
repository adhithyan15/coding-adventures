import { useMemo, useState } from "react";
import { traceHopfieldRecall, type BipolarState } from "./hopfield-memory.js";

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  return Number.isInteger(value) ? String(value) : value.toFixed(2).replace(/0+$/, "").replace(/\.$/, "");
}

function patternText(state: readonly BipolarState[]): string {
  return `[${state.map((value) => value > 0 ? "+1" : "-1").join(", ")}]`;
}

const PHASES = [
  { eyebrow: "0. Store", title: "Hebbian weights" },
  { eyebrow: "1. Cue", title: "One flipped bit" },
  { eyebrow: "2. Recall", title: "Update neuron 0" },
  { eyebrow: "3. Recall", title: "Update neuron 1" },
  { eyebrow: "4. Recall", title: "Update neuron 2" },
  { eyebrow: "5. Recall", title: "Update neuron 3" },
] as const;

export function HopfieldWorkbench() {
  const trace = useMemo(() => traceHopfieldRecall(), []);
  const [phase, setPhase] = useState(0);
  const visibleUpdateCount = Math.max(phase - 1, 0);
  const activeUpdate = visibleUpdateCount > 0 ? trace.updates[visibleUpdateCount - 1] : null;
  const visibleState = phase === 0
    ? trace.storedPattern
    : activeUpdate?.stateAfter ?? trace.corruptedState;
  const visibleEnergy = phase === 0
    ? trace.finalEnergy
    : activeUpdate?.energyAfter ?? trace.initialEnergy;
  const visibleOverlap = phase === 0
    ? 1
    : activeUpdate?.overlapAfter ?? trace.initialOverlap;

  return (
    <main className="workspace workspace--hopfield">
      <section className="hopfield-stage" aria-label="Hopfield associative memory trace">
        <div className="hopfield-intro">
          <div>
            <p className="eyebrow">NN20 - a remembered pattern becomes an attractor</p>
            <h2>Restore one flipped bit with four connected neurons</h2>
            <p>
              Store a bipolar pattern in symmetric weights, present a damaged cue,
              and audit every asynchronous update as energy moves downhill.
            </p>
          </div>
          <div className="hopfield-chip">4 neurons - 1 memory</div>
        </div>

        <section className="hopfield-store-panel" aria-label="Hopfield Hebbian storage rule">
          <div className="hopfield-heading">
            <div>
              <p>Outer product, then erase self-connections</p>
              <h2>Turn the saved pattern into weights</h2>
            </div>
            <code>w_ij = p_i p_j / 4, w_ii = 0</code>
          </div>
          <div className="hopfield-pattern-row">
            <div>
              <small>stored pattern p</small>
              <strong>{patternText(trace.storedPattern)}</strong>
            </div>
            <span aria-hidden="true">-&gt;</span>
            <div>
              <small>normalization</small>
              <strong>divide by {trace.normalization}</strong>
            </div>
            <span aria-hidden="true">-&gt;</span>
            <div>
              <small>diagonal</small>
              <strong>set to 0</strong>
            </div>
          </div>
          <div className="hopfield-matrix" role="table" aria-label="Hopfield learned weight matrix">
            <div className="hopfield-matrix__corner" />
            {trace.storedPattern.map((_, column) => <b key={`column-${column}`}>from {column}</b>)}
            {trace.weights.map((row, rowIndex) => (
              <div className="hopfield-matrix__row" role="row" key={`row-${rowIndex}`}>
                <b>to {rowIndex}</b>
                {row.map((weight, columnIndex) => (
                  <code className={rowIndex === columnIndex ? "hopfield-weight hopfield-weight--diagonal" : "hopfield-weight"} key={`${rowIndex}-${columnIndex}`}>
                    {formatNumber(weight)}
                  </code>
                ))}
              </div>
            ))}
          </div>
          <p className="hopfield-note">
            Symmetry makes the energy score valid. A zero diagonal keeps each neuron
            from voting for itself.
          </p>
        </section>

        <section className="hopfield-recall-panel" aria-label="Hopfield asynchronous recall trace">
          <div className="hopfield-heading">
            <div>
              <p>Use the newest state immediately</p>
              <h2>Recall one neuron at a time</h2>
            </div>
            <code>state_i = sign(sum_j w_ij state_j)</code>
          </div>
          <div className="hopfield-recall-lane">
            <div className="hopfield-state">
              <small>damaged cue</small>
              <strong>{patternText(trace.corruptedState)}</strong>
              <span>distance {trace.initialHammingDistance}</span>
            </div>
            {trace.updates.map((update, index) => (
              <div className={visibleUpdateCount > index ? "hopfield-update hopfield-update--visible" : "hopfield-update"} key={update.step}>
                <small>update {update.neuronIndex}</small>
                <strong>{visibleUpdateCount > index ? patternText(update.stateAfter) : "?"}</strong>
                <span>{visibleUpdateCount > index ? `field ${formatNumber(update.localField)}` : "advance to reveal"}</span>
              </div>
            ))}
          </div>

          <div className="hopfield-audit-grid">
            <div>
              <small>visible state</small>
              <strong>{patternText(visibleState)}</strong>
            </div>
            <div>
              <small>normalized overlap</small>
              <strong>{formatNumber(visibleOverlap)}</strong>
            </div>
            <div>
              <small>Hopfield energy</small>
              <strong>{formatNumber(visibleEnergy)}</strong>
            </div>
          </div>

          {activeUpdate === null ? (
            <div className="hopfield-contribution-panel">
              <p>{phase === 0 ? "The stored pattern is already a low-energy fixed point." : "The cue matches three of four saved bits. Update neuron 0 first."}</p>
            </div>
          ) : (
            <div className="hopfield-contribution-panel" aria-label="Hopfield active neuron calculation">
              <div>
                <small>active neuron</small>
                <strong>{activeUpdate.neuronIndex}</strong>
              </div>
              <div className="hopfield-contributions">
                {activeUpdate.incoming.map((row) => (
                  <code key={row.sourceIndex}>
                    {formatNumber(row.weight)} x {row.sourceState > 0 ? "+1" : "-1"} = {formatNumber(row.contribution)}
                  </code>
                ))}
              </div>
              <div>
                <small>local field -&gt; next state</small>
                <strong>{formatNumber(activeUpdate.localField)} -&gt; {activeUpdate.nextState > 0 ? "+1" : "-1"}</strong>
              </div>
              <div>
                <small>energy before -&gt; after</small>
                <strong>{formatNumber(activeUpdate.energyBefore)} -&gt; {formatNumber(activeUpdate.energyAfter)}</strong>
              </div>
              <div>
                <small>overlap before -&gt; after</small>
                <strong>{formatNumber(activeUpdate.overlapBefore)} -&gt; {formatNumber(activeUpdate.overlapAfter)}</strong>
              </div>
            </div>
          )}
        </section>
      </section>

      <aside className="hopfield-controls" aria-label="Hopfield phase controls">
        <p>Associative recall</p>
        <h2>Advance the memory</h2>
        <p>
          The first recall step repairs the flipped bit. The other steps prove the
          recovered pattern is stable under a complete deterministic sweep.
        </p>
        <div className="hopfield-phase-buttons">
          {PHASES.map((item, index) => (
            <button aria-pressed={phase === index} type="button" onClick={() => setPhase(index)} key={item.title}>
              <span>{item.eyebrow}</span>
              <strong>{item.title}</strong>
            </button>
          ))}
        </div>
        <div className="hopfield-selected-summary">
          <small>selected state</small>
          <strong>{PHASES[phase]!.title}</strong>
          <span>energy = {formatNumber(visibleEnergy)}</span>
          <span>overlap = {formatNumber(visibleOverlap)}</span>
          {phase === PHASES.length - 1 ? <b>fixed point recovered</b> : null}
        </div>
      </aside>
    </main>
  );
}
