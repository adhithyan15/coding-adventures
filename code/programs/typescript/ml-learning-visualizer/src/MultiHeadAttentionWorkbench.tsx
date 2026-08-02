import { useMemo, useState } from "react";
import type { AttentionTokenId } from "./attention-qkv-lab.js";
import {
  multiHeadAttentionRow,
  traceMultiHeadAttention,
} from "./multi-head-attention-lab.js";

interface MultiHeadAttentionWorkbenchProps {
  onShowDecoder: () => void;
  onShowWeights: () => void;
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

function headLabel(id: string): string {
  return id === "horizontal" ? "Head A - horizontal" : "Head B - vertical";
}

export function MultiHeadAttentionWorkbench({
  onShowDecoder,
  onShowWeights,
}: MultiHeadAttentionWorkbenchProps) {
  const [tokenId, setTokenId] = useState<AttentionTokenId>("blue");
  const [includeResidual, setIncludeResidual] = useState(true);
  const [applyLayerNorm, setApplyLayerNorm] = useState(true);
  const trace = useMemo(
    () => traceMultiHeadAttention(includeResidual, applyLayerNorm),
    [applyLayerNorm, includeResidual],
  );
  const selected = multiHeadAttentionRow(trace, tokenId);

  return (
    <main className="workspace workspace--multi-head">
      <section className="multi-head-stage" aria-label="Multi-head attention block trace">
        <div className="multi-head-intro">
          <div>
            <p className="eyebrow">NN14 - parallel views rejoin one stream</p>
            <h2>Multi-head add-and-norm block</h2>
            <p>
              Run two causal heads on the same token, keep their weights
              separate, then follow concatenation, projection, residual, and
              layer normalization without skipping a boundary.
            </p>
          </div>
          <div className="multi-head-chip">2 heads x 1 feature</div>
        </div>

        <section className="multi-head-panel" aria-label={`Two attention heads for ${tokenId}`}>
          <div className="multi-head-heading">
            <div>
              <p className="eyebrow">Selected - {tokenId} query</p>
              <h2>Same token, different learned views</h2>
            </div>
            <code>each head softmaxes alone</code>
          </div>

          <div className="multi-head-lanes">
            {selected.heads.map((head) => (
              <article className={`multi-head-lane multi-head-lane--${head.id}`} key={head.id}>
                <div className="multi-head-lane__heading">
                  <div>
                    <small>{headLabel(head.id)}</small>
                    <strong>q = {formatNumber(head.query)}</strong>
                  </div>
                  <code>context {formatNumber(head.context)}</code>
                </div>

                <div className="multi-head-score-row">
                  <span>scores</span>
                  <code>{formatVector(head.scaledScores)}</code>
                </div>

                <div className="multi-head-weight-row" role="list" aria-label={`${head.id} weights`}>
                  {trace.tokenIds.map((keyId, keyIndex) => (
                    <div
                      className={head.allowed[keyIndex]
                        ? "multi-head-weight"
                        : "multi-head-weight multi-head-weight--blocked"}
                      key={keyId}
                      role="listitem"
                    >
                      <span>{keyId}</span>
                      <strong>{head.allowed[keyIndex] ? formatNumber(head.weights[keyIndex]!) : "blocked"}</strong>
                      <i aria-hidden="true" style={{ width: `${head.weights[keyIndex]! * 100}%` }} />
                    </div>
                  ))}
                </div>

                <div className="multi-head-value-row">
                  {trace.tokenIds.map((keyId, keyIndex) => (
                    <code key={keyId}>
                      {formatNumber(head.weights[keyIndex]!)} x {formatNumber(head.values[keyIndex]!)} = {formatNumber(head.valueContributions[keyIndex]!)}
                    </code>
                  ))}
                </div>
              </article>
            ))}
          </div>
        </section>

        <section className="multi-head-join-panel" aria-label="Concatenate project and add residual trace">
          <div className="multi-head-heading">
            <div>
              <p className="eyebrow">Heads rejoin before the shortcut</p>
              <h2>Concatenate - project - add</h2>
            </div>
            <code>model width = 2</code>
          </div>

          <div className="multi-head-join-flow">
            <div>
              <small>head contexts</small>
              <code>{formatVector(selected.heads.map((head) => head.context))}</code>
            </div>
            <span aria-hidden="true">-&gt;</span>
            <div>
              <small>concatenate</small>
              <code>{formatVector(selected.concatenated)}</code>
            </div>
            <span aria-hidden="true">-&gt;</span>
            <div>
              <small>identity W_o</small>
              <code>{formatVector(selected.projectedAttention)}</code>
            </div>
            <span aria-hidden="true">+</span>
            <div className={includeResidual ? "multi-head-residual" : "multi-head-residual multi-head-residual--off"}>
              <small>{includeResidual ? `${tokenId} residual` : "residual removed"}</small>
              <code>{formatVector(includeResidual ? selected.input : [0, 0])}</code>
            </div>
            <span aria-hidden="true">=</span>
            <div className="multi-head-join-result">
              <small>add result</small>
              <code>{formatVector(selected.residualSum)}</code>
            </div>
          </div>
        </section>

        <section className="multi-head-norm-panel" aria-label="Layer normalization arithmetic">
          <div className="multi-head-heading">
            <div>
              <p className="eyebrow">One token - normalize across features</p>
              <h2>{applyLayerNorm ? "Layer normalization" : "Layer normalization bypassed"}</h2>
            </div>
            <div className="multi-head-output">
              <small>block output</small>
              <strong>{formatVector(selected.output)}</strong>
            </div>
          </div>

          <div className={applyLayerNorm ? "multi-head-norm-flow" : "multi-head-norm-flow multi-head-norm-flow--off"}>
            <div>
              <small>mean</small>
              <code>{formatNumber(selected.layerNorm.mean)}</code>
            </div>
            <div>
              <small>centered</small>
              <code>{formatVector(selected.layerNorm.centered)}</code>
            </div>
            <div>
              <small>squared deviations</small>
              <code>{formatVector(selected.layerNorm.squaredDeviations)}</code>
            </div>
            <div>
              <small>variance</small>
              <code>{formatNumber(selected.layerNorm.variance)}</code>
            </div>
            <div>
              <small>sqrt(var + 0.00001)</small>
              <code>{formatNumber(selected.layerNorm.denominator)}</code>
            </div>
            <div className="multi-head-norm-result">
              <small>gamma x normalized + beta</small>
              <code>{formatVector(selected.layerNorm.output)}</code>
            </div>
          </div>
        </section>
      </section>

      <aside className="multi-head-controls" aria-label="Multi-head attention controls">
        <p className="eyebrow">Inspect one token row</p>
        <h2>Block controls</h2>
        <p>
          Both heads stay visible so their different scores and value mixes can
          be compared on the same causal boundary.
        </p>

        <button className="attention-back-button" type="button" onClick={onShowWeights}>
          Return to single-head weights
        </button>

        <button className="attention-back-button" type="button" onClick={onShowDecoder}>
          Open tiny decoder training
        </button>

        <div className="attention-query-buttons" aria-label="Multi-head token selection">
          {trace.tokenIds.map((id) => (
            <button
              aria-pressed={tokenId === id}
              key={id}
              type="button"
              onClick={() => setTokenId(id)}
            >
              {id}
            </button>
          ))}
        </div>

        <label className="attention-scale-control">
          <input
            type="checkbox"
            checked={includeResidual}
            onChange={(event) => setIncludeResidual(event.target.checked)}
          />
          <span>
            <strong>Add residual token</strong>
            <small>Keep the original embedding on a short route.</small>
          </span>
        </label>

        <label className="attention-scale-control">
          <input
            type="checkbox"
            checked={applyLayerNorm}
            onChange={(event) => setApplyLayerNorm(event.target.checked)}
          />
          <span>
            <strong>Apply layer normalization</strong>
            <small>Use population variance across this token's features.</small>
          </span>
        </label>

        <div className="attention-selected-summary">
          <small>selected block output</small>
          <strong>{formatVector(selected.output)}</strong>
          <span>{tokenId} after both head paths rejoin.</span>
        </div>

        <div className="attention-value-boundary">
          <span>Why keep the heads separate?</span>
          <p>
            A softmax row belongs to one head. Concatenation happens only after
            each head has produced its own context.
          </p>
        </div>

        <div className="attention-next-note">
          <span>What scales next?</span>
          <p>
            A decoder repeats this block across tokens and layers, then adds
            embeddings, a feed-forward path, logits, loss, and an optimizer.
          </p>
        </div>
      </aside>
    </main>
  );
}
