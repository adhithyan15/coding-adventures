import { useMemo, useState } from "react";
import { traceTwoNumberAutoencoder } from "./autoencoder-lab.js";

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  return Number(value.toFixed(8)).toString();
}

function formatVector(values: readonly number[]): string {
  return `[${values.map(formatNumber).join(", ")}]`;
}

export function AutoencoderWorkbench() {
  const trace = useMemo(() => traceTwoNumberAutoencoder(), []);
  const [selectedOutput, setSelectedOutput] = useState(0);
  const [showUpdated, setShowUpdated] = useState(false);
  const currentForward = showUpdated ? trace.postUpdate : trace.forward;
  const currentParameters = showUpdated
    ? trace.updatedParameters
    : trace.parameters;

  return (
    <main className="workspace workspace--autoencoder">
      <section className="autoencoder-stage" aria-label="Two-number autoencoder bottleneck trace">
        <div className="autoencoder-intro">
          <div>
            <p className="eyebrow">NN16 - representation through constraint</p>
            <h2>Two numbers through one bottleneck</h2>
            <p>
              Compress a two-coordinate input into one scalar, reconstruct both
              coordinates from that shared value, and follow both errors back
              through one audited SGD step.
            </p>
          </div>
          <div className="autoencoder-chip">2 -&gt; 1 -&gt; 2</div>
        </div>

        <section className="autoencoder-network-panel" aria-label="Autoencoder encode and decode path">
          <div className="autoencoder-heading">
            <div>
              <p className="eyebrow">The decoder never sees the original pair</p>
              <h2>One scalar must serve two reconstructions</h2>
            </div>
            <code>{showUpdated ? "after one SGD step" : "saved forward pass"}</code>
          </div>

          <div className="autoencoder-network">
            <div className="autoencoder-input-stack">
              <small>input is also target</small>
              {trace.input.map((value, index) => (
                <div key={index}>
                  <span>x{index}</span>
                  <strong>{formatNumber(value)}</strong>
                </div>
              ))}
            </div>

            <span className="autoencoder-arrow" aria-hidden="true">-&gt;</span>

            <div className="autoencoder-encoder-stack">
              <small>encoder products</small>
              {currentForward.encoderProducts.map((product, index) => (
                <code key={index}>
                  {formatNumber(trace.input[index]!)} x {formatNumber(currentParameters.encoder.weights[index]!)} = {formatNumber(product)}
                </code>
              ))}
              <code>+ bias {formatNumber(currentParameters.encoder.bias)}</code>
            </div>

            <span className="autoencoder-arrow" aria-hidden="true">-&gt;</span>

            <div className="autoencoder-bottleneck">
              <small>bottleneck z</small>
              <strong>{formatNumber(currentForward.bottleneck)}</strong>
              <span>one saved number</span>
            </div>

            <span className="autoencoder-arrow" aria-hidden="true">-&gt;</span>

            <div className="autoencoder-output-stack">
              <small>decoder reconstructions</small>
              {currentForward.reconstruction.map((value, index) => (
                <button
                  aria-label={`Select reconstruction ${index}`}
                  aria-pressed={selectedOutput === index}
                  key={index}
                  type="button"
                  onClick={() => setSelectedOutput(index)}
                >
                  <span>x_hat{index}</span>
                  <strong>{formatNumber(value)}</strong>
                  <small>target {formatNumber(trace.input[index]!)}</small>
                </button>
              ))}
            </div>
          </div>
        </section>

        <section className="autoencoder-reconstruction-panel" aria-label={`Selected autoencoder reconstruction ${selectedOutput}`}>
          <div className="autoencoder-heading">
            <div>
              <p className="eyebrow">Selected - reconstruction {selectedOutput}</p>
              <h2>Decode and measure one coordinate</h2>
            </div>
            <div className="autoencoder-loss-badge">
              <small>total mean loss</small>
              <strong>{formatNumber(currentForward.loss)}</strong>
            </div>
          </div>

          <div className="autoencoder-reconstruction-flow">
            <div>
              <small>shared bottleneck</small>
              <code>z = {formatNumber(currentForward.bottleneck)}</code>
            </div>
            <span aria-hidden="true">x</span>
            <div>
              <small>decoder weight {selectedOutput}</small>
              <code>{formatNumber(currentParameters.decoder.weights[selectedOutput]!)}</code>
            </div>
            <span aria-hidden="true">+</span>
            <div>
              <small>decoder bias {selectedOutput}</small>
              <code>{formatNumber(currentParameters.decoder.bias[selectedOutput]!)}</code>
            </div>
            <span aria-hidden="true">=</span>
            <div className="autoencoder-reconstruction-result">
              <small>reconstruction</small>
              <strong>{formatNumber(currentForward.reconstruction[selectedOutput]!)}</strong>
            </div>
            <span aria-hidden="true">-</span>
            <div>
              <small>input target</small>
              <code>{formatNumber(trace.input[selectedOutput]!)}</code>
            </div>
            <span aria-hidden="true">=</span>
            <div className="autoencoder-error-result">
              <small>error / loss gradient</small>
              <strong>{formatNumber(currentForward.errors[selectedOutput]!)}</strong>
              <code>squared {formatNumber(currentForward.squaredErrors[selectedOutput]!)}</code>
            </div>
          </div>
        </section>

        <section className="autoencoder-backward-panel" aria-label="Autoencoder bottleneck gradient trace">
          <div className="autoencoder-heading">
            <div>
              <p className="eyebrow">Two decoder branches meet at z</p>
              <h2>Reconstruction error flows back through compression</h2>
            </div>
            <code>dL/dz = sum of both routes</code>
          </div>

          <div className="autoencoder-branch-gradients">
            {trace.backward.bottleneckGradientContributions.map((contribution, index) => (
              <button
                aria-label={`Select reconstruction gradient ${index}`}
                aria-pressed={selectedOutput === index}
                key={index}
                type="button"
                onClick={() => setSelectedOutput(index)}
              >
                <small>output {index} route</small>
                <code>
                  {formatNumber(trace.backward.reconstructionGradients[index]!)} x {formatNumber(trace.parameters.decoder.weights[index]!)}
                </code>
                <strong>{formatNumber(contribution)}</strong>
              </button>
            ))}
            <span aria-hidden="true">sum</span>
            <div className="autoencoder-bottleneck-gradient">
              <small>bottleneck gradient</small>
              <strong>{formatNumber(trace.backward.bottleneckGradient)}</strong>
            </div>
          </div>

          <div className="autoencoder-gradient-grid">
            <div>
              <small>decoder weight gradients</small>
              <code>{formatVector(trace.backward.decoderWeightGradients)}</code>
            </div>
            <div>
              <small>decoder bias gradients</small>
              <code>{formatVector(trace.backward.decoderBiasGradients)}</code>
            </div>
            <div>
              <small>encoder weight gradients</small>
              <code>{formatVector(trace.backward.encoderWeightGradients)}</code>
            </div>
            <div>
              <small>encoder bias gradient</small>
              <code>{formatNumber(trace.backward.encoderBiasGradient)}</code>
            </div>
          </div>
        </section>

        <section className="autoencoder-update-panel" aria-label="Autoencoder SGD update and gradient audit">
          <div className="autoencoder-heading">
            <div>
              <p className="eyebrow">All seven parameters move together</p>
              <h2>Audit, update, rerun</h2>
            </div>
            <code>parameter - {trace.learningRate} x gradient</code>
          </div>

          <div className="autoencoder-parameter-grid">
            <div>
              <small>encoder before</small>
              <code>w {formatVector(trace.parameters.encoder.weights)}</code>
              <code>b {formatNumber(trace.parameters.encoder.bias)}</code>
            </div>
            <div>
              <small>encoder after</small>
              <code>w {formatVector(trace.updatedParameters.encoder.weights)}</code>
              <code>b {formatNumber(trace.updatedParameters.encoder.bias)}</code>
            </div>
            <div>
              <small>decoder before</small>
              <code>w {formatVector(trace.parameters.decoder.weights)}</code>
              <code>b {formatVector(trace.parameters.decoder.bias)}</code>
            </div>
            <div>
              <small>decoder after</small>
              <code>w {formatVector(trace.updatedParameters.decoder.weights)}</code>
              <code>b {formatVector(trace.updatedParameters.decoder.bias)}</code>
            </div>
          </div>

          <div className="autoencoder-gradient-audit">
            <span>Central finite differences - 7 parameters</span>
            <code>epsilon = {trace.gradientCheck.epsilon}</code>
            <strong>max error {trace.gradientCheck.maxAbsoluteError.toExponential(3)}</strong>
          </div>

          <div className="autoencoder-loss-drop">
            <div><small>loss before</small><strong>{formatNumber(trace.forward.loss)}</strong></div>
            <span aria-hidden="true">-&gt;</span>
            <div><small>loss after</small><strong>{formatNumber(trace.postUpdate.loss)}</strong></div>
            <p>One reconstruction improves sharply; the shared mean objective falls.</p>
          </div>
        </section>
      </section>

      <aside className="autoencoder-controls" aria-label="Autoencoder trace controls">
        <p className="eyebrow">Open one decoder branch</p>
        <h2>Bottleneck controls</h2>
        <p>
          Both outputs stay visible. Selection follows one reconstruction's
          arithmetic and gradient route without disconnecting the shared scalar.
        </p>

        <div className="attention-query-buttons" aria-label="Autoencoder reconstruction selection">
          {[0, 1].map((index) => (
            <button
              aria-pressed={selectedOutput === index}
              key={index}
              type="button"
              onClick={() => setSelectedOutput(index)}
            >
              output {index}
            </button>
          ))}
        </div>

        <label className="attention-scale-control">
          <input
            type="checkbox"
            checked={showUpdated}
            onChange={(event) => setShowUpdated(event.target.checked)}
          />
          <span>
            <strong>Use updated parameters</strong>
            <small>Rerun encode, decode, and loss after one SGD step.</small>
          </span>
        </label>

        <div className="attention-selected-summary">
          <small>selected reconstruction</small>
          <strong>x_hat{selectedOutput}</strong>
          <span>
            {formatNumber(currentForward.reconstruction[selectedOutput]!)} versus target {formatNumber(trace.input[selectedOutput]!)}
          </span>
        </div>

        <div className="attention-value-boundary">
          <span>What is actually compressed?</span>
          <p>
            The decoder receives z only. It cannot inspect either original
            coordinate while rebuilding the pair.
          </p>
        </div>

        <div className="attention-next-note">
          <span>Keep the claim small</span>
          <p>
            One example explains the mechanics. A useful representation needs
            many examples to reveal a shared lower-dimensional pattern.
          </p>
        </div>
      </aside>
    </main>
  );
}
