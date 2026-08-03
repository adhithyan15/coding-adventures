import { useMemo, useState } from "react";
import {
  implementationCoverageCatalog,
  traceImplementationCoverage,
  type CoverageLaneId,
} from "./implementation-coverage.js";

function number(value: number): string {
  return Number.isInteger(value) ? value.toFixed(1) : value.toString();
}

export function ImplementationCoverageWorkbench() {
  const [laneId, setLaneId] = useState<CoverageLaneId>("go-native");
  const trace = useMemo(() => traceImplementationCoverage(laneId), [laneId]);
  const hand = trace.catalog.handCheck;

  return (
    <main className="workspace workspace--language-consumers workspace--implementation-coverage">
      <section className="consumer-stage">
        <header className="consumer-intro">
          <div>
            <p className="eyebrow">NN36 - count ownership, not just languages</p>
            <h2>Implementation coverage</h2>
            <p>{trace.catalog.question}</p>
          </div>
          <span className="consumer-chip">3 native + 1 binding</span>
        </header>

        <section className="consumer-paper" aria-label="Implementation coverage hand calculation">
          <div className="panel-heading">
            <div><p className="eyebrow">1 - hold the answer still</p><h2>Every lane must still earn 1.35</h2></div>
            <span className="consumer-result consumer-result--pass">4 verified lanes</span>
          </div>
          <div className="consumer-products">
            <div><small>contribution 1</small><code>{number(hand.inputs[0])} * {number(hand.weights[0])}</code><strong>= {number(hand.contributions[0])}</strong></div>
            <div><small>contribution 2</small><code>{number(hand.inputs[1])} * {number(hand.weights[1])}</code><strong>= {number(hand.contributions[1])}</strong></div>
            <div><small>prediction</small><code>1.0 + 0.25 + 0.1</code><strong>= {number(hand.prediction)}</strong></div>
            <div><small>coverage</small><code>3 native + 1 binding</code><strong>= 4 lanes</strong></div>
          </div>
        </section>

        <section className="consumer-lanes" aria-label="Native and Rust-core coverage lanes">
          <div className="panel-heading">
            <div><p className="eyebrow">2 - inspect ownership</p><h2>Select an implementation path</h2></div>
            <span className="consumer-chip">owner: {trace.lane.arithmeticOwner}</span>
          </div>
          <div className="consumer-lane-grid implementation-coverage-lanes">
            {implementationCoverageCatalog.lanes.map((lane) => (
              <button
                aria-label={`Inspect ${lane.language} ${lane.implementation} coverage lane`}
                aria-pressed={lane.id === laneId}
                key={lane.id}
                onClick={() => setLaneId(lane.id)}
                type="button"
              >
                <small>{lane.implementation.replaceAll("-", " ")}</small>
                <strong>{lane.language}</strong>
                <span>{lane.arithmeticOwner} owns arithmetic</span>
              </button>
            ))}
          </div>
        </section>

        <section className="consumer-selected" aria-label="Selected implementation coverage contract">
          <div className="panel-heading">
            <div><p className="eyebrow">3 - follow the call</p><h2>{trace.lane.interface}</h2></div>
            <span className="consumer-chip">{trace.lane.id}</span>
          </div>
          <dl>
            <div><dt>classification</dt><dd><code>{trace.lane.implementation}</code></dd></div>
            <div><dt>arithmetic owner</dt><dd><code>{trace.lane.arithmeticOwner}</code></dd></div>
            <div><dt>execution evidence</dt><dd><code>{trace.lane.evidence}</code></dd></div>
            <div><dt>validator</dt><dd><code>{trace.lane.validator}</code></dd></div>
          </dl>
        </section>

        <section className="consumer-protocol" aria-label="Implementation coverage rules">
          <div className="panel-heading">
            <div><p className="eyebrow">4 - count only earned evidence</p><h2>Coverage is an inventory</h2></div>
          </div>
          <ol>
            {trace.catalog.rules.map((rule, index) => (
              <li key={rule}><span>{index + 1}</span><p>{rule}</p></li>
            ))}
          </ol>
        </section>
      </section>

      <aside className="consumer-controls">
        <p className="eyebrow">One fixture, two ownership models</p>
        <h2>{trace.nativeFraction} native; {trace.bindingFraction} binding</h2>
        <code className="consumer-command">{trace.catalog.contracts.coverage_validator}</code>
        <section><p className="eyebrow">Native</p><p>Go, Ruby, and Rust each parse the NN03 fixture and perform their own two products and sum.</p></section>
        <section><p className="eyebrow">Rust-core binding</p><p>Python allocates buffers with `ctypes`, crosses the versioned C ABI, and lets Rust perform the arithmetic.</p></section>
        <section><p className="eyebrow">Executable evidence</p><p>The coverage validator reruns both underlying gates before it prints the four-lane total.</p></section>
        <section><p className="eyebrow">Browser evidence</p><p>This view validates registered metadata and recomputes the paper trace. It does not execute Go, Ruby, Python, or the native library.</p></section>
      </aside>
    </main>
  );
}
