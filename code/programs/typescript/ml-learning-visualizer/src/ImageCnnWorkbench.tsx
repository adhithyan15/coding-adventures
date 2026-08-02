import { useMemo, useState } from "react";
import {
  DEFAULT_IMAGE_BETA,
  DEFAULT_IMAGE_CHANNELS,
  DEFAULT_IMAGE_EPSILON,
  DEFAULT_IMAGE_FILTERS,
  DEFAULT_IMAGE_GAMMA,
  traceTinyImageCnn,
  type NumberMatrix,
} from "./image-cnn-lab.js";

const PIPELINE_STAGES = [
  { id: "channels", label: "Channels" },
  { id: "convolve", label: "Convolve" },
  { id: "normalize", label: "Normalize" },
  { id: "relu", label: "ReLU" },
  { id: "pool", label: "Pool" },
] as const;

type PipelineStage = typeof PIPELINE_STAGES[number]["id"];

function formatNumber(value: number): string {
  if (Math.abs(value) < 1e-12) {
    return "0";
  }
  return Number(value.toFixed(4)).toString();
}

function MatrixGrid({
  values,
  label,
  selected,
  winner,
}: {
  values: NumberMatrix;
  label: string;
  selected?: readonly [number, number];
  winner?: readonly [number, number];
}) {
  return (
    <div className="image-matrix-block">
      <span>{label}</span>
      <div
        className="image-matrix"
        style={{ gridTemplateColumns: `repeat(${values[0]!.length}, minmax(44px, 1fr))` }}
        aria-label={label}
      >
        {values.flatMap((row, rowIndex) => row.map((value, columnIndex) => {
          const isSelected = selected?.[0] === rowIndex && selected[1] === columnIndex;
          const isWinner = winner?.[0] === rowIndex && winner[1] === columnIndex;
          const className = isWinner
            ? "image-matrix-cell image-matrix-cell--winner"
            : isSelected
              ? "image-matrix-cell image-matrix-cell--selected"
              : "image-matrix-cell";
          return (
            <div className={className} key={`${rowIndex}-${columnIndex}`}>
              <small>[{rowIndex},{columnIndex}]</small>
              <strong>{formatNumber(value)}</strong>
            </div>
          );
        }))}
      </div>
    </div>
  );
}

export function ImageCnnWorkbench() {
  const [stage, setStage] = useState<PipelineStage>("channels");
  const [selectedFilter, setSelectedFilter] = useState(0);
  const [selectedPosition, setSelectedPosition] = useState(3);
  const trace = useMemo(() => traceTinyImageCnn(), []);
  const stageIndex = PIPELINE_STAGES.findIndex((item) => item.id === stage);
  const row = Math.floor(selectedPosition / 2);
  const column = selectedPosition % 2;
  const filter = DEFAULT_IMAGE_FILTERS[selectedFilter]!;
  const position = trace.positions[selectedFilter]![row]![column]!;
  const selectedCoordinate = [row, column] as const;

  function moveStage(offset: number): void {
    const nextIndex = Math.min(
      Math.max(stageIndex + offset, 0),
      PIPELINE_STAGES.length - 1,
    );
    setStage(PIPELINE_STAGES[nextIndex]!.id);
  }

  function reset(): void {
    setStage("channels");
    setSelectedFilter(0);
    setSelectedPosition(3);
  }

  return (
    <main className="workspace workspace--image-cnn">
      <section className="image-cnn-stage" aria-label="Tiny image CNN trace">
        <div className="image-cnn-intro">
          <div>
            <p className="eyebrow">NN07 · tiny image CNN</p>
            <h2>Open the image pipeline</h2>
            <p>
              Follow two image channels through shared spatial windows, channel
              reduction, normalization, ReLU, and max pooling.
            </p>
          </div>
          <div className="image-shape-chip">2 × 3 × 3 → 2 × 2 × 2 → 2</div>
        </div>

        <nav className="image-pipeline" aria-label="Image CNN pipeline stages">
          {PIPELINE_STAGES.map((item, index) => (
            <button
              aria-label={`Show ${item.label} stage`}
              className={item.id === stage
                ? "image-stage-button image-stage-button--active"
                : index < stageIndex
                  ? "image-stage-button image-stage-button--visited"
                  : "image-stage-button"}
              key={item.id}
              type="button"
              onClick={() => setStage(item.id)}
            >
              <small>{index + 1}</small>
              <strong>{item.label}</strong>
            </button>
          ))}
        </nav>

        {stage === "channels" ? (
          <section className="image-stage-panel" aria-label="Input image channels">
            <div className="image-stage-heading">
              <div>
                <p className="eyebrow">Stage 1 · input tensor</p>
                <h2>One image can have several number grids</h2>
              </div>
              <code>shape [channels, rows, columns] = [2, 3, 3]</code>
            </div>
            <div className="image-channel-grid">
              {DEFAULT_IMAGE_CHANNELS.map((channel, channelIndex) => (
                <article className="image-channel-card" key={channel.name}>
                  <div>
                    <small>input channel {channelIndex}</small>
                    <strong>{channel.name}</strong>
                  </div>
                  <MatrixGrid values={channel.values} label={`${channel.name} values`} />
                </article>
              ))}
            </div>
            <p className="image-stage-note">
              A filter owns one kernel per input channel. Their spatial results
              meet only after each channel has produced its own partial sum.
            </p>
          </section>
        ) : null}

        {stage === "convolve" ? (
          <section className="image-stage-panel" aria-label="Channel accumulation trace">
            <div className="image-stage-heading">
              <div>
                <p className="eyebrow">
                  Stage 2 · filter {selectedFilter} · output [{row},{column}]
                </p>
                <h2>Correlate each channel, then add</h2>
              </div>
              <strong className="image-output-value">{formatNumber(position.output)}</strong>
            </div>

            <div className="channel-math-grid">
              {DEFAULT_IMAGE_CHANNELS.map((channel, channelIndex) => (
                <article className="channel-math-card" key={channel.name}>
                  <div className="channel-math-title">
                    <div>
                      <small>channel {channelIndex}</small>
                      <strong>{channel.name}</strong>
                    </div>
                    <strong>{formatNumber(position.channelSums[channelIndex]!)}</strong>
                  </div>
                  <div className="window-kernel-pair">
                    <MatrixGrid
                      values={position.windows[channelIndex]!}
                      label="selected window"
                    />
                    <span aria-hidden="true">×</span>
                    <MatrixGrid
                      values={filter.kernels[channelIndex]!}
                      label="channel kernel"
                    />
                  </div>
                  <div className="image-product-list">
                    {position.products[channelIndex]!.flatMap((productRow, kernelRow) => (
                      productRow.map((product, kernelColumn) => (
                        <code key={`${kernelRow}-${kernelColumn}`}>
                          {formatNumber(position.windows[channelIndex]![kernelRow]![kernelColumn]!)}
                          ×{formatNumber(filter.kernels[channelIndex]![kernelRow]![kernelColumn]!)}
                          ={formatNumber(product)}
                        </code>
                      ))
                    ))}
                  </div>
                </article>
              ))}
            </div>

            <div className="channel-reduction" aria-label="Channel reduction equation">
              <div>
                <small>channel 0</small>
                <strong>{formatNumber(position.channelSums[0]!)}</strong>
              </div>
              <span>+</span>
              <div>
                <small>channel 1</small>
                <strong>{formatNumber(position.channelSums[1]!)}</strong>
              </div>
              <span>+</span>
              <div>
                <small>bias</small>
                <strong>{formatNumber(filter.bias)}</strong>
              </div>
              <span>=</span>
              <div className="channel-reduction__result">
                <small>output</small>
                <strong>{formatNumber(position.output)}</strong>
              </div>
            </div>

            <div className="image-map-pair">
              {trace.convolution.map((featureMap, filterIndex) => (
                <MatrixGrid
                  key={filterIndex}
                  values={featureMap}
                  label={`filter ${filterIndex} convolution map`}
                  selected={filterIndex === selectedFilter ? selectedCoordinate : undefined}
                />
              ))}
            </div>
          </section>
        ) : null}

        {stage === "normalize" ? (
          <section className="image-stage-panel" aria-label="Spatial normalization trace">
            <div className="image-stage-heading">
              <div>
                <p className="eyebrow">Stage 3 · output channel {selectedFilter}</p>
                <h2>Four spatial values share statistics</h2>
              </div>
              <code>(x − μ) / √(variance + ε)</code>
            </div>
            <div className="normalization-flow">
              <MatrixGrid
                values={trace.convolution[selectedFilter]!}
                label="convolution map"
                selected={selectedCoordinate}
              />
              <div className="normalization-stats">
                <div><small>mean μ</small><strong>{formatNumber(trace.normalization.means[selectedFilter]!)}</strong></div>
                <div><small>variance</small><strong>{formatNumber(trace.normalization.variances[selectedFilter]!)}</strong></div>
                <div><small>epsilon ε</small><strong>{formatNumber(DEFAULT_IMAGE_EPSILON)}</strong></div>
                <div><small>denominator</small><strong>{formatNumber(trace.normalization.denominators[selectedFilter]!)}</strong></div>
              </div>
              <MatrixGrid
                values={trace.normalization.maps[selectedFilter]!}
                label="normalized map"
                selected={selectedCoordinate}
              />
            </div>
            <code className="normalization-equation">
              ({formatNumber(position.output)} − {formatNumber(trace.normalization.means[selectedFilter]!)})
              {' '}/ {formatNumber(trace.normalization.denominators[selectedFilter]!)}
              {' '}× γ {formatNumber(DEFAULT_IMAGE_GAMMA[selectedFilter]!)}
              {' '}+ β {formatNumber(DEFAULT_IMAGE_BETA[selectedFilter]!)}
              {' '}= {formatNumber(trace.normalization.maps[selectedFilter]![row]![column]!)}
            </code>
          </section>
        ) : null}

        {stage === "relu" ? (
          <section className="image-stage-panel" aria-label="ReLU activation trace">
            <div className="image-stage-heading">
              <div>
                <p className="eyebrow">Stage 4 · output channel {selectedFilter}</p>
                <h2>Keep positive evidence</h2>
              </div>
              <code>ReLU(x) = max(0, x)</code>
            </div>
            <div className="activation-flow">
              <MatrixGrid
                values={trace.normalization.maps[selectedFilter]!}
                label="normalized values"
                selected={selectedCoordinate}
              />
              <span aria-hidden="true">→</span>
              <MatrixGrid
                values={trace.activation[selectedFilter]!}
                label="after ReLU"
                selected={selectedCoordinate}
              />
            </div>
            <code className="normalization-equation">
              max(0, {formatNumber(trace.normalization.maps[selectedFilter]![row]![column]!)})
              {' '}= {formatNumber(trace.activation[selectedFilter]![row]![column]!)}
            </code>
          </section>
        ) : null}

        {stage === "pool" ? (
          <section className="image-stage-panel" aria-label="Max pooling trace">
            <div className="image-stage-heading">
              <div>
                <p className="eyebrow">Stage 5 · shrink the maps</p>
                <h2>Keep each channel's strongest location</h2>
              </div>
              <code>2 × 2 max pool · stride 2</code>
            </div>
            <div className="pooling-grid">
              {trace.activation.map((featureMap, filterIndex) => (
                <article className="pooling-card" key={filterIndex}>
                  <MatrixGrid
                    values={featureMap}
                    label={`filter ${filterIndex} activated map`}
                    winner={trace.pooling.argmax[filterIndex]!}
                  />
                  <span aria-hidden="true">→</span>
                  <div className="pooled-value">
                    <small>pooled[{filterIndex}]</small>
                    <strong>{formatNumber(trace.pooling.values[filterIndex]!)}</strong>
                    <code>
                      from [{trace.pooling.argmax[filterIndex]![0]},
                      {trace.pooling.argmax[filterIndex]![1]}]
                    </code>
                  </div>
                </article>
              ))}
            </div>
            <p className="image-stage-note">
              Only the highlighted winner receives gradient through max pooling.
              The other three values were useful for comparison, but are discarded.
            </p>
          </section>
        ) : null}
      </section>

      <aside className="image-cnn-controls" aria-label="Image CNN trace controls">
        <p className="eyebrow">Choose one path</p>
        <h2>Filter and output</h2>
        <p>Selections stay synchronized as you move through the pipeline.</p>

        <div className="image-control-group">
          <span>Output filter</span>
          <div className="image-filter-buttons">
            {DEFAULT_IMAGE_FILTERS.map((item, index) => (
              <button
                aria-label={`Select filter ${index} ${item.name}`}
                className={index === selectedFilter ? "image-choice image-choice--active" : "image-choice"}
                key={item.name}
                type="button"
                onClick={() => setSelectedFilter(index)}
              >
                <small>filter {index}</small>
                <strong>{item.name}</strong>
              </button>
            ))}
          </div>
        </div>

        <div className="image-control-group">
          <span>Spatial output</span>
          <div className="image-position-buttons">
            {[0, 1, 2, 3].map((index) => {
              const outputRow = Math.floor(index / 2);
              const outputColumn = index % 2;
              return (
                <button
                  aria-label={`Select image output row ${outputRow} column ${outputColumn}`}
                  className={index === selectedPosition ? "image-choice image-choice--active" : "image-choice"}
                  key={index}
                  type="button"
                  onClick={() => setSelectedPosition(index)}
                >
                  <small>[{outputRow},{outputColumn}]</small>
                  <strong>{formatNumber(trace.convolution[selectedFilter]![outputRow]![outputColumn]!)}</strong>
                </button>
              );
            })}
          </div>
        </div>

        <div className="button-grid image-stage-controls">
          <button type="button" disabled={stageIndex === 0} onClick={() => moveStage(-1)}>
            Previous stage
          </button>
          <button
            type="button"
            disabled={stageIndex === PIPELINE_STAGES.length - 1}
            onClick={() => moveStage(1)}
          >
            Next stage
          </button>
          <button type="button" onClick={reset}>Reset trace</button>
        </div>

        <div className="image-cnn-note">
          <span>What scales next?</span>
          <p>
            Larger CNNs repeat these same loops over batches, many channels,
            many filters, and deeper feature maps. Accelerators change the
            schedule, not the arithmetic contract.
          </p>
        </div>
      </aside>
    </main>
  );
}
