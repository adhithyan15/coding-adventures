import { useMemo, useState } from "react";
import { traceOneDimensionalDiffusion } from "./diffusion-lab.js";

type DiffusionPhase =
  | "clean"
  | "forward1"
  | "forward2"
  | "learn"
  | "reverse2"
  | "reverse1";

const PHASES: Array<{
  value: DiffusionPhase;
  shortLabel: string;
  label: string;
}> = [
  { value: "clean", shortLabel: "0. Data", label: "Clean sample" },
  { value: "forward1", shortLabel: "1. Forward", label: "Noise level 1" },
  { value: "forward2", shortLabel: "2. Forward", label: "Noise level 2" },
  { value: "learn", shortLabel: "3. Learn", label: "Predict saved noise" },
  { value: "reverse2", shortLabel: "4. Reverse", label: "Denoise step 2" },
  { value: "reverse1", shortLabel: "5. Reverse", label: "Denoise step 1" },
];

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  return Number(value.toFixed(8)).toString();
}

export function DiffusionWorkbench() {
  const [phase, setPhase] = useState<DiffusionPhase>("clean");
  const trace = useMemo(() => traceOneDimensionalDiffusion(), []);
  const phaseIndex = PHASES.findIndex((candidate) => candidate.value === phase);
  const showLearned = phaseIndex >= 3;
  const predictionRows = showLearned
    ? trace.postUpdateDenoising
    : trace.initialDenoising;
  const predictionLoss = showLearned
    ? trace.postUpdateMeanLoss
    : trace.initialMeanLoss;
  const parameters = showLearned ? trace.updatedDenoiser : trace.denoiser;
  const reverseTwoVisible = phaseIndex >= 4;
  const reverseOneVisible = phaseIndex >= 5;
  const selectedValue = phase === "clean"
    ? trace.cleanSample
    : phase === "forward1"
      ? trace.forwardSteps[0]!.noisySample
      : phase === "reverse2"
        ? trace.reverseSteps[0]!.outputMean
        : phase === "reverse1"
          ? trace.finalReconstruction
          : trace.forwardSteps[1]!.noisySample;

  return (
    <main className="workspace workspace--diffusion">
      <section className="diffusion-stage" aria-label="One-dimensional diffusion trace">
        <div className="diffusion-intro">
          <div>
            <p className="eyebrow">NN19 - add known noise, then learn to remove it</p>
            <h2>One clean number through a diffusion round trip</h2>
            <p>
              Trade signal for one saved noise value at two known levels,
              train a timestep-aware predictor, and follow its deterministic
              reverse mean back toward the data.
            </p>
          </div>
          <div className="diffusion-chip">x0 -&gt; x1 -&gt; x2 -&gt; mean1 -&gt; mean0</div>
        </div>

        <section className="diffusion-forward-panel" aria-label="Diffusion forward noise schedule">
          <div className="diffusion-heading">
            <div>
              <p className="eyebrow">One epsilon, two comparable noise levels</p>
              <h2>Signal shrinks while noise grows</h2>
            </div>
            <code>saved epsilon = {formatNumber(trace.savedNoise)}</code>
          </div>
          <div className="diffusion-forward-lane">
            <div className={phase === "clean" ? "diffusion-state diffusion-state--active" : "diffusion-state"}>
              <small>clean data</small>
              <strong>x0 = {formatNumber(trace.cleanSample)}</strong>
              <span>100% signal</span>
            </div>
            {trace.forwardSteps.map((step, index) => (
              <div className="diffusion-forward-hop" key={step.t}>
                <span aria-hidden="true">+ noise</span>
                <div className={phase === `forward${step.t}` ? "diffusion-state diffusion-state--active diffusion-state--noisy" : "diffusion-state diffusion-state--noisy"}>
                  <small>noise level {step.t}</small>
                  <code>
                    {formatNumber(step.signalScale)} x {formatNumber(trace.cleanSample)} + {formatNumber(step.noiseScale)} x ({formatNumber(trace.savedNoise)})
                  </code>
                  <strong>x{step.t} = {formatNumber(step.noisySample)}</strong>
                  <span>alpha_bar = {formatNumber(step.alphaBar)}</span>
                </div>
              </div>
            ))}
          </div>
          <div className="diffusion-coefficient-grid">
            {trace.forwardSteps.map((step) => (
              <div key={step.t}>
                <small>level {step.t} contributions</small>
                <code>signal {formatNumber(step.signalContribution)}</code>
                <code>noise {formatNumber(step.noiseContribution)}</code>
                <strong>{formatNumber(step.signalContribution)} + {formatNumber(step.noiseContribution)} = {formatNumber(step.noisySample)}</strong>
              </div>
            ))}
          </div>
          <p className="diffusion-forward-note">
            Each row samples directly from x0 with the same saved epsilon. That
            makes coefficient changes comparable; it is not one Markov noise path.
          </p>
        </section>

        <section className="diffusion-predict-panel" aria-label="Diffusion noise prediction objective">
          <div className="diffusion-heading">
            <div>
              <p className="eyebrow">The model predicts corruption, not x0 directly</p>
              <h2>Condition the denoiser on sample and timestep</h2>
            </div>
            <div className="diffusion-loss-badge">
              <small>{showLearned ? "mean loss after SGD" : "initial mean loss"}</small>
              <strong>{formatNumber(predictionLoss)}</strong>
            </div>
          </div>
          <div className="diffusion-equation">
            <code>
              epsilon_hat = {formatNumber(parameters.sampleWeight)} x x_t + {formatNumber(parameters.timestepWeight)} x normalized_t + {formatNumber(parameters.bias)}
            </code>
          </div>
          <div className="diffusion-prediction-grid">
            {predictionRows.map((row) => (
              <div key={row.t}>
                <small>level {row.t}, normalized t = {formatNumber(row.normalizedT)}</small>
                <code>input x{row.t} = {formatNumber(row.noisySample)}</code>
                <strong>predicted {formatNumber(row.predictedNoise)}</strong>
                <span>target {formatNumber(row.targetNoise)}</span>
                <span>half-squared loss {formatNumber(row.loss)}</span>
              </div>
            ))}
          </div>
        </section>

        <section className="diffusion-gradient-panel" aria-label="Diffusion denoiser gradient and update">
          <div className="diffusion-heading">
            <div>
              <p className="eyebrow">Both timesteps train one shared denoiser</p>
              <h2>Add row contributions, audit, then update</h2>
            </div>
            <code>parameter - {formatNumber(trace.learningRate)} x gradient</code>
          </div>
          <div className="diffusion-gradient-rows">
            {trace.backward.perStep.map((row) => (
              <div key={row.t}>
                <small>level {row.t}</small>
                <code>dL / d prediction = {formatNumber(row.predictionGradient)}</code>
                <span>sample-w route {formatNumber(row.sampleWeightContribution)}</span>
                <span>time-w route {formatNumber(row.timestepWeightContribution)}</span>
                <span>bias route {formatNumber(row.biasContribution)}</span>
              </div>
            ))}
          </div>
          <div className="diffusion-gradient-sum">
            <div><small>sample weight gradient</small><strong>{formatNumber(trace.backward.sampleWeightGradient)}</strong></div>
            <div><small>timestep weight gradient</small><strong>{formatNumber(trace.backward.timestepWeightGradient)}</strong></div>
            <div><small>bias gradient</small><strong>{formatNumber(trace.backward.biasGradient)}</strong></div>
          </div>
          <div className="diffusion-update-row">
            <div>
              <small>parameters before -&gt; after</small>
              <code>sample w {formatNumber(trace.denoiser.sampleWeight)} -&gt; {formatNumber(trace.updatedDenoiser.sampleWeight)}</code>
              <code>time w {formatNumber(trace.denoiser.timestepWeight)} -&gt; {formatNumber(trace.updatedDenoiser.timestepWeight)}</code>
              <code>bias {formatNumber(trace.denoiser.bias)} -&gt; {formatNumber(trace.updatedDenoiser.bias)}</code>
            </div>
            <div>
              <small>central finite-difference audit</small>
              <strong>3 parameters</strong>
              <code>max error {trace.gradientCheck.maxAbsoluteError.toExponential(3)}</code>
            </div>
            <div className="diffusion-loss-drop">
              <small>same two rows rerun</small>
              <strong>{formatNumber(trace.initialMeanLoss)} -&gt; {formatNumber(trace.postUpdateMeanLoss)}</strong>
              <span>noise prediction improves</span>
            </div>
          </div>
        </section>

        <section className="diffusion-reverse-panel" aria-label="Diffusion deterministic reverse mean path">
          <div className="diffusion-heading">
            <div>
              <p className="eyebrow">Subtract predicted noise one level at a time</p>
              <h2>Run the updated model backward</h2>
            </div>
            <code>no fresh reverse noise in this audit</code>
          </div>
          <div className="diffusion-reverse-lane">
            <div className="diffusion-state diffusion-state--noisy">
              <small>start at noisiest sample</small>
              <strong>x2 = {formatNumber(trace.forwardSteps[1]!.noisySample)}</strong>
            </div>
            <span aria-hidden="true">-&gt;</span>
            <div className={phase === "reverse2" ? "diffusion-reverse-step diffusion-reverse-step--active" : "diffusion-reverse-step"}>
              <small>reverse t = 2</small>
              {reverseTwoVisible ? (
                <>
                  <code>{formatNumber(trace.reverseSteps[0]!.inputSample)} - ({formatNumber(trace.reverseSteps[0]!.scaledNoiseCorrection)})</code>
                  <strong>mean1 = {formatNumber(trace.reverseSteps[0]!.outputMean)}</strong>
                  <span>predicted noise {formatNumber(trace.reverseSteps[0]!.predictedNoise)}</span>
                </>
              ) : <strong>?</strong>}
            </div>
            <span aria-hidden="true">-&gt;</span>
            <div className={phase === "reverse1" ? "diffusion-reverse-step diffusion-reverse-step--active" : "diffusion-reverse-step"}>
              <small>reverse t = 1</small>
              {reverseOneVisible ? (
                <>
                  <code>{formatNumber(trace.reverseSteps[1]!.inputSample)} - ({formatNumber(trace.reverseSteps[1]!.scaledNoiseCorrection)})</code>
                  <strong>mean0 = {formatNumber(trace.finalReconstruction)}</strong>
                  <span>predicted noise {formatNumber(trace.reverseSteps[1]!.predictedNoise)}</span>
                </>
              ) : <strong>?</strong>}
            </div>
            <span aria-hidden="true">-&gt;</span>
            <div className="diffusion-final-state">
              <small>reconstructed clean sample</small>
              <strong>{reverseOneVisible ? formatNumber(trace.finalReconstruction) : "?"}</strong>
              <span>{reverseOneVisible ? `absolute error ${formatNumber(trace.finalAbsoluteError)}` : "finish both reverse means"}</span>
            </div>
          </div>
        </section>
      </section>

      <aside className="diffusion-controls" aria-label="Diffusion phase controls">
        <p className="eyebrow">Round-trip schedule</p>
        <h2>Advance the process</h2>
        <p>
          Forward levels share saved noise. Reverse levels reuse the learned
          denoiser but feed each generated mean into the next step.
        </p>
        <div className="diffusion-phase-buttons">
          {PHASES.map((candidate) => (
            <button
              key={candidate.value}
              type="button"
              aria-pressed={phase === candidate.value}
              onClick={() => setPhase(candidate.value)}
            >
              <span>{candidate.shortLabel}</span>
              <strong>{candidate.label}</strong>
            </button>
          ))}
        </div>
        <div className="diffusion-selected-summary">
          <small>selected state</small>
          <strong>{PHASES[phaseIndex]!.label}</strong>
          <span>visible scalar = {formatNumber(selectedValue)}</span>
          <span>{showLearned ? "updated denoiser" : "initial denoiser"}</span>
        </div>
      </aside>
    </main>
  );
}
