import { useMemo, useState } from "react";
import {
  consumerCatalog,
  traceLanguageConsumer,
  type ConsumerLaneId,
} from "./cross-language-consumers.js";

function number(value: number): string {
  return Number.isInteger(value) ? value.toFixed(1) : value.toString();
}

function command(tokens: readonly string[]): string {
  return tokens.join(" ");
}

export function CrossLanguageConsumerWorkbench() {
  const [laneId, setLaneId] = useState<ConsumerLaneId>("go-native");
  const trace = useMemo(() => traceLanguageConsumer(laneId), [laneId]);
  const hand = trace.catalog.handCheck;

  return (
    <main className="workspace workspace--language-consumers">
      <section className="consumer-stage">
        <header className="consumer-intro">
          <div>
            <p className="eyebrow">NN34 - one fixture, three independent programs</p>
            <h2>Language consumers</h2>
            <p>{trace.catalog.question}</p>
          </div>
          <span className="consumer-chip">3 native lanes</span>
        </header>

        <section className="consumer-paper" aria-label="Weighted-neuron hand calculation">
          <div className="panel-heading">
            <div><p className="eyebrow">1 - calculate</p><h2>The arithmetic never changes languages</h2></div>
            <code>{trace.row}</code>
          </div>
          <div className="consumer-products">
            {trace.contributions.map((contribution, index) => (
              <div key={index}>
                <small>contribution {index + 1}</small>
                <code>{number(hand.input[index]!)} * {number(hand.weights[index]!)}</code>
                <strong>= {number(contribution)}</strong>
              </div>
            ))}
            <div>
              <small>bias</small>
              <code>+ {number(hand.bias)}</code>
              <strong>= {number(trace.preactivation)}</strong>
            </div>
            <div>
              <small>identity</small>
              <code>identity({number(trace.preactivation)})</code>
              <strong>= {number(trace.prediction)}</strong>
            </div>
          </div>
        </section>

        <section className="consumer-lanes" aria-label="Registered native language consumers">
          <div className="panel-heading">
            <div><p className="eyebrow">2 - inspect a registration</p><h2>Select a consumer contract</h2></div>
            <span className={trace.passes ? "consumer-result consumer-result--pass" : "consumer-result consumer-result--fail"}>
              expected receipt {trace.passes ? "passes" : "fails"}
            </span>
          </div>
          <div className="consumer-lane-grid">
            {consumerCatalog.lanes.map((lane) => (
              <button
                aria-label={`Inspect ${lane.language} native consumer`}
                aria-pressed={lane.id === laneId}
                key={lane.id}
                onClick={() => setLaneId(lane.id)}
                type="button"
              >
                <small>{lane.family.replaceAll("-", " ")}</small>
                <strong>{lane.language}</strong>
                <span>{lane.execution} arithmetic</span>
              </button>
            ))}
          </div>
        </section>

        <section className="consumer-selected" aria-label="Selected consumer contract">
          <div className="panel-heading">
            <div><p className="eyebrow">3 - inspect the boundary</p><h2>Expected receipt from the {trace.lane.language} CLI</h2></div>
            <span className="consumer-chip">{trace.lane.id}</span>
          </div>
          <dl>
            <div><dt>source</dt><dd><code>{trace.lane.source}</code></dd></div>
            <div><dt>working directory</dt><dd><code>{trace.lane.workingDirectory}</code></dd></div>
            <div><dt>command vector</dt><dd><code>{command(trace.lane.command)}</code></dd></div>
          </dl>
          <div className="consumer-receipt">
            <span>contributions <code>[{trace.contributions.map(number).join(", ")}]</code></span>
            <span>prediction <code>[{number(trace.prediction)}]</code></span>
            <span>maximum error <code>{trace.maximumAbsoluteError}</code></span>
          </div>
        </section>

        <section className="consumer-protocol" aria-label="Cross-language evidence chain">
          <div className="panel-heading">
            <div><p className="eyebrow">4 - earn parity</p><h2>A zero exit is not enough</h2></div>
          </div>
          <ol>
            {trace.catalog.steps.map((step, index) => (
              <li key={step}><span>{index + 1}</span><p>{step}</p></li>
            ))}
          </ol>
        </section>
      </section>

      <aside className="consumer-controls">
        <p className="eyebrow">Complete native gate</p>
        <h2>The CLI executes all three lanes</h2>
        <code className="consumer-command">{trace.catalog.command}</code>
        <section>
          <p className="eyebrow">Browser evidence</p>
          <p>This workbench validates the catalog and recomputes the paper trace. It shows registered commands, not external runtime execution.</p>
        </section>
        <section>
          <p className="eyebrow">CLI and CI evidence</p>
          <p>The orchestrator launches fixed Go, Ruby, and Rust argument arrays, then distrusts and independently checks each JSON receipt.</p>
        </section>
        <section>
          <p className="eyebrow">Native today</p>
          <p>Each lane owns two multiplications and two additions in its own runtime. Python only judges the receipts.</p>
        </section>
        <section>
          <p className="eyebrow">Rust core next</p>
          <p>A binding-backed lane may change execution and ownership. The fixture, receipt shape, prediction, and tolerance stay fixed.</p>
        </section>
      </aside>
    </main>
  );
}
