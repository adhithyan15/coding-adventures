import { useMemo, useState } from "react";
import {
  DYNAMIC_AUTOGRAD_SCENARIOS,
  traceDynamicAutograd,
  type AutogradNodeTrace,
  type DynamicAutogradScenarioId,
} from "./dynamic-autograd-lab.js";

function formatNumber(value: number, digits = 6): string {
  if (Math.abs(value) < 1e-12) return "0";
  if (Math.abs(value) < 0.0001 || Math.abs(value) >= 1000) return value.toExponential(3);
  return Number(value.toFixed(digits)).toString();
}

function operationLabel(operation: AutogradNodeTrace["operation"]): string {
  return operation === "input" ? "leaf input" : operation;
}

function forwardFormula(node: AutogradNodeTrace): string {
  if (node.operation === "input") return `${node.id} entered the graph as a leaf`;
  if (node.operation === "multiply") return `${node.id} = ${node.parents[0]} × ${node.parents[1]}`;
  if (node.operation === "add") return `${node.id} = ${node.parents[0]} + ${node.parents[1]}`;
  if (node.operation === "square") return `${node.id} = ${node.parents[0]}²`;
  if (node.operation === "negate") return `${node.id} = -${node.parents[0]}`;
  return `${node.id} = identity(${node.parents[0]})`;
}

export function DynamicAutogradWorkbench() {
  const [scenarioId, setScenarioId] = useState<DynamicAutogradScenarioId>("multiply_add_square");
  const [selectedNodeId, setSelectedNodeId] = useState("m");
  const [selectedBackwardIndex, setSelectedBackwardIndex] = useState(0);
  const [applyMutations, setApplyMutations] = useState(true);
  const trace = useMemo(
    () => traceDynamicAutograd(scenarioId, applyMutations),
    [scenarioId, applyMutations],
  );
  const selectedNode = trace.nodes.find((node) => node.id === selectedNodeId) ?? trace.nodes.at(-1)!;
  const selectedBackward = trace.backwardSteps[
    Math.min(selectedBackwardIndex, trace.backwardSteps.length - 1)
  ]!;
  const hasMutation = Object.keys(trace.scenario.mutationsAfterForward).length > 0;

  function chooseScenario(id: DynamicAutogradScenarioId): void {
    const scenario = DYNAMIC_AUTOGRAD_SCENARIOS.find((item) => item.id === id)!;
    setScenarioId(id);
    setSelectedNodeId(scenario.steps[0]!.id);
    setSelectedBackwardIndex(0);
    setApplyMutations(true);
  }

  return (
    <main className="workspace workspace--dynamic-autograd">
      <section className="autograd-stage" aria-label="Dynamic autograd and saved value visualizer">
        <section className="autograd-intro">
          <div>
            <p className="eyebrow">NN27 / tensor and autograd bridge</p>
            <h2>Dynamic graph and saved-value microscope</h2>
            <p>The forward run records only executed operations. Backward reverses that graph and reads immutable forward snapshots.</p>
          </div>
          <div className="autograd-chip">reverse mode</div>
        </section>

        <section className="autograd-graph-panel" aria-label="Executed dynamic computation graph">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Step 1 / record what ran</p>
              <h2>{trace.scenario.expression}</h2>
            </div>
            <span>{trace.nodes.length} executed nodes</span>
          </div>
          <div className="autograd-order-strip">
            <small>topological order</small>
            <code>{trace.topologicalOrder.join(" → ")}</code>
          </div>
          <div className="autograd-node-lane">
            {trace.nodes.map((node) => (
              <button
                aria-label={`Open node ${node.id}, ${operationLabel(node.operation)}, value ${formatNumber(node.forwardValue)}`}
                aria-pressed={node.id === selectedNode.id}
                key={node.id}
                type="button"
                onClick={() => setSelectedNodeId(node.id)}
              >
                <small>{operationLabel(node.operation)}</small>
                <strong>{node.id} = {formatNumber(node.forwardValue)}</strong>
                <span>{node.parents.length ? `from ${node.parents.join(" + ")}` : "leaf"}</span>
              </button>
            ))}
          </div>
          {Object.entries(trace.branchChoices).map(([nodeId, choice]) => (
            <div className="autograd-branch-note" key={nodeId}>
              <strong>{nodeId}</strong> chose the <code>{choice}</code> branch. The other operation is absent from this graph.
            </div>
          ))}
        </section>

        <section className="autograd-saved-panel" aria-label="Selected node forward and saved value trace">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Step 2 / save the derivative ingredients</p>
              <h2>Open node {selectedNode.id}</h2>
            </div>
            <span>{operationLabel(selectedNode.operation)}</span>
          </div>
          <div className="autograd-selected-grid">
            <div>
              <small>forward rule</small>
              <code>{forwardFormula(selectedNode)}</code>
              <strong>value {formatNumber(selectedNode.forwardValue)}</strong>
            </div>
            <div>
              <small>saved for backward</small>
              {selectedNode.savedValues.length ? selectedNode.savedValues.map((item) => (
                <code key={item.name}>{item.name} ← {item.sourceId} = {formatNumber(item.value)}</code>
              )) : <code>nothing — local derivative is constant</code>}
            </div>
          </div>
          {hasMutation ? (
            <div className="autograd-mutation-strip">
              {trace.scenario.inputs.map((input) => {
                const live = trace.liveInputValues[input.id]!;
                return (
                  <div className={live !== input.value ? "is-mutated" : ""} key={input.id}>
                    <small>{input.id}</small>
                    <code>forward {formatNumber(input.value)}</code>
                    <strong>live {formatNumber(live)}</strong>
                  </div>
                );
              })}
              <p>Backward reads the saved forward snapshots, never the later live value.</p>
            </div>
          ) : null}
        </section>

        <section className="autograd-backward-panel" aria-label="Reverse topological backward trace">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Step 3 / reverse the executed graph</p>
              <h2>Upstream × local derivative</h2>
            </div>
            <span>{trace.backwardOrder.join(" ← ")}</span>
          </div>
          <div className="autograd-backward-buttons">
            {trace.backwardSteps.map((step, index) => (
              <button
                aria-label={`Open backward node ${step.nodeId}, upstream ${formatNumber(step.upstreamGradient)}`}
                aria-pressed={index === selectedBackwardIndex}
                key={step.nodeId}
                type="button"
                onClick={() => setSelectedBackwardIndex(index)}
              >
                <small>{step.operation}</small>
                <strong>{step.nodeId}</strong>
                <code>upstream {formatNumber(step.upstreamGradient)}</code>
              </button>
            ))}
          </div>
          <div className="autograd-backward-equations" aria-label="Selected backward calculation">
            {selectedBackward.localDerivatives.map((derivative, index) => {
              const contribution = selectedBackward.parentContributions[index]!;
              return (
                <div key={`${selectedBackward.nodeId}-${derivative.parentId}`}>
                  <small>toward {derivative.parentId}</small>
                  <code>
                    {formatNumber(selectedBackward.upstreamGradient)} × {formatNumber(derivative.value)} = {formatNumber(contribution.value)}
                  </code>
                  <span>local source: {derivative.source}</span>
                </div>
              );
            })}
          </div>
        </section>

        <section className="autograd-audit-panel" aria-label="Dynamic autograd finite difference audit">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Step 4 / distrust the graph once</p>
              <h2>Fresh forwards check every leaf</h2>
            </div>
            <span>epsilon 1e-5</span>
          </div>
          <div className="autograd-audit-grid">
            {trace.scenario.inputs.map((input) => (
              <div key={input.id}>
                <strong>{input.id}</strong>
                <span>analytical <code>{formatNumber(trace.gradients[input.id]!)}</code></span>
                <span>numerical <code>{formatNumber(trace.finiteDifferenceGradients[input.id]!)}</code></span>
                <small>error {formatNumber(trace.gradientAbsoluteErrors[input.id]!)}</small>
              </div>
            ))}
            <div className="autograd-audit-max">
              <strong>maximum error</strong>
              <code>{formatNumber(trace.maxGradientAbsoluteError)}</code>
              <small>must stay below 1e-8</small>
            </div>
          </div>
        </section>
      </section>

      <aside className="controls autograd-controls" aria-label="Dynamic autograd scenarios">
        <p className="eyebrow">Graph presets</p>
        <h2>Change one graph rule</h2>
        <div className="autograd-scenario-buttons">
          {DYNAMIC_AUTOGRAD_SCENARIOS.map((scenario) => (
            <button
              aria-pressed={scenario.id === scenarioId}
              key={scenario.id}
              type="button"
              onClick={() => chooseScenario(scenario.id)}
            >
              <strong>{scenario.title}</strong>
              <code>{scenario.expression}</code>
              <span>{scenario.summary}</span>
            </button>
          ))}
        </div>
        {hasMutation ? (
          <button
            className="autograd-mutation-toggle"
            aria-pressed={applyMutations}
            type="button"
            onClick={() => setApplyMutations((value) => !value)}
          >
            {applyMutations ? "Restore forward-time live values" : "Apply post-forward mutation"}
          </button>
        ) : null}
        <div className="autograd-mental-model">
          <p className="eyebrow">Keep this picture</p>
          <h2>Record, save, reverse.</h2>
          <p>Record only executed operations. Save only derivative ingredients. Reverse children before parents.</p>
        </div>
      </aside>
    </main>
  );
}
