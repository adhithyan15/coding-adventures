import { useMemo, useState } from "react";
import {
  DEFAULT_DECODER_BIAS,
  DEFAULT_DECODER_UNEMBEDDING,
  decoderTrainingRow,
  traceTinyDecoderTraining,
} from "./decoder-language-model-lab.js";

interface DecoderTrainingWorkbenchProps {
  onShowMultiHead: () => void;
}

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  return Number(value.toFixed(6)).toString();
}

function formatVector(values: readonly number[]): string {
  return `[${values.map(formatNumber).join(", ")}]`;
}

function formatMatrix(values: readonly (readonly number[])[]): string {
  return values.map(formatVector).join("  ");
}

export function DecoderTrainingWorkbench({
  onShowMultiHead,
}: DecoderTrainingWorkbenchProps) {
  const trace = useMemo(() => traceTinyDecoderTraining(), []);
  const [position, setPosition] = useState(1);
  const [showUpdated, setShowUpdated] = useState(false);
  const selected = decoderTrainingRow(trace, position);
  const current = showUpdated ? trace.postUpdateRows[position]! : selected;

  return (
    <main className="workspace workspace--decoder">
      <section className="decoder-stage" aria-label="Tiny decoder language model training trace">
        <div className="decoder-intro">
          <div>
            <p className="eyebrow">NN15 - a complete next-token learning step</p>
            <h2>Tiny decoder training trace</h2>
            <p>
              Shift one sequence into two causal predictions, turn saved decoder
              states into vocabulary probabilities, then follow the shared error
              through cross-entropy and one loss-reducing SGD update.
            </p>
          </div>
          <div className="decoder-chip">3-token vocabulary - 2 positions</div>
        </div>

        <section className="decoder-shift-panel" aria-label="Causal next-token sequence shift">
          <div className="decoder-heading">
            <div>
              <p className="eyebrow">One sequence - shifted by one</p>
              <h2>Prefixes predict what comes next</h2>
            </div>
            <code>red blue purple</code>
          </div>
          <div className="decoder-position-lanes">
            {trace.rows.map((row) => (
              <button
                aria-label={`Select position ${row.position}: ${row.causalPrefix.join(" ")} predicts ${row.targetToken}`}
                aria-pressed={position === row.position}
                className="decoder-position-button"
                key={row.position}
                type="button"
                onClick={() => setPosition(row.position)}
              >
                <span>position {row.position}</span>
                <strong>{row.causalPrefix.join(" ")}</strong>
                <i aria-hidden="true">-&gt;</i>
                <strong>{row.targetToken}</strong>
                <small>future target stays outside the prefix</small>
              </button>
            ))}
          </div>
        </section>

        <section className="decoder-prediction-panel" aria-label={`Selected decoder prediction at position ${position}`}>
          <div className="decoder-heading">
            <div>
              <p className="eyebrow">Selected - position {position}</p>
              <h2>{showUpdated ? "Rerun the updated head" : "State to target surprise"}</h2>
            </div>
            <div className="decoder-loss-badge">
              <small>position loss</small>
              <strong>{formatNumber(current.loss)}</strong>
            </div>
          </div>

          <div className="decoder-forward-flow">
            <div className="decoder-state-node">
              <small>saved causal state</small>
              <strong>h_{selected.inputToken}</strong>
              <code>{formatVector(selected.decoderState)}</code>
            </div>
            <span aria-hidden="true">-&gt;</span>
            <div className="decoder-logit-node">
              <small>{showUpdated ? "updated logits" : "shared head logits"}</small>
              <code>{formatVector(current.logits)}</code>
            </div>
            <span aria-hidden="true">-&gt;</span>
            <div className="decoder-probability-node">
              <small>stable softmax</small>
              <code>{formatVector(current.probabilities)}</code>
            </div>
            <span aria-hidden="true">-&gt;</span>
            <div className="decoder-target-node">
              <small>target probability</small>
              <strong>P({selected.targetToken}) = {formatNumber(current.targetProbability)}</strong>
              <code>-ln(P) = {formatNumber(current.loss)}</code>
            </div>
          </div>

          <div className="decoder-vocabulary-grid" role="list" aria-label="Vocabulary probability distribution">
            {trace.vocabulary.map((token, vocabularyIndex) => {
              const probability = current.probabilities[vocabularyIndex]!;
              const isTarget = vocabularyIndex === selected.targetIndex;
              return (
                <div
                  className={isTarget ? "decoder-vocabulary-row decoder-vocabulary-row--target" : "decoder-vocabulary-row"}
                  key={token}
                  role="listitem"
                >
                  <div>
                    <span>{token}{isTarget ? " - target" : ""}</span>
                    <strong>{formatNumber(probability)}</strong>
                  </div>
                  <i aria-hidden="true" style={{ width: `${probability * 100}%` }} />
                  {!showUpdated ? (
                    <code>
                      {formatNumber(selected.logitProducts[vocabularyIndex]![0]!)} + {formatNumber(selected.logitProducts[vocabularyIndex]![1]!)} + bias = {formatNumber(selected.logits[vocabularyIndex]!)}
                    </code>
                  ) : null}
                </div>
              );
            })}
          </div>

          {!showUpdated ? (
            <div className="decoder-softmax-trace" aria-label="Stable softmax arithmetic">
              <div><small>row max</small><code>{formatNumber(selected.rowMax)}</code></div>
              <div><small>shift logits</small><code>{formatVector(selected.shiftedLogits)}</code></div>
              <div><small>exponentials</small><code>{formatVector(selected.exponentials)}</code></div>
              <div><small>denominator</small><code>{formatNumber(selected.denominator)}</code></div>
            </div>
          ) : null}
        </section>

        <section className="decoder-gradient-panel" aria-label="Decoder loss gradient trace">
          <div className="decoder-heading">
            <div>
              <p className="eyebrow">Probability minus target - divided by two</p>
              <h2>Error flows back through the shared head</h2>
            </div>
            <code>(p - one_hot) / positions</code>
          </div>
          <div className="decoder-gradient-flow">
            <div>
              <small>logit gradient</small>
              <code>{formatVector(selected.logitGradients)}</code>
            </div>
            <span aria-hidden="true">-&gt;</span>
            <div>
              <small>this position's unembedding contribution</small>
              <code>{formatMatrix(selected.unembeddingGradientContribution)}</code>
            </div>
            <span aria-hidden="true">+</span>
            <div>
              <small>bias contribution</small>
              <code>{formatVector(selected.biasGradientContribution)}</code>
            </div>
            <span aria-hidden="true">-&gt;</span>
            <div className="decoder-state-gradient">
              <small>gradient entering decoder body</small>
              <code>{formatVector(selected.stateGradient)}</code>
            </div>
          </div>
        </section>

        <section className="decoder-update-panel" aria-label="Shared decoder head SGD update">
          <div className="decoder-heading">
            <div>
              <p className="eyebrow">Both positions reduce into one update</p>
              <h2>Shared-head SGD checkpoint</h2>
            </div>
            <code>parameter - {trace.learningRate} x gradient</code>
          </div>
          <div className="decoder-update-grid">
            <div>
              <small>unembedding before</small>
              <code>{formatMatrix(DEFAULT_DECODER_UNEMBEDDING)}</code>
            </div>
            <div>
              <small>reduced gradient</small>
              <code>{formatMatrix(trace.unembeddingGradient)}</code>
            </div>
            <div>
              <small>unembedding after</small>
              <code>{formatMatrix(trace.updatedUnembedding)}</code>
            </div>
            <div>
              <small>bias before</small>
              <code>{formatVector(DEFAULT_DECODER_BIAS)}</code>
            </div>
            <div>
              <small>bias gradient</small>
              <code>{formatVector(trace.biasGradient)}</code>
            </div>
            <div>
              <small>bias after</small>
              <code>{formatVector(trace.updatedBias)}</code>
            </div>
          </div>
          <div className="decoder-gradient-audit">
            <span>Central finite-difference audit</span>
            <code>epsilon = {trace.gradientCheck.epsilon}</code>
            <strong>max error {trace.gradientCheck.maxAbsoluteError.toExponential(3)}</strong>
          </div>
          <div className="decoder-loss-drop">
            <div><small>mean loss before</small><strong>{formatNumber(trace.meanLoss)}</strong></div>
            <span aria-hidden="true">-&gt;</span>
            <div><small>mean loss after one step</small><strong>{formatNumber(trace.postUpdateMeanLoss)}</strong></div>
            <p>Both target probabilities rise; the deterministic objective falls.</p>
          </div>
        </section>
      </section>

      <aside className="decoder-controls" aria-label="Tiny decoder training controls">
        <p className="eyebrow">Inspect one prediction</p>
        <h2>Training controls</h2>
        <p>
          The causal prefixes and saved states do not change. The toggle swaps
          only the shared vocabulary head before and after its one SGD step.
        </p>

        <button className="attention-back-button" type="button" onClick={onShowMultiHead}>
          Return to multi-head block
        </button>

        <div className="attention-query-buttons" aria-label="Decoder position selection">
          {trace.rows.map((row) => (
            <button
              aria-pressed={position === row.position}
              key={row.position}
              type="button"
              onClick={() => setPosition(row.position)}
            >
              position {row.position}
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
            <strong>Use updated vocabulary head</strong>
            <small>Rerun logits and loss after one SGD step.</small>
          </span>
        </label>

        <div className="attention-selected-summary">
          <small>selected target</small>
          <strong>{selected.targetToken}</strong>
          <span>{selected.causalPrefix.join(" ")} -&gt; {selected.targetToken}</span>
        </div>

        <div className="attention-value-boundary">
          <span>Frozen on purpose</span>
          <p>
            This first trace updates unembedding and bias. The state gradient is
            preserved for a later full-decoder autograd pass.
          </p>
        </div>

        <div className="attention-next-note">
          <span>What scales next?</span>
          <p>
            Add token sampling and a generation trace, then continue the saved
            state gradients through every decoder-block parameter.
          </p>
        </div>
      </aside>
    </main>
  );
}
