import { useMemo, useState } from "react";
import {
  attentionSoftmaxRow,
  traceAttentionSoftmax,
} from "./attention-softmax-lab.js";
import type { AttentionTokenId } from "./attention-qkv-lab.js";

interface AttentionWeightsWorkbenchProps {
  onShowScores: () => void;
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

function formatNullableVector(values: ReadonlyArray<number | null>): string {
  return `[${values.map((value) => value === null ? "blocked" : formatNumber(value)).join(", ")}]`;
}

export function AttentionWeightsWorkbench({ onShowScores }: AttentionWeightsWorkbenchProps) {
  const [causal, setCausal] = useState(true);
  const [queryId, setQueryId] = useState<AttentionTokenId>("blue");
  const trace = useMemo(() => traceAttentionSoftmax(causal), [causal]);
  const selected = attentionSoftmaxRow(trace, queryId);

  return (
    <main className="workspace workspace--attention-softmax">
      <section className="attention-softmax-stage" aria-label="Causal attention weight trace">
        <div className="attention-softmax-intro">
          <div>
            <p className="eyebrow">NN13 · normalize without looking ahead</p>
            <h2>Causal-softmax mixer</h2>
            <p>
              Mask future keys, normalize one query row into weights, then
              follow each weight into the value vector it scales.
            </p>
          </div>
          <div className="attention-softmax-chip">{causal ? "causal decoder" : "full context"}</div>
        </div>

        <section className="attention-weight-panel" aria-label="Attention weight matrix">
          <div className="attention-softmax-heading">
            <div>
              <p className="eyebrow">Rows normalize independently</p>
              <h2>{causal ? "Causal" : "Unmasked"} attention weights</h2>
            </div>
            <code>each row sums to 1</code>
          </div>

          <div
            className="attention-weight-grid"
            role="grid"
            aria-label={`${causal ? "Causal" : "Unmasked"} attention weight matrix`}
          >
            <span className="attention-grid-corner">q \ k</span>
            {trace.tokenIds.map((tokenId) => (
              <span className="attention-grid-label" key={`weight-key-${tokenId}`}>{tokenId} k</span>
            ))}
            {trace.rows.flatMap((row) => [
              <button
                aria-label={`Select ${row.queryId} query row`}
                aria-pressed={queryId === row.queryId}
                className={queryId === row.queryId
                  ? "attention-weight-row-button attention-weight-row-button--active"
                  : "attention-weight-row-button"}
                key={`weight-query-${row.queryId}`}
                type="button"
                onClick={() => setQueryId(row.queryId)}
              >
                {row.queryId} q
              </button>,
              ...row.weights.map((weight, keyIndex) => {
                const blocked = !row.allowed[keyIndex];
                return (
                  <div
                    aria-label={`${row.queryId} query to ${trace.tokenIds[keyIndex]} key: ${blocked ? "blocked" : formatNumber(weight)}`}
                    className={blocked
                      ? "attention-weight-cell attention-weight-cell--blocked"
                      : queryId === row.queryId
                        ? "attention-weight-cell attention-weight-cell--selected-row"
                        : "attention-weight-cell"}
                    key={`${row.queryId}-${trace.tokenIds[keyIndex]}`}
                    role="gridcell"
                  >
                    <strong>{blocked ? "blocked" : formatNumber(weight)}</strong>
                    <span aria-hidden="true" style={{ width: `${Math.max(weight * 100, 0)}%` }} />
                  </div>
                );
              }),
            ])}
          </div>
        </section>

        <section className="attention-normalize-panel" aria-label="Selected softmax row trace">
          <div className="attention-softmax-heading">
            <div>
              <p className="eyebrow">Selected · {selected.queryId} query</p>
              <h2>Score → mask → stable exponentials → weights</h2>
            </div>
            <code>max = {formatNumber(selected.rowMax)}</code>
          </div>

          <div className="attention-normalize-flow">
            <div>
              <small>scaled scores</small>
              <code>{formatVector(selected.scaledScores)}</code>
            </div>
            <span aria-hidden="true">→</span>
            <div>
              <small>after mask</small>
              <code>{formatNullableVector(selected.maskedScores)}</code>
            </div>
            <span aria-hidden="true">→</span>
            <div>
              <small>subtract max, exp</small>
              <code>{formatVector(selected.exponentials)}</code>
            </div>
            <span aria-hidden="true">→</span>
            <div className="attention-normalize-flow__result">
              <small>divide by {formatNumber(selected.denominator)}</small>
              <code>{formatVector(selected.weights)}</code>
            </div>
          </div>
        </section>

        <section className="attention-value-mix-panel" aria-label="Selected weighted value mix">
          <div className="attention-softmax-heading">
            <div>
              <p className="eyebrow">Weights finally meet values</p>
              <h2>Build the {selected.queryId} context</h2>
            </div>
            <div className="attention-context-result">
              <small>context</small>
              <strong>{formatVector(selected.context)}</strong>
            </div>
          </div>

          <div className="attention-value-lanes">
            {trace.tokenIds.map((tokenId, keyIndex) => (
              <div
                className={selected.allowed[keyIndex]
                  ? "attention-value-lane"
                  : "attention-value-lane attention-value-lane--blocked"}
                key={tokenId}
              >
                <span><i className={`attention-token-dot attention-token-dot--${tokenId}`} />{tokenId} value</span>
                <code>
                  {formatNumber(selected.weights[keyIndex]!)} × {formatVector(selected.values[keyIndex]!)}
                </code>
                <strong>= {formatVector(selected.valueContributions[keyIndex]!)}</strong>
              </div>
            ))}
          </div>
        </section>
      </section>

      <aside className="attention-softmax-controls" aria-label="Causal attention controls">
        <p className="eyebrow">One information boundary</p>
        <h2>Mask controls</h2>
        <p>
          Select a whole query row. Softmax belongs to that row, not to one
          score cell.
        </p>

        <button className="attention-back-button" type="button" onClick={onShowScores}>
          Return to Q/K/V scores
        </button>

        <label className="attention-scale-control">
          <input
            type="checkbox"
            checked={causal}
            onChange={(event) => setCausal(event.target.checked)}
          />
          <span>
            <strong>Block future keys</strong>
            <small>Allow column j only when j ≤ query row i.</small>
          </span>
        </label>

        <div className="attention-query-buttons" aria-label="Query row selection">
          {trace.tokenIds.map((tokenId) => (
            <button
              aria-pressed={queryId === tokenId}
              key={tokenId}
              type="button"
              onClick={() => setQueryId(tokenId)}
            >
              {tokenId}
            </button>
          ))}
        </div>

        <div className="attention-selected-summary">
          <small>selected context</small>
          <strong>{formatVector(selected.context)}</strong>
          <span>{selected.queryId} reads {selected.allowed.filter(Boolean).length} value{selected.allowed.filter(Boolean).length === 1 ? "" : "s"}.</span>
        </div>

        <div className="attention-value-boundary">
          <span>Why subtract the maximum?</span>
          <p>
            It keeps exponentials finite without changing their normalized
            proportions. The maximum shifted score is always zero.
          </p>
        </div>

        <div className="attention-next-note">
          <span>What scales next?</span>
          <p>
            Multiple heads repeat this calculation with different projections,
            then concatenate their context vectors.
          </p>
        </div>
      </aside>
    </main>
  );
}
