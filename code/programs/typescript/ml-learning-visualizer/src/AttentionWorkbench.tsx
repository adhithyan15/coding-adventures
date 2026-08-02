import { useMemo, useState } from "react";
import {
  attentionCell,
  traceAttentionQkv,
  type AttentionTokenId,
} from "./attention-qkv-lab.js";
import { AttentionWeightsWorkbench } from "./AttentionWeightsWorkbench.js";
import { DecoderTrainingWorkbench } from "./DecoderTrainingWorkbench.js";
import { MultiHeadAttentionWorkbench } from "./MultiHeadAttentionWorkbench.js";

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  return Number(value.toFixed(6)).toString();
}

function formatVector(values: readonly number[]): string {
  return `[${values.map(formatNumber).join(", ")}]`;
}

interface AttentionScoreWorkbenchProps {
  onShowWeights: () => void;
}

function AttentionScoreWorkbench({ onShowWeights }: AttentionScoreWorkbenchProps) {
  const trace = useMemo(() => traceAttentionQkv(), []);
  const [queryId, setQueryId] = useState<AttentionTokenId>("blue");
  const [keyId, setKeyId] = useState<AttentionTokenId>("purple");
  const [scaled, setScaled] = useState(false);
  const selected = attentionCell(trace, queryId, keyId);
  const query = trace.projections.find((item) => item.id === queryId)!;
  const key = trace.projections.find((item) => item.id === keyId)!;
  const scoreMatrix = scaled ? trace.scaledScoreMatrix : trace.rawScoreMatrix;

  return (
    <main className="workspace workspace--attention">
      <section className="attention-stage" aria-label="Three-token attention score trace">
        <div className="attention-intro">
          <div>
            <p className="eyebrow">NN12 · attention foundations</p>
            <h2>Query-key score microscope</h2>
            <p>
              Give every token three jobs, then open any score cell to see the
              two multiplications and one addition behind its match strength.
            </p>
          </div>
          <div className="attention-sequence-chip">red · blue · purple</div>
        </div>

        <section className="attention-projection-panel" aria-label="Token projections">
          <div className="attention-panel-heading">
            <div>
              <p className="eyebrow">One token · three projections</p>
              <h2>Ask, advertise, carry</h2>
            </div>
            <p>Each row uses the same three learned matrices.</p>
          </div>
          <div className="attention-projection-table">
            <div className="attention-projection-head" aria-hidden="true">
              <span>token x</span><span>query q</span><span>key k</span><span>value v</span>
            </div>
            {trace.projections.map((projection) => (
              <div className="attention-projection-row" key={projection.id}>
                <div>
                  <i className={`attention-token-dot attention-token-dot--${projection.id}`} />
                  <strong>{projection.label}</strong>
                  <code>{formatVector(projection.embedding)}</code>
                </div>
                <div><small>asks with</small><code>{formatVector(projection.query)}</code></div>
                <div><small>matches with</small><code>{formatVector(projection.key)}</code></div>
                <div><small>carries</small><code>{formatVector(projection.value)}</code></div>
              </div>
            ))}
          </div>
        </section>

        <section className="attention-score-panel" aria-label="Query-key score matrix">
          <div className="attention-panel-heading">
            <div>
              <p className="eyebrow">Rows ask · columns match</p>
              <h2>{scaled ? "Scaled" : "Raw"} query-key scores</h2>
            </div>
            <code>{scaled ? "QK^T / sqrt(2)" : "QK^T"}</code>
          </div>

          <div className="attention-score-layout">
            <div className="attention-score-grid" role="grid" aria-label={`${scaled ? "Scaled" : "Raw"} attention scores`}>
              <span className="attention-grid-corner">q \ k</span>
              {trace.projections.map((projection) => (
                <span className="attention-grid-label" key={`key-${projection.id}`}>{projection.label} k</span>
              ))}
              {trace.projections.flatMap((queryProjection, row) => [
                <span className="attention-grid-label" key={`query-${queryProjection.id}`}>{queryProjection.label} q</span>,
                ...trace.projections.map((keyProjection, column) => {
                  const active = queryId === queryProjection.id && keyId === keyProjection.id;
                  return (
                    <button
                      aria-label={`Select ${queryProjection.label} query and ${keyProjection.label} key`}
                      aria-selected={active}
                      className={active ? "attention-score-cell attention-score-cell--active" : "attention-score-cell"}
                      key={`${queryProjection.id}-${keyProjection.id}`}
                      role="gridcell"
                      type="button"
                      onClick={() => {
                        setQueryId(queryProjection.id);
                        setKeyId(keyProjection.id);
                      }}
                    >
                      {formatNumber(scoreMatrix[row]![column]!)}
                    </button>
                  );
                }),
              ])}
            </div>

            <div className="attention-cell-trace" aria-label="Selected dot-product arithmetic">
              <div>
                <p className="eyebrow">Selected cell</p>
                <h3>{query.label} asks · {key.label} matches</h3>
              </div>
              <div className="attention-vector-pair">
                <span><small>query</small><code>q_{query.id} = {formatVector(query.query)}</code></span>
                <span><small>key</small><code>k_{key.id} = {formatVector(key.key)}</code></span>
              </div>
              <div className="attention-dot-equation">
                <code>{`${formatNumber(query.query[0]!)} × ${formatNumber(key.key[0]!)} + ${formatNumber(query.query[1]!)} × ${formatNumber(key.key[1]!)}`}</code>
                <strong>= {formatNumber(selected.rawScore)}</strong>
              </div>
              <div className="attention-products">
                coordinate products {formatVector(selected.products)}
              </div>
              {scaled ? (
                <div className="attention-scale-equation">
                  {formatNumber(selected.rawScore)} / sqrt(2) = <strong>{formatNumber(selected.scaledScore)}</strong>
                </div>
              ) : null}
            </div>
          </div>
        </section>
      </section>

      <aside className="attention-controls" aria-label="Attention score controls">
        <p className="eyebrow">Keep the boundary honest</p>
        <h2>Score controls</h2>
        <p>
          A score says how strongly one query matches one key. It does not yet
          say how much of a value to blend.
        </p>

        <button className="attention-back-button" type="button" onClick={onShowWeights}>
          Apply softmax and causal mask
        </button>

        <label className="attention-scale-control">
          <input
            type="checkbox"
            checked={scaled}
            onChange={(event) => setScaled(event.target.checked)}
          />
          <span>
            <strong>Scale by sqrt(key dimension)</strong>
            <small>Divide every raw score by sqrt(2).</small>
          </span>
        </label>

        <div className="attention-selected-summary">
          <small>selected score</small>
          <strong>{formatNumber(scaled ? selected.scaledScore : selected.rawScore)}</strong>
          <span>{query.label} query → {key.label} key</span>
        </div>

        <div className="attention-value-boundary">
          <span>Value waiting downstream</span>
          <code>v_{key.id} = {formatVector(key.value)}</code>
          <p>This payload does not enter the score calculation.</p>
        </div>

        <div className="attention-next-note">
          <span>What comes next?</span>
          <p>
            Open the next view to turn each score row into weights and use
            those weights to blend the value vectors.
          </p>
        </div>
      </aside>
    </main>
  );
}

export function AttentionWorkbench() {
  const [view, setView] = useState<"scores" | "weights" | "multi-head" | "decoder">("scores");

  if (view === "weights") {
    return (
      <AttentionWeightsWorkbench
        onShowMultiHead={() => setView("multi-head")}
        onShowScores={() => setView("scores")}
      />
    );
  }
  if (view === "multi-head") {
    return (
      <MultiHeadAttentionWorkbench
        onShowDecoder={() => setView("decoder")}
        onShowWeights={() => setView("weights")}
      />
    );
  }
  if (view === "decoder") {
    return <DecoderTrainingWorkbench onShowMultiHead={() => setView("multi-head")} />;
  }
  return <AttentionScoreWorkbench onShowWeights={() => setView("weights")} />;
}
