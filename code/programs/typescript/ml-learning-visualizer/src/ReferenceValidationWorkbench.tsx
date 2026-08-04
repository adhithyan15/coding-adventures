import { useMemo, useState } from "react";
import {
  referenceCatalog,
  traceReferenceValidation,
  type ReferenceTrackId,
} from "./reference-validation-catalog.js";

const TRACK_LABELS: Readonly<Record<ReferenceTrackId, string>> = {
  foundation: "Foundation",
  spatial: "Spatial",
  sequence: "Sequence",
  attention: "Attention",
  representation: "Representation",
  structured: "Structured",
  "deep-training": "Deep training",
  autograd: "Tensor + autograd",
  compilation: "Compilation",
};

function scientific(value: number): string {
  return value === 0 ? "0" : value.toExponential(3);
}

export function ReferenceValidationWorkbench() {
  const [familyId, setFamilyId] = useState("neural-learning");
  const [track, setTrack] = useState<ReferenceTrackId | "all">("all");
  const trace = useMemo(() => traceReferenceValidation(familyId), [familyId]);
  const visibleFamilies = referenceCatalog.families.filter((family) => track === "all" || family.track === track);

  return (
    <main className="workspace workspace--reference-validation">
      <section className="reference-stage">
        <header className="reference-intro">
          <div>
            <p className="eyebrow">NN33 - every answer needs an independent witness</p>
            <h2>Reference fixture catalog</h2>
            <p>{trace.catalog.question}</p>
          </div>
          <span className="reference-chip">{trace.familyCount} families · {trace.labCount} labs</span>
        </header>

        <section className="reference-paper" aria-label="Hand-calculated tolerance check">
          <div className="panel-heading">
            <div><p className="eyebrow">1 - calculate</p><h2>Check one stored answer by hand</h2></div>
            <code>{trace.catalog.handCheck.equation}</code>
          </div>
          <div className="reference-equation">
            <code>|{trace.catalog.handCheck.recomputed} - {trace.catalog.handCheck.stored}|</code>
            <span>=</span>
            <strong>{scientific(trace.recomputedError)}</strong>
            <span>≤</span>
            <code>{scientific(trace.catalog.handCheck.absoluteTolerance)}</code>
            <span className={trace.passes ? "reference-result reference-result--pass" : "reference-result reference-result--fail"}>
              {trace.passes ? "passes" : "fails"}
            </span>
          </div>
        </section>

        <section className="reference-protocol" aria-label="Reference validation evidence chain">
          <div className="panel-heading">
            <div><p className="eyebrow">2 - follow evidence</p><h2>One failure stops the chain</h2></div>
          </div>
          <ol>
            {trace.catalog.steps.map((step, index) => (
              <li key={step}><span>{index + 1}</span><p>{step}</p></li>
            ))}
          </ol>
        </section>

        <section className="reference-roster" aria-label="Registered neural fixture families">
          <div className="panel-heading">
            <div><p className="eyebrow">3 - inspect the roster</p><h2>Every curriculum family is accounted for</h2></div>
            <label>
              track
              <select aria-label="Filter reference families by track" value={track} onChange={(event) => setTrack(event.currentTarget.value as ReferenceTrackId | "all")}>
                <option value="all">All {trace.trackCount} tracks</option>
                {Object.entries(TRACK_LABELS).map(([id, label]) => <option key={id} value={id}>{label}</option>)}
              </select>
            </label>
          </div>
          <div className="reference-family-grid">
            {visibleFamilies.map((family) => (
              <button
                aria-label={`Inspect NN${String(family.order).padStart(2, "0")} ${family.title}`}
                aria-pressed={family.id === familyId}
                key={family.id}
                onClick={() => setFamilyId(family.id)}
                type="button"
              >
                <small>NN{String(family.order).padStart(2, "0")} · {TRACK_LABELS[family.track]}</small>
                <strong>{family.title}</strong>
                <span>{family.labCount} lab{family.labCount === 1 ? "" : "s"} · registered</span>
              </button>
            ))}
          </div>
        </section>

        <section className="reference-selected" aria-label="Selected fixture-family contract">
          <div className="panel-heading">
            <div><p className="eyebrow">4 - selected contract</p><h2>NN{String(trace.family.order).padStart(2, "0")} · {trace.family.title}</h2></div>
            <span className="reference-chip">{trace.family.oracle}</span>
          </div>
          <dl>
            <div><dt>spec</dt><dd><code>{trace.family.spec}</code></dd></div>
            <div><dt>fixture root</dt><dd><code>{trace.family.fixtureRoot}</code></dd></div>
            <div><dt>reference validator</dt><dd><code>{trace.family.validator}</code></dd></div>
          </dl>
        </section>
      </section>

      <aside className="reference-controls">
        <p className="eyebrow">Executable contract</p>
        <h2>The CLI earns the green result</h2>
        <code className="reference-command">{trace.catalog.command}</code>
        <section>
          <p className="eyebrow">Browser evidence</p>
          <p>This workbench verifies the catalog shape and recomputes the small tolerance example. Each family is labeled registered, not executed.</p>
        </section>
        <section>
          <p className="eyebrow">CLI and CI evidence</p>
          <p>The Python orchestrator runs all 30 validators without a shell. A timeout, silent success, or non-zero exit fails the complete gate.</p>
        </section>
        <section>
          <p className="eyebrow">Cross-language direction</p>
          <p>Each port reads the same JSON and earns its own comparison. The next tranche adds representative native consumers.</p>
        </section>
        <section>
          <p className="eyebrow">Rust-core direction</p>
          <p>A future C ABI may run the math faster, but its shapes, bytes, outputs, and tolerances still answer to this catalog.</p>
        </section>
      </aside>
    </main>
  );
}
