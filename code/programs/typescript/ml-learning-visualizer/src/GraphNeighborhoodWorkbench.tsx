import { useMemo, useState } from "react";
import { traceGraphNeighborhoodComparison } from "./graph-neighborhood-lab.js";

function fmt(value: number): string {
  if (Math.abs(value) < 1e-12) return "0";
  if (Number.isInteger(value)) return String(value);
  return value.toFixed(6).replace(/0+$/, "");
}

export function GraphNeighborhoodWorkbench() {
  const trace = useMemo(() => traceGraphNeighborhoodComparison(), []);
  const [model, setModel] = useState<"gcn" | "gat">("gcn");
  const [target, setTarget] = useState(1);
  const gcn = trace.gcn[target]!;
  const gat = trace.gat[target]!;

  return (
    <main className="workspace workspace--graph-neighborhood">
      <section className="graph-neighborhood-stage" aria-label="Graph convolution and attention trace">
        <div className="graph-neighborhood-intro"><div><p className="eyebrow">NN22 - same neighborhood, two weighting rules</p><h2>Compare graph convolution with graph attention</h2><p>Add self-loops to one three-node path, then inspect fixed degree normalization beside learned softmax attention.</p></div><div className="graph-neighborhood-chip">GCN vs GAT</div></div>

        <section className="graph-neighborhood-map" aria-label="Graph neighborhood selector">
          <div className="graph-neighborhood-heading"><div><p>Original scalar features</p><h2>Select a target neighborhood</h2></div><code>0(1) &lt;-&gt; 1(2) &lt;-&gt; 2(-1), plus self-loops</code></div>
          <div className="graph-targets">{trace.features.map((feature, node) => <button aria-pressed={target === node} type="button" onClick={() => setTarget(node)} key={node}><small>node {node}</small><strong>{fmt(feature)}</strong><span>degree {trace.degrees[node]}</span></button>)}</div>
          <p>Target {target} reads sources [{trace.neighborhoods[target]!.join(", ")}]. Both models use exactly this same inbox.</p>
        </section>

        {model === "gcn" ? (
          <section className="graph-model-panel" aria-label="Graph convolution calculation">
            <div className="graph-neighborhood-heading"><div><p>Fixed structural weights</p><h2>Normalize by both endpoint degrees</h2></div><code>coefficient = 1 / sqrt(d_target x d_source)</code></div>
            <div className="graph-row-grid">{gcn.rows.map((row) => <div key={row.source}><small>source {row.source}</small><code>1 / sqrt({row.targetDegree} x {row.sourceDegree})</code><strong>{fmt(row.coefficient)}</strong><span>x feature {fmt(row.sourceFeature)}</span><b>= {fmt(row.contribution)}</b></div>)}</div>
            <div className="graph-result"><span>sum contributions</span><strong>{gcn.rows.map((row) => fmt(row.contribution)).join(" + ")} = {fmt(gcn.preactivation)}</strong><b>ReLU -&gt; {fmt(gcn.output)}</b></div>
          </section>
        ) : (
          <section className="graph-model-panel" aria-label="Graph attention calculation">
            <div className="graph-neighborhood-heading"><div><p>Data-dependent weights</p><h2>Softmax the source scores inside this inbox</h2></div><code>score = source feature; alpha = stable softmax(score)</code></div>
            <div className="graph-softmax-summary"><span>row max = {fmt(gat.maximumScore)}</span><span>denominator = {fmt(gat.denominator)}</span><strong>weights sum = {fmt(gat.rows.reduce((sum, row) => sum + row.attentionWeight, 0))}</strong></div>
            <div className="graph-row-grid">{gat.rows.map((row) => <div key={row.source}><small>source {row.source}</small><code>score {fmt(row.score)} - max {fmt(gat.maximumScore)} = {fmt(row.shiftedScore)}</code><span>exp = {fmt(row.exponential)}</span><strong>alpha = {fmt(row.attentionWeight)}</strong><b>x {fmt(row.sourceFeature)} = {fmt(row.contribution)}</b></div>)}</div>
            <div className="graph-result"><span>weighted sum</span><strong>{gat.rows.map((row) => fmt(row.contribution)).join(" + ")} = {fmt(gat.preactivation)}</strong><b>ReLU -&gt; {fmt(gat.output)}</b></div>
          </section>
        )}

        <section className="graph-output-panel" aria-label="Graph model output comparison"><div><small>GCN outputs</small><strong>[{trace.gcnOutputs.map(fmt).join(", ")}]</strong></div><div><small>GAT outputs</small><strong>[{trace.gatOutputs.map(fmt).join(", ")}]</strong></div><p>GCN weights depend only on graph degrees. GAT weights change with the node features, even though the edges are unchanged.</p></section>
      </section>

      <aside className="graph-neighborhood-controls" aria-label="Graph model controls"><p>Neighborhood model</p><h2>Switch the weighting rule</h2><p>Keep the target and graph fixed while changing how its inbox is weighted.</p><button aria-pressed={model === "gcn"} type="button" onClick={() => setModel("gcn")}><span>Degree rule</span><strong>Graph convolution</strong></button><button aria-pressed={model === "gat"} type="button" onClick={() => setModel("gat")}><span>Softmax rule</span><strong>Graph attention</strong></button><div><small>selected target</small><strong>{target}</strong><span>{model === "gcn" ? "structural coefficients" : "feature-dependent attention"}</span></div></aside>
    </main>
  );
}
