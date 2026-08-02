import { useMemo, useState } from "react";
import { traceScalarVariationalAutoencoder } from "./variational-lab.js";

const BETA_OPTIONS = [0, 0.1, 0.25, 1];

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  return Number(value.toFixed(8)).toString();
}

export function VariationalWorkbench() {
  const [beta, setBeta] = useState(0.1);
  const [gradientTarget, setGradientTarget] = useState<"mean" | "logVariance">("mean");
  const [showUpdated, setShowUpdated] = useState(false);
  const trace = useMemo(
    () => traceScalarVariationalAutoencoder(beta),
    [beta],
  );
  const currentForward = showUpdated ? trace.postUpdate : trace.forward;
  const currentParameters = showUpdated
    ? trace.updatedParameters
    : trace.parameters;
  const reconstructionRoute = gradientTarget === "mean"
    ? trace.backward.reconstructionMeanGradient
    : trace.backward.reconstructionLogVarianceGradient;
  const klRoute = gradientTarget === "mean"
    ? trace.backward.weightedKlMeanGradient
    : trace.backward.weightedKlLogVarianceGradient;
  const combinedRoute = gradientTarget === "mean"
    ? trace.backward.meanGradient
    : trace.backward.logVarianceGradient;
  const gradientTargetLabel = gradientTarget === "mean"
    ? "mean"
    : "log-variance";

  return (
    <main className="workspace workspace--variational">
      <section className="variational-stage" aria-label="Scalar variational autoencoder trace">
        <div className="variational-intro">
          <div>
            <p className="eyebrow">NN17 - uncertainty without hidden randomness</p>
            <h2>One Gaussian latent sample, fully unpacked</h2>
            <p>
              Encode a mean and log-variance, transform one saved noise value,
              then watch reconstruction and prior matching negotiate one update.
            </p>
          </div>
          <div className="variational-chip">mean + sigma x epsilon</div>
        </div>

        <section className="variational-flow-panel" aria-label="Variational encode sample and decode path">
          <div className="variational-heading">
            <div>
              <p className="eyebrow">The sample is random; the path is differentiable</p>
              <h2>Move noise outside the network</h2>
            </div>
            <code>{showUpdated ? "after one SGD step" : "saved epsilon = 0.5"}</code>
          </div>

          <div className="variational-flow">
            <div className="variational-scalar-node">
              <small>input is target</small>
              <strong>x = {formatNumber(trace.input)}</strong>
            </div>
            <span className="variational-arrow" aria-hidden="true">-&gt;</span>
            <div className="variational-distribution-node">
              <small>encoder distribution</small>
              <code>
                mean = {formatNumber(currentForward.meanProduct)} + {formatNumber(currentParameters.encoder.mean.bias)} = {formatNumber(currentForward.mean)}
              </code>
              <code>
                log var = {formatNumber(currentForward.logVarianceProduct)} + {formatNumber(currentParameters.encoder.logVariance.bias)} = {formatNumber(currentForward.logVariance)}
              </code>
              <code>sigma = {formatNumber(currentForward.standardDeviation)}</code>
            </div>
            <span className="variational-arrow" aria-hidden="true">-&gt;</span>
            <div className="variational-sample-node">
              <small>reparameterized sample</small>
              <code>
                {formatNumber(currentForward.mean)} + {formatNumber(currentForward.standardDeviation)} x {formatNumber(currentForward.epsilon)}
              </code>
              <strong>z = {formatNumber(currentForward.latent)}</strong>
              <span>epsilon stays fixed for this audit</span>
            </div>
            <span className="variational-arrow" aria-hidden="true">-&gt;</span>
            <div className="variational-scalar-node variational-scalar-node--output">
              <small>decoder reconstruction</small>
              <code>
                {formatNumber(currentForward.latent)} x {formatNumber(currentParameters.decoder.weight)} + {formatNumber(currentParameters.decoder.bias)}
              </code>
              <strong>x_hat = {formatNumber(currentForward.reconstruction)}</strong>
            </div>
          </div>
        </section>

        <section className="variational-objective-panel" aria-label="Variational reconstruction and KL objective">
          <div className="variational-heading">
            <div>
              <p className="eyebrow">Two pressures, one weighted objective</p>
              <h2>Reconstruct here; stay sampleable everywhere</h2>
            </div>
            <div className="variational-loss-badge">
              <small>total loss</small>
              <strong>{formatNumber(currentForward.totalLoss)}</strong>
            </div>
          </div>

          <div className="variational-objective-equation">
            <div>
              <small>reconstruction</small>
              <code>0.5 x ({formatNumber(currentForward.error)})^2</code>
              <strong>{formatNumber(currentForward.reconstructionLoss)}</strong>
              <span>preserve this input</span>
            </div>
            <span aria-hidden="true">+</span>
            <div>
              <small>KL to Normal(0, 1)</small>
              <code>
                0.5 x ({formatNumber(currentForward.meanSquared)} + {formatNumber(currentForward.variance)} - 1 - {formatNumber(currentForward.logVariance)})
              </code>
              <strong>{formatNumber(currentForward.kl)}</strong>
              <span>keep latent space sampleable</span>
            </div>
            <span aria-hidden="true">x</span>
            <div className="variational-beta-node">
              <small>beta</small>
              <strong>{formatNumber(beta)}</strong>
            </div>
            <span aria-hidden="true">=</span>
            <div className="variational-total-node">
              <small>weighted total</small>
              <code>
                {formatNumber(currentForward.reconstructionLoss)} + {formatNumber(currentForward.weightedKl)}
              </code>
              <strong>{formatNumber(currentForward.totalLoss)}</strong>
            </div>
          </div>
        </section>

        <section className="variational-gradient-panel" aria-label={`Variational ${gradientTargetLabel} gradient tradeoff`}>
          <div className="variational-heading">
            <div>
              <p className="eyebrow">Both objectives meet at the encoder</p>
              <h2>Beta can reinforce, soften, or reverse a direction</h2>
            </div>
            <code>saved forward pass gradients</code>
          </div>

          <div className="variational-gradient-targets" aria-label="Variational gradient target">
            <button
              aria-pressed={gradientTarget === "mean"}
              type="button"
              onClick={() => setGradientTarget("mean")}
            >
              mean output
            </button>
            <button
              aria-pressed={gradientTarget === "logVariance"}
              type="button"
              onClick={() => setGradientTarget("logVariance")}
            >
              log-variance output
            </button>
          </div>

          <div className="variational-gradient-routes">
            <div>
              <small>reconstruction route</small>
              <strong>{formatNumber(reconstructionRoute)}</strong>
              <span>sample should rebuild x</span>
            </div>
            <span aria-hidden="true">+</span>
            <div>
              <small>beta x KL route</small>
              <code>
                {formatNumber(beta)} x {formatNumber(gradientTarget === "mean" ? trace.backward.klMeanGradient : trace.backward.klLogVarianceGradient)}
              </code>
              <strong>{formatNumber(klRoute)}</strong>
              <span>distribution should match prior</span>
            </div>
            <span aria-hidden="true">=</span>
            <div className="variational-combined-gradient">
              <small>combined {gradientTargetLabel} gradient</small>
              <strong>{formatNumber(combinedRoute)}</strong>
              <span>
                {combinedRoute === 0 ? "the routes cancel exactly" : "this is the encoder's update direction"}
              </span>
            </div>
          </div>

          <div className="variational-gradient-grid">
            <div><small>decoder weight</small><code>{formatNumber(trace.backward.decoderWeightGradient)}</code></div>
            <div><small>decoder bias</small><code>{formatNumber(trace.backward.decoderBiasGradient)}</code></div>
            <div><small>mean weight / bias</small><code>{formatNumber(trace.backward.meanWeightGradient)} / {formatNumber(trace.backward.meanBiasGradient)}</code></div>
            <div><small>log-var weight / bias</small><code>{formatNumber(trace.backward.logVarianceWeightGradient)} / {formatNumber(trace.backward.logVarianceBiasGradient)}</code></div>
          </div>
        </section>

        <section className="variational-update-panel" aria-label="Variational SGD update and gradient audit">
          <div className="variational-heading">
            <div>
              <p className="eyebrow">Same epsilon for analytical and numerical slopes</p>
              <h2>Audit six parameters, then rerun everything</h2>
            </div>
            <code>parameter - {trace.learningRate} x gradient</code>
          </div>

          <div className="variational-parameter-grid">
            <div>
              <small>mean head before -&gt; after</small>
              <code>w {formatNumber(trace.parameters.encoder.mean.weight)} -&gt; {formatNumber(trace.updatedParameters.encoder.mean.weight)}</code>
              <code>b {formatNumber(trace.parameters.encoder.mean.bias)} -&gt; {formatNumber(trace.updatedParameters.encoder.mean.bias)}</code>
            </div>
            <div>
              <small>log-var head before -&gt; after</small>
              <code>w {formatNumber(trace.parameters.encoder.logVariance.weight)} -&gt; {formatNumber(trace.updatedParameters.encoder.logVariance.weight)}</code>
              <code>b {formatNumber(trace.parameters.encoder.logVariance.bias)} -&gt; {formatNumber(trace.updatedParameters.encoder.logVariance.bias)}</code>
            </div>
            <div>
              <small>decoder before -&gt; after</small>
              <code>w {formatNumber(trace.parameters.decoder.weight)} -&gt; {formatNumber(trace.updatedParameters.decoder.weight)}</code>
              <code>b {formatNumber(trace.parameters.decoder.bias)} -&gt; {formatNumber(trace.updatedParameters.decoder.bias)}</code>
            </div>
          </div>

          <div className="variational-audit-row">
            <span>Central finite differences - 6 parameters</span>
            <code>epsilon = {trace.gradientCheck.epsilon}</code>
            <strong>max error {trace.gradientCheck.maxAbsoluteError.toExponential(3)}</strong>
          </div>

          <div className="variational-loss-drop">
            <div><small>total before</small><strong>{formatNumber(trace.forward.totalLoss)}</strong></div>
            <span aria-hidden="true">-&gt;</span>
            <div><small>total after</small><strong>{formatNumber(trace.postUpdate.totalLoss)}</strong></div>
            <p>
              Reconstruction falls from {formatNumber(trace.forward.reconstructionLoss)} to {formatNumber(trace.postUpdate.reconstructionLoss)}; KL may move differently while the selected weighted objective falls.
            </p>
          </div>
        </section>
      </section>

      <aside className="variational-controls" aria-label="Variational trace controls">
        <p className="eyebrow">Turn the prior pressure</p>
        <h2>KL tradeoff controls</h2>
        <p>
          Epsilon stays fixed at 0.5. Changing beta therefore changes the
          objective and gradient, not the sampled noise.
        </p>

        <div className="variational-beta-buttons" aria-label="Variational beta selection">
          {BETA_OPTIONS.map((option) => (
            <button
              aria-pressed={beta === option}
              key={option}
              type="button"
              onClick={() => {
                setBeta(option);
                setShowUpdated(false);
              }}
            >
              beta {option}
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
            <small>Rerun distribution, sample, decoder, and both losses.</small>
          </span>
        </label>

        <div className="variational-selected-summary">
          <small>selected beta</small>
          <strong>{formatNumber(beta)}</strong>
          <span>
            mean gradient {formatNumber(trace.backward.meanGradient)}; total {formatNumber(trace.forward.totalLoss)}
          </span>
        </div>

        <div className="attention-value-boundary">
          <span>Why save epsilon?</span>
          <p>
            The trace remains stochastic in meaning but reproducible in
            execution. Finite differences compare the same noise on both sides.
          </p>
        </div>

        <div className="attention-next-note">
          <span>Do not optimize one term alone</span>
          <p>
            A useful VAE needs reconstruction and a navigable latent prior.
            Their weighted sum, not either isolated term, defines this step.
          </p>
        </div>
      </aside>
    </main>
  );
}
