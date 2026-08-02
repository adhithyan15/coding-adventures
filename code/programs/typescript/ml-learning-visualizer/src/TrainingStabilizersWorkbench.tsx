import { useMemo, useState } from "react";
import {
  TRAINING_STABILIZER_ROUTES,
  traceTrainingStabilizers,
  type TrainingStabilizerRouteId,
} from "./training-stabilizers-lab.js";

function formatNumber(value: number, digits = 6): string {
  if (Math.abs(value) < 1e-12) return "0";
  if (Math.abs(value) < 0.0001 || Math.abs(value) >= 1000) return value.toExponential(3);
  return Number(value.toFixed(digits)).toString();
}

function formatVector(values: readonly number[]): string {
  return `[${values.map((value) => formatNumber(value)).join(", ")}]`;
}

function VectorStrip({
  label,
  values,
  selectedCoordinate,
  tone = "blue",
}: {
  label: string;
  values: readonly number[];
  selectedCoordinate: number;
  tone?: "blue" | "green" | "purple" | "red";
}) {
  return (
    <div className={`stabilizer-vector stabilizer-vector--${tone}`}>
      <span>{label}</span>
      <div>
        {values.map((value, index) => (
          <code className={selectedCoordinate === index ? "is-selected" : ""} key={`${label}-${index}`}>
            <small>{index + 1}</small>
            {formatNumber(value)}
          </code>
        ))}
      </div>
    </div>
  );
}

export function TrainingStabilizersWorkbench() {
  const [routeId, setRouteId] = useState<TrainingStabilizerRouteId>("plain");
  const [selectedCoordinate, setSelectedCoordinate] = useState(0);
  const trace = useMemo(() => traceTrainingStabilizers(), []);
  const route = trace.routes.find((item) => item.id === routeId)!;
  const definition = TRAINING_STABILIZER_ROUTES.find((item) => item.id === routeId)!;
  const coordinate = selectedCoordinate;
  const maxInputError = Math.max(...route.inputGradientAbsoluteError);

  return (
    <main className="workspace workspace--stabilizers">
      <section className="stabilizer-stage" aria-label="Normalization dropout and residual comparison">
        <div className="stabilizer-intro">
          <div>
            <p className="eyebrow">NN25 / one branch, four routes</p>
            <h2>Normalization, dropout, and residual paths</h2>
            <p>Hold one learned branch fixed, then watch each training mechanism change its forward values and reverse gradient.</p>
          </div>
          <div className="stabilizer-chip">4 coordinates</div>
        </div>

        <section className="stabilizer-common-panel" aria-label="Shared stabilizer branch">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Shared setup</p>
              <h2>Everything starts from the same branch</h2>
            </div>
            <span>score = upstream · output</span>
          </div>
          <div className="stabilizer-common-flow">
            <VectorStrip label="input x" values={trace.input} selectedCoordinate={coordinate} />
            <div className="stabilizer-flow-arrow">× {formatNumber(trace.branchWeight)}</div>
            <VectorStrip label="learned branch h" values={trace.branch} selectedCoordinate={coordinate} tone="purple" />
            <VectorStrip label="upstream dS/doutput" values={trace.upstreamGradient} selectedCoordinate={coordinate} tone="red" />
          </div>
        </section>

        <section className="stabilizer-comparison-panel" aria-label="Training stabilizer route comparison">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Same numbers, different jobs</p>
              <h2>Compare all four routes</h2>
            </div>
            <span>select a route to unpack it</span>
          </div>
          <div className="stabilizer-comparison-grid">
            {trace.routes.map((item) => {
              const itemDefinition = TRAINING_STABILIZER_ROUTES.find((candidate) => candidate.id === item.id)!;
              return (
                <button
                  aria-pressed={item.id === routeId}
                  key={item.id}
                  type="button"
                  onClick={() => setRouteId(item.id)}
                >
                  <strong>{itemDefinition.label}</strong>
                  <span>{itemDefinition.summary}</span>
                  <code>output {formatVector(item.output)}</code>
                  <code>dS/dx {formatVector(item.inputGradient)}</code>
                  <small>score {formatNumber(item.score)}</small>
                </button>
              );
            })}
          </div>
        </section>

        <section className="stabilizer-forward-panel" aria-label="Selected stabilizer forward trace">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Forward / {definition.label}</p>
              <h2>What changes on this route?</h2>
            </div>
            <strong>score {formatNumber(route.score)}</strong>
          </div>
          {routeId === "normalization" ? (
            <div className="stabilizer-mechanism-trace">
              <div className="stabilizer-stat-grid">
                <div><small>mean</small><strong>{formatNumber(trace.normalization.mean)}</strong></div>
                <div><small>variance / population</small><strong>{formatNumber(trace.normalization.variance)}</strong></div>
                <div><small>standard deviation</small><strong>{formatNumber(trace.normalization.standardDeviation)}</strong></div>
                <div><small>epsilon / hand fixture</small><strong>0</strong></div>
              </div>
              <VectorStrip label="centered h - mean" values={trace.normalization.centered} selectedCoordinate={coordinate} tone="purple" />
              <VectorStrip label="normalized output" values={route.output} selectedCoordinate={coordinate} tone="green" />
              <code className="stabilizer-formula">normalized[i] = (h[i] - mean) / standard deviation</code>
            </div>
          ) : routeId === "dropout" ? (
            <div className="stabilizer-mechanism-trace">
              <VectorStrip label="binary mask" values={trace.dropoutMask} selectedCoordinate={coordinate} tone="red" />
              <VectorStrip label="mask / keep probability" values={trace.dropout.scaledMask} selectedCoordinate={coordinate} tone="purple" />
              <VectorStrip label="training output" values={route.output} selectedCoordinate={coordinate} tone="green" />
              <div className="stabilizer-dropout-compare">
                <div><small>evaluation / dropout off</small><code>{formatVector(trace.dropout.evaluationOutput)}</code></div>
                <div><small>expectation over training masks</small><code>{formatVector(trace.dropout.trainingExpectation)}</code></div>
              </div>
              <code className="stabilizer-formula">training output[i] = h[i] × mask[i] / {formatNumber(trace.keepProbability)}</code>
            </div>
          ) : routeId === "residual" ? (
            <div className="stabilizer-mechanism-trace">
              <VectorStrip label="identity skip x" values={trace.input} selectedCoordinate={coordinate} />
              <div className="stabilizer-plus">+</div>
              <VectorStrip label="learned branch h" values={trace.branch} selectedCoordinate={coordinate} tone="purple" />
              <div className="stabilizer-plus">=</div>
              <VectorStrip label="residual output" values={route.output} selectedCoordinate={coordinate} tone="green" />
              <code className="stabilizer-formula">output[i] = input[i] + branch[i]</code>
            </div>
          ) : (
            <div className="stabilizer-mechanism-trace">
              <VectorStrip label="plain output = h" values={route.output} selectedCoordinate={coordinate} tone="green" />
              <code className="stabilizer-formula">No extra route: output[i] = {formatNumber(trace.branchWeight)} × input[i]</code>
            </div>
          )}
        </section>

        <section className="stabilizer-backward-panel" aria-label="Selected stabilizer backward trace">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Backward / vector-Jacobian product</p>
              <h2>Where does the score gradient travel?</h2>
            </div>
            <span>dS/dweight {formatNumber(route.weightGradient)}</span>
          </div>
          <div className="stabilizer-gradient-flow">
            <VectorStrip label="upstream" values={trace.upstreamGradient} selectedCoordinate={coordinate} tone="red" />
            <VectorStrip label="into learned branch" values={route.branchGradient} selectedCoordinate={coordinate} tone="purple" />
            {routeId === "residual" ? (
              <VectorStrip label="through identity skip" values={route.skipGradient} selectedCoordinate={coordinate} />
            ) : null}
            <VectorStrip label="total dS/dinput" values={route.inputGradient} selectedCoordinate={coordinate} tone="green" />
          </div>
        </section>

        <section className="stabilizer-arithmetic-panel" aria-label="Selected stabilizer coordinate calculation">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Open coordinate {coordinate + 1}</p>
              <h2>One reverse calculation</h2>
            </div>
            <span>input {formatNumber(trace.input[coordinate]!)}</span>
          </div>
          <div className="stabilizer-equations">
            {routeId === "normalization" ? (
              <>
                <code>(4 × {formatNumber(trace.upstreamGradient[coordinate]!)} - {formatNumber(trace.normalization.upstreamSum)} - {formatNumber(trace.normalization.normalized[coordinate]!)} × {formatNumber(trace.normalization.upstreamDotNormalized)}) / (4 × {formatNumber(trace.normalization.standardDeviation)}) = {formatNumber(route.branchGradient[coordinate]!)}</code>
                <span>layer norm couples this coordinate to both vector-wide sums</span>
              </>
            ) : routeId === "dropout" ? (
              <>
                <code>{formatNumber(trace.upstreamGradient[coordinate]!)} × {formatNumber(trace.dropoutMask[coordinate]!)} / {formatNumber(trace.keepProbability)} = {formatNumber(route.branchGradient[coordinate]!)}</code>
                <span>a dropped coordinate receives zero branch gradient</span>
              </>
            ) : (
              <>
                <code>dS/dh[{coordinate + 1}] = {formatNumber(route.branchGradient[coordinate]!)}</code>
                <span>{routeId === "residual" ? "the branch and skip both receive the upstream gradient" : "the plain branch passes the upstream gradient unchanged"}</span>
              </>
            )}
            <code>{formatNumber(trace.branchWeight)} × {formatNumber(route.branchGradient[coordinate]!)} + {formatNumber(route.skipGradient[coordinate]!)} = {formatNumber(route.inputGradient[coordinate]!)}</code>
            <span>branch contribution + identity-skip contribution</span>
            <code>Σ dS/dh[i] × input[i] = {formatNumber(route.weightGradient)}</code>
            <span>the shared scalar branch-weight gradient</span>
          </div>
        </section>

        <section className="stabilizer-audit-panel" aria-label="Training stabilizer finite difference audit">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Independent numerical audit</p>
              <h2>Analytical gradients match score slopes</h2>
            </div>
            <span>epsilon 1e-6</span>
          </div>
          <div className="stabilizer-audit-grid">
            <div><small>selected analytical dS/dx</small><code>{formatNumber(route.inputGradient[coordinate]!)}</code></div>
            <div><small>selected finite difference</small><code>{formatNumber(route.finiteDifferenceInputGradient[coordinate]!)}</code></div>
            <div><small>maximum input error</small><code>{formatNumber(maxInputError)}</code></div>
            <div><small>analytical dS/dweight</small><code>{formatNumber(route.weightGradient)}</code></div>
            <div><small>weight finite difference</small><code>{formatNumber(route.finiteDifferenceWeightGradient)}</code></div>
            <div><small>weight error</small><code>{formatNumber(route.weightGradientAbsoluteError)}</code></div>
          </div>
        </section>
      </section>

      <aside className="controls stabilizer-controls" aria-label="Training stabilizer controls">
        <p className="eyebrow">Training mechanism</p>
        <h2>Choose a route</h2>
        <p>The learned branch, input, and upstream vector stay fixed.</p>
        <div className="stabilizer-route-buttons">
          {TRAINING_STABILIZER_ROUTES.map((item) => (
            <button
              aria-pressed={item.id === routeId}
              key={item.id}
              type="button"
              onClick={() => setRouteId(item.id)}
            >
              <strong>{item.label}</strong>
              <span>{item.summary}</span>
            </button>
          ))}
        </div>
        <p className="eyebrow">Coordinate microscope</p>
        <div className="stabilizer-coordinate-buttons">
          {trace.input.map((value, index) => (
            <button
              aria-label={`Open stabilizer coordinate ${index + 1}`}
              aria-pressed={coordinate === index}
              key={index}
              type="button"
              onClick={() => setSelectedCoordinate(index)}
            >
              <span>{index + 1}</span>
              <code>x = {formatNumber(value)}</code>
            </button>
          ))}
        </div>
        <div className="stabilizer-reading">
          <p className="eyebrow">Different jobs</p>
          <h2>{routeId === "normalization" ? "Coordinates share context" : routeId === "dropout" ? "Training samples a subnetwork" : routeId === "residual" ? "The skip keeps a short route" : "The control exposes the branch"}</h2>
          <p>These mechanisms can coexist, but they are not interchangeable fixes for depth.</p>
        </div>
      </aside>
    </main>
  );
}
