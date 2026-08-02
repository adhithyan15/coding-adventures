import { useMemo, useState } from "react";
import {
  traceOneDimensionalGan,
  type GanState,
} from "./gan-lab.js";

type GanPhase = "initial" | "discriminator" | "generator";

const PHASES: Array<{ value: GanPhase; label: string; shortLabel: string }> = [
  { value: "initial", label: "Before training", shortLabel: "0. Forward" },
  {
    value: "discriminator",
    label: "Discriminator moves",
    shortLabel: "1. Critic",
  },
  {
    value: "generator",
    label: "Generator responds",
    shortLabel: "2. Maker",
  },
];

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  return Number(value.toFixed(8)).toString();
}

function phaseDescription(phase: GanPhase): string {
  if (phase === "discriminator") {
    return "The fake sample is detached. Only the discriminator can move.";
  }
  if (phase === "generator") {
    return "The updated discriminator is frozen. Its input gradient teaches the generator.";
  }
  return "Both players make predictions, but neither has moved yet.";
}

function SampleNumberLine({ state, realSample }: { state: GanState; realSample: number }) {
  const position = (value: number) => `${Math.max(3, Math.min(97, value * 72 + 12))}%`;
  return (
    <div className="gan-number-line" aria-label="GAN sample number line">
      <div className="gan-number-line__axis" aria-hidden="true">
        <span>0</span><span>0.5</span><span>1</span>
      </div>
      <div
        className="gan-number-line__marker gan-number-line__marker--fake"
        style={{ left: position(state.fakeSample) }}
      >
        <strong>fake {formatNumber(state.fakeSample)}</strong>
        <small>G(noise)</small>
      </div>
      <div
        className="gan-number-line__marker gan-number-line__marker--real"
        style={{ left: position(realSample) }}
      >
        <strong>real {formatNumber(realSample)}</strong>
        <small>data</small>
      </div>
    </div>
  );
}

export function GanWorkbench() {
  const [phase, setPhase] = useState<GanPhase>("initial");
  const trace = useMemo(() => traceOneDimensionalGan(), []);
  const state = phase === "initial"
    ? trace.initial
    : phase === "discriminator"
      ? trace.discriminatorStep.state
      : trace.generatorStep.state;
  const discriminator = phase === "initial"
    ? trace.parameters.discriminator
    : trace.discriminatorStep.updatedParameters;
  const generator = phase === "generator"
    ? trace.generatorStep.updatedParameters
    : trace.parameters.generator;

  return (
    <main className="workspace workspace--gan">
      <section className="gan-stage" aria-label="One-dimensional GAN game trace">
        <div className="gan-intro">
          <div>
            <p className="eyebrow">NN18 - two losses, two turns, one game</p>
            <h2>A generator and discriminator on one number line</h2>
            <p>
              The critic learns to separate one real point from one generated
              point. Then the maker follows the frozen critic&apos;s slope toward a
              more convincing sample.
            </p>
          </div>
          <div className="gan-chip">D moves -&gt; freeze D -&gt; G moves</div>
        </div>

        <section className="gan-sample-panel" aria-label="GAN samples and discriminator probabilities">
          <div className="gan-heading">
            <div>
              <p className="eyebrow">Same saved noise through every phase</p>
              <h2>Watch the fake sample move toward the data</h2>
            </div>
            <code>{phaseDescription(phase)}</code>
          </div>
          <SampleNumberLine state={state} realSample={trace.realSample} />
          <div className="gan-probability-grid">
            <div>
              <small>critic on real</small>
              <code>sigmoid({formatNumber(state.realLogit)})</code>
              <strong>{formatNumber(state.realProbability)}</strong>
            </div>
            <div>
              <small>critic on fake</small>
              <code>sigmoid({formatNumber(state.fakeLogit)})</code>
              <strong>{formatNumber(state.fakeProbability)}</strong>
            </div>
            <div>
              <small>generator equation</small>
              <code>
                {formatNumber(trace.savedNoise)} x {formatNumber(generator.weight)} + {formatNumber(generator.bias)}
              </code>
              <strong>{formatNumber(state.fakeSample)}</strong>
            </div>
          </div>
        </section>

        <section className="gan-objective-panel" aria-label="GAN competing objectives">
          <div className="gan-heading">
            <div>
              <p className="eyebrow">The players do not minimize one shared loss</p>
              <h2>Judge correctly; fool the judge</h2>
            </div>
            <code>non-saturating generator objective</code>
          </div>
          <div className="gan-objectives">
            <div className={phase === "discriminator" ? "gan-player gan-player--active" : "gan-player"}>
              <small>discriminator minimizes</small>
              <code>-0.5 x [log D(real) + log(1 - D(fake))]</code>
              <strong>D loss {formatNumber(state.discriminatorLoss)}</strong>
              <span>real label 1, fake label 0</span>
            </div>
            <div className="gan-versus" aria-hidden="true">vs</div>
            <div className={phase === "generator" ? "gan-player gan-player--active gan-player--generator" : "gan-player gan-player--generator"}>
              <small>generator minimizes</small>
              <code>-log D(G(noise))</code>
              <strong>G loss {formatNumber(state.generatorLoss)}</strong>
              <span>make the fake receive label 1</span>
            </div>
          </div>
        </section>

        <section className="gan-gradient-panel" aria-label="GAN active gradient route">
          <div className="gan-heading">
            <div>
              <p className="eyebrow">Only one parameter set moves per turn</p>
              <h2>{phase === "generator" ? "The critic becomes a teaching signal" : phase === "discriminator" ? "The generated value is detached" : "Choose a move to reveal its gradient"}</h2>
            </div>
            <code>{phase === "initial" ? "forward pass only" : "active route highlighted"}</code>
          </div>
          {phase === "initial" ? (
            <div className="gan-gradient-placeholder">
              Start with two sigmoid scores. The turn buttons expose which
              edges carry gradients and which parameter set stays frozen.
            </div>
          ) : phase === "discriminator" ? (
            <div className="gan-gradient-route">
              <div>
                <small>real-logit route</small>
                <code>0.5 x (D(real) - 1)</code>
                <strong>{formatNumber(trace.discriminatorStep.backward.realLogitGradient)}</strong>
              </div>
              <span aria-hidden="true">+</span>
              <div>
                <small>fake-logit route</small>
                <code>0.5 x D(fake)</code>
                <strong>{formatNumber(trace.discriminatorStep.backward.fakeLogitGradient)}</strong>
              </div>
              <span aria-hidden="true">-&gt;</span>
              <div className="gan-gradient-route__result">
                <small>D weight / bias gradient</small>
                <strong>
                  {formatNumber(trace.discriminatorStep.backward.weightGradient)} / {formatNumber(trace.discriminatorStep.backward.biasGradient)}
                </strong>
                <span>gradient into fake = 0 (detached)</span>
              </div>
            </div>
          ) : (
            <div className="gan-gradient-route">
              <div>
                <small>G loss to fake logit</small>
                <code>D(fake) - 1</code>
                <strong>{formatNumber(trace.generatorStep.backward.fakeLogitGradient)}</strong>
              </div>
              <span aria-hidden="true">x</span>
              <div>
                <small>frozen D input slope</small>
                <code>D weight = {formatNumber(discriminator.weight)}</code>
                <strong>{formatNumber(trace.generatorStep.backward.fakeSampleGradient)}</strong>
              </div>
              <span aria-hidden="true">-&gt;</span>
              <div className="gan-gradient-route__result gan-gradient-route__result--generator">
                <small>G weight / bias gradient</small>
                <strong>
                  {formatNumber(trace.generatorStep.backward.weightGradient)} / {formatNumber(trace.generatorStep.backward.biasGradient)}
                </strong>
                <span>D parameters stay frozen</span>
              </div>
            </div>
          )}
        </section>

        <section className="gan-update-panel" aria-label="GAN alternating updates and gradient audits">
          <div className="gan-heading">
            <div>
              <p className="eyebrow">Audit each player against its own objective</p>
              <h2>The losses push back after alternating moves</h2>
            </div>
            <code>central difference epsilon = 1e-6</code>
          </div>
          <div className="gan-update-grid">
            <div>
              <small>discriminator update</small>
              <code>w {formatNumber(trace.parameters.discriminator.weight)} -&gt; {formatNumber(trace.discriminatorStep.updatedParameters.weight)}</code>
              <code>b {formatNumber(trace.parameters.discriminator.bias)} -&gt; {formatNumber(trace.discriminatorStep.updatedParameters.bias)}</code>
              <strong>D loss {formatNumber(trace.initial.discriminatorLoss)} -&gt; {formatNumber(trace.discriminatorStep.state.discriminatorLoss)}</strong>
              <span>max audit error {trace.discriminatorStep.gradientCheck.maxAbsoluteError.toExponential(3)}</span>
            </div>
            <div>
              <small>generator counter-move</small>
              <code>w {formatNumber(trace.parameters.generator.weight)} -&gt; {formatNumber(trace.generatorStep.updatedParameters.weight)}</code>
              <code>b {formatNumber(trace.parameters.generator.bias)} -&gt; {formatNumber(trace.generatorStep.updatedParameters.bias)}</code>
              <strong>G loss {formatNumber(trace.discriminatorStep.state.generatorLoss)} -&gt; {formatNumber(trace.generatorStep.state.generatorLoss)}</strong>
              <span>max audit error {trace.generatorStep.gradientCheck.maxAbsoluteError.toExponential(3)}</span>
            </div>
          </div>
          <div className="gan-counterpush">
            <strong>After G moves, D loss rises to {formatNumber(trace.generatorStep.state.discriminatorLoss)}.</strong>
            <p>That is the game working: the newly improved fake is harder for the frozen critic.</p>
          </div>
        </section>
      </section>

      <aside className="gan-controls" aria-label="GAN game phase controls">
        <p className="eyebrow">Alternating schedule</p>
        <h2>Advance one turn</h2>
        <p>
          These are snapshots of one deterministic round, not three independent
          experiments.
        </p>
        <div className="gan-phase-buttons">
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
        <div className="gan-selected-summary">
          <small>current snapshot</small>
          <strong>{PHASES.find((candidate) => candidate.value === phase)!.label}</strong>
          <span>fake = {formatNumber(state.fakeSample)}</span>
          <span>D(fake) = {formatNumber(state.fakeProbability)}</span>
        </div>
        <div className="gan-freeze-key">
          <small>freeze contract</small>
          <code>{phase === "discriminator" ? "grad(G) = 0" : phase === "generator" ? "grad(D params) = 0" : "no backward pass"}</code>
        </div>
      </aside>
    </main>
  );
}
