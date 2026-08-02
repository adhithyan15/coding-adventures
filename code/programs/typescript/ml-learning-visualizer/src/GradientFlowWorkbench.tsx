import { useMemo, useState } from "react";
import { GRADIENT_SCENARIOS, traceGradientFlow } from "./gradient-flow-lab.js";

function formatNumber(value: number, digits = 6): string {
  if (Math.abs(value) < 1e-12) return "0";
  if (Math.abs(value) < 0.0001 || Math.abs(value) >= 1000) return value.toExponential(3);
  return Number(value.toFixed(digits)).toString();
}

export function GradientFlowWorkbench() {
  const [scenarioId, setScenarioId] = useState("small-tanh");
  const [selectedLayer, setSelectedLayer] = useState(3);
  const trace = useMemo(() => traceGradientFlow(scenarioId), [scenarioId]);
  const comparison = useMemo(
    () => GRADIENT_SCENARIOS.map((scenario) => traceGradientFlow(scenario.id)),
    [],
  );
  const layer = trace.layers[selectedLayer]!;
  const maxLogGradient = Math.max(
    ...comparison.map((item) => Math.log10(1 + Math.abs(item.inputGradient))),
    1e-12,
  );

  return (
    <main className="workspace workspace--gradient-flow">
      <section className="gradient-flow-stage" aria-label="Vanishing and exploding gradient explorer">
        <div className="gradient-flow-intro">
          <div>
            <p className="eyebrow">NN24 / reverse one scalar chain</p>
            <h2>Vanishing and exploding gradients</h2>
            <p>Multiply four local Jacobians and watch one loss gradient travel from the output back to the input.</p>
          </div>
          <div className={`gradient-flow-chip gradient-flow-chip--${trace.classification}`}>{trace.classification}</div>
        </div>

        <section className="gradient-forward-panel" aria-label="Gradient flow forward pass">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Forward / save every value</p>
              <h2>Input to loss</h2>
            </div>
            <span>half squared error target {formatNumber(trace.scenario.target)}</span>
          </div>
          <div className="gradient-forward-lane">
            <div><span>input</span><strong>{formatNumber(trace.scenario.input)}</strong></div>
            {trace.layers.map((item) => (
              <div key={item.layer}>
                <span>layer {item.layer}</span>
                <code>{formatNumber(item.input)} x {formatNumber(item.weight)}</code>
                <strong>{trace.scenario.activation} = {formatNumber(item.activation)}</strong>
              </div>
            ))}
            <div className="gradient-loss-node"><span>loss</span><strong>{formatNumber(trace.loss)}</strong></div>
          </div>
        </section>

        <section className="gradient-backward-panel" aria-label="Gradient flow backward pass">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Backward / multiply local slopes</p>
              <h2>Loss to input</h2>
            </div>
            <span>start dL/da4 = {formatNumber(trace.outputError)}</span>
          </div>
          <div className="gradient-backward-lane">
            {[...trace.layers].reverse().map((item) => (
              <button
                aria-pressed={selectedLayer === item.layer - 1}
                key={item.layer}
                type="button"
                onClick={() => setSelectedLayer(item.layer - 1)}
              >
                <span>layer {item.layer}</span>
                <small>upstream {formatNumber(item.upstreamGradient)}</small>
                <strong>local x {formatNumber(item.localJacobian)}</strong>
                <code>to input {formatNumber(item.inputGradient)}</code>
              </button>
            ))}
            <div className="gradient-input-node">
              <span>input gradient</span>
              <strong>{formatNumber(trace.inputGradient)}</strong>
            </div>
          </div>
        </section>

        <section className="gradient-arithmetic-panel" aria-label="Selected gradient calculation">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Open layer {layer.layer}</p>
              <h2>One chain-rule step</h2>
            </div>
            <span>saved input {formatNumber(layer.input)}</span>
          </div>
          <div className="gradient-equation-grid">
            <code>{formatNumber(layer.upstreamGradient)} x {formatNumber(layer.activationDerivative)} = {formatNumber(layer.preactivationGradient)}</code>
            <span>dL/da x da/dz = dL/dz</span>
            <code>{formatNumber(layer.preactivationGradient)} x {formatNumber(layer.weight)} = {formatNumber(layer.inputGradient)}</code>
            <span>dL/dz x dz/dinput</span>
            <code>{formatNumber(layer.preactivationGradient)} x {formatNumber(layer.input)} = {formatNumber(layer.weightGradient)}</code>
            <span>dL/dz x saved input = dL/dw</span>
          </div>
        </section>

        <section className="gradient-chain-panel" aria-label="Gradient chain product">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Separate the path from the loss</p>
              <h2>Total local Jacobian product</h2>
            </div>
            <strong>{formatNumber(trace.chainJacobian)}</strong>
          </div>
          <div className="gradient-chain-equation">
            {trace.layers.map((item) => <code key={item.layer}>{formatNumber(item.localJacobian)}</code>)}
            <span>=</span>
            <strong>{formatNumber(trace.chainJacobian)}</strong>
          </div>
          <p>{formatNumber(trace.outputError)} output error x {formatNumber(trace.chainJacobian)} chain = <strong>{formatNumber(trace.inputGradient)}</strong> input gradient.</p>
          <div className="gradient-audit">
            <span>central finite difference</span>
            <code>{formatNumber(trace.finiteDifferenceInputGradient)}</code>
            <span>absolute error</span>
            <code>{formatNumber(trace.finiteDifferenceError)}</code>
          </div>
        </section>

        <section className="gradient-comparison-panel" aria-label="Gradient scenario comparison">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Four mechanisms side by side</p>
              <h2>Compare input-gradient magnitude</h2>
            </div>
            <span>bar uses log10(1 + |gradient|)</span>
          </div>
          <div className="gradient-comparison-grid">
            {comparison.map((item) => (
              <article className={item.scenario.id === scenarioId ? "is-selected" : ""} key={item.scenario.id}>
                <strong>{item.scenario.label}</strong>
                <span>{item.classification}</span>
                <i style={{ width: `${(Math.log10(1 + Math.abs(item.inputGradient)) / maxLogGradient) * 100}%` }} />
                <code>dL/dinput {formatNumber(item.inputGradient)}</code>
                <small>chain {formatNumber(item.chainJacobian)}</small>
              </article>
            ))}
          </div>
        </section>
      </section>

      <aside className="controls gradient-flow-controls">
        <p className="eyebrow">Gradient mechanism</p>
        <h2>Choose a chain</h2>
        <p>Each scenario keeps four scalar layers and target zero.</p>
        <div className="gradient-scenario-buttons">
          {GRADIENT_SCENARIOS.map((scenario) => (
            <button
              aria-pressed={scenario.id === scenarioId}
              key={scenario.id}
              type="button"
              onClick={() => setScenarioId(scenario.id)}
            >
              <strong>{scenario.label}</strong>
              <span>{scenario.summary}</span>
              <code>{scenario.weights.join(" x ")} / {scenario.activation}</code>
            </button>
          ))}
        </div>
        <div className="gradient-flow-reading">
          <p className="eyebrow">What to notice</p>
          <h2>{trace.classification === "vanishing" ? "Early layers hear a whisper" : trace.classification === "exploding" ? "Early layers receive a blast" : "The gradient keeps its scale"}</h2>
          <p>Changing a weight changes both the forward activation and the local factor used on the reverse path.</p>
        </div>
      </aside>
    </main>
  );
}
