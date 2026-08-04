import { useMemo, useState } from "react";
import { traceTinyMessagePassing } from "./message-passing-lab.js";

function format(value: number): string {
  if (Math.abs(value) < 1e-12) return "0";
  return Number.isInteger(value) ? String(value) : value.toFixed(2).replace(/0+$/, "").replace(/\.$/, "");
}

const PHASES = ["Graph", "Messages", "Aggregate", "Update"] as const;

export function MessagePassingWorkbench() {
  const trace = useMemo(() => traceTinyMessagePassing(), []);
  const [phase, setPhase] = useState<(typeof PHASES)[number]>("Graph");
  const [selectedNode, setSelectedNode] = useState(1);
  const update = trace.nodeUpdates[selectedNode]!;
  const showMessages = phase !== "Graph";
  const showAggregate = phase === "Aggregate" || phase === "Update";
  const showOutput = phase === "Update";

  return (
    <main className="workspace workspace--message-passing">
      <section className="message-stage" aria-label="Tiny graph message-passing trace">
        <div className="message-intro">
          <div>
            <p className="eyebrow">NN21 - neighbors send, nodes collect, one round updates</p>
            <h2>Pass scalar messages across a three-node path</h2>
            <p>Expand two undirected edges into four directed messages, sum each inbox, and update all nodes from the same saved feature snapshot.</p>
          </div>
          <div className="message-chip">3 nodes - 2 edges</div>
        </div>

        <section className="message-graph-panel" aria-label="Tiny graph and directed messages">
          <div className="message-heading">
            <div><p>Synchronous round</p><h2>Original features stay fixed while messages travel</h2></div>
            <code>m(source -&gt; target) = 0.5 x source</code>
          </div>
          <div className="message-graph">
            {trace.nodeFeatures.map((feature, node) => (
              <button className={selectedNode === node ? "message-node message-node--selected" : "message-node"} type="button" onClick={() => setSelectedNode(node)} key={node}>
                <small>node {node}</small><strong>{showOutput ? format(trace.outputFeatures[node]!) : format(feature)}</strong>
                <span>{showOutput ? "new feature" : "old feature"}</span>
              </button>
            ))}
            <div className="message-edge message-edge--left">0 &lt;-&gt; 1</div>
            <div className="message-edge message-edge--right">1 &lt;-&gt; 2</div>
          </div>
          <div className="message-ledger">
            {trace.directedMessages.map((row) => (
              <div className={showMessages && row.target === selectedNode ? "message-card message-card--active" : "message-card"} key={`${row.source}-${row.target}`}>
                <small>{row.source} -&gt; {row.target}</small>
                <code>0.5 x {format(row.sourceFeature)}</code>
                <strong>{showMessages ? format(row.message) : "?"}</strong>
              </div>
            ))}
          </div>
        </section>

        <section className="message-update-panel" aria-label="Selected graph node update">
          <div className="message-heading">
            <div><p>Selected node {selectedNode}</p><h2>Open its inbox and update equation</h2></div>
            <code>ReLU(0.25 x self + sum(messages) - 0.5)</code>
          </div>
          <div className="message-inbox">
            <div><small>incoming messages</small><strong>{showMessages ? update.incoming.map((row) => format(row.message)).join(" + ") : "hidden"}</strong></div>
            <span>=</span>
            <div><small>sum aggregate</small><strong>{showAggregate ? format(update.aggregate) : "?"}</strong></div>
          </div>
          <div className="message-equation">
            <div><small>self route</small><code>0.25 x {format(update.oldFeature)}</code><strong>{showAggregate ? format(update.selfContribution) : "?"}</strong></div>
            <span>+</span>
            <div><small>neighbor route</small><code>sum inbox</code><strong>{showAggregate ? format(update.aggregate) : "?"}</strong></div>
            <span>+</span>
            <div><small>bias</small><code>-0.5</code><strong>{showAggregate ? "-0.5" : "?"}</strong></div>
            <span>=</span>
            <div><small>preactivation</small><code>before ReLU</code><strong>{showAggregate ? format(update.preactivation) : "?"}</strong></div>
            <span>-&gt;</span>
            <div className="message-output"><small>new feature</small><code>ReLU</code><strong>{showOutput ? format(update.outputFeature) : "?"}</strong></div>
          </div>
          <p className="message-sync-note">All four messages use the original features `[1, 2, -1]`. No node reads another node's new output during this round.</p>
        </section>
      </section>

      <aside className="message-controls" aria-label="Message-passing phase controls">
        <p>One graph round</p><h2>Reveal the pipeline</h2>
        <p>Select any node, then expose directed messages, its order-invariant sum, and the shared update rule.</p>
        <div className="message-phase-buttons">
          {PHASES.map((item, index) => <button aria-pressed={phase === item} type="button" onClick={() => setPhase(item)} key={item}><span>{index}. Phase</span><strong>{item}</strong></button>)}
        </div>
        <div className="message-selected-summary"><small>selected node</small><strong>{selectedNode}</strong><span>neighbors = {update.incoming.map((row) => row.source).join(", ")}</span><span>output = {showOutput ? format(update.outputFeature) : "?"}</span>{showOutput ? <b>round complete</b> : null}</div>
      </aside>
    </main>
  );
}
