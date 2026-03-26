import { useState } from "react";
import {
  DEFAULT_RENDER_CONFIG,
  drawCode39,
  encodeCode39,
  expandCode39Runs,
  normalizeCode39,
  type BarcodeRun,
  type EncodedCharacter,
} from "@coding-adventures/code39";
import { renderSvg } from "@coding-adventures/draw-instructions-svg";

const DEFAULT_VALUE = "CODE39-123";
const SUPPORTED_CHARACTERS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%";

interface BarcodeModel {
  input: string;
  normalized: string;
  encodedCharacters: EncodedCharacter[];
  runs: BarcodeRun[];
  symbolLayouts: SymbolLayout[];
  svg: string;
  width: number;
  height: number;
}

interface SymbolLayout {
  char: string;
  start: number;
  width: number;
  center: number;
  isStartStop: boolean;
}

function runWidth(run: BarcodeRun): number {
  return run.width === "wide" ? DEFAULT_RENDER_CONFIG.wideUnit : DEFAULT_RENDER_CONFIG.narrowUnit;
}

function buildSymbolLayouts(
  encodedCharacters: EncodedCharacter[],
  runs: BarcodeRun[],
): SymbolLayout[] {
  const quietZone = DEFAULT_RENDER_CONFIG.quietZoneUnits * DEFAULT_RENDER_CONFIG.narrowUnit;
  const layouts: SymbolLayout[] = [];
  let cursorX = quietZone;

  encodedCharacters.forEach((encodedCharacter, sourceIndex) => {
    const symbolRuns = runs.filter((run) => run.sourceIndex === sourceIndex && !run.isInterCharacterGap);
    const width = symbolRuns.reduce((sum, run) => sum + runWidth(run), 0);

    layouts.push({
      char: encodedCharacter.char,
      start: cursorX,
      width,
      center: cursorX + width / 2,
      isStartStop: encodedCharacter.isStartStop,
    });

    cursorX += width;

    if (sourceIndex < encodedCharacters.length - 1) {
      cursorX += DEFAULT_RENDER_CONFIG.narrowUnit;
    }
  });

  return layouts;
}

function buildBarcodeModel(input: string): BarcodeModel {
  const normalized = normalizeCode39(input);
  const encodedCharacters = encodeCode39(normalized);
  const runs = expandCode39Runs(normalized);
  const symbolLayouts = buildSymbolLayouts(encodedCharacters, runs);
  const scene = drawCode39(normalized, {
    ...DEFAULT_RENDER_CONFIG,
    includeHumanReadableText: false,
  });

  return {
    input,
    normalized,
    encodedCharacters,
    runs,
    symbolLayouts,
    svg: renderSvg(scene),
    width: scene.width,
    height: scene.height,
  };
}

function runLabel(run: BarcodeRun): string {
  const role = run.color === "bar" ? "bar" : "space";
  const width = run.width === "wide" ? "wide" : "narrow";
  return `${role} (${width})`;
}

export function App() {
  const [value, setValue] = useState(DEFAULT_VALUE);

  let model: BarcodeModel | null = null;
  let errorMessage: string | null = null;

  try {
    model = buildBarcodeModel(value);
  } catch (error) {
    errorMessage = error instanceof Error ? error.message : String(error);
  }

  const barCount = model?.runs.filter((run) => run.color === "bar").length ?? 0;
  const spaceCount = model?.runs.filter((run) => run.color === "space").length ?? 0;

  return (
    <div className="app">
      <header className="hero">
        <div>
          <p className="eyebrow">Renderer-First Barcode App</p>
          <h1>Code 39 Visualizer</h1>
          <p className="lede">
            Type any standard Code 39 value and this app will normalize it, encode it, and
            render the resulting barcode as SVG.
          </p>
        </div>

        <dl className="hero-metadata">
          <div>
            <dt>Symbology</dt>
            <dd>Code 39</dd>
          </div>
          <div>
            <dt>Output</dt>
            <dd>SVG</dd>
          </div>
          <div>
            <dt>Pipeline</dt>
            <dd>Input -&gt; Runs -&gt; Draw Scene -&gt; SVG</dd>
          </div>
        </dl>
      </header>

      <main className="layout">
        <section className="panel controls-panel">
          <div className="panel-heading">
            <h2>Input</h2>
            <p>
              Lowercase letters are promoted to uppercase. The <code>*</code> character is
              reserved for the barcode start and stop marker.
            </p>
          </div>

          <label className="field" htmlFor="barcode-input">
            <span>Value to encode</span>
            <input
              id="barcode-input"
              name="barcode-input"
              type="text"
              value={value}
              onChange={(event) => setValue(event.target.value)}
              spellCheck={false}
              autoComplete="off"
            />
          </label>

          <p className="helper">
            Supported characters: <code>{SUPPORTED_CHARACTERS}</code>
          </p>

          {errorMessage === null ? (
            <div className="status ok" role="status">
              The current input is valid Code 39 data.
            </div>
          ) : (
            <div className="status error" role="alert">
              {errorMessage}
            </div>
          )}
        </section>

        <section className="panel preview-panel">
          <div className="panel-heading">
            <h2>Barcode Preview</h2>
            <p>The bars below come from the shared draw scene and are serialized by the SVG backend.</p>
          </div>

          {model === null ? (
            <div className="empty-state">
              Fix the input to generate a barcode preview.
            </div>
          ) : (
            <>
              <div
                className="barcode-frame"
                dangerouslySetInnerHTML={{ __html: model.svg }}
              />
              <div className="barcode-legend" aria-label="encoded symbol alignment">
                <div
                  className="barcode-legend-track"
                  style={{ width: `${model.width}px` }}
                >
                  {model.symbolLayouts.map((layout, index) => (
                    <div
                      key={`${layout.char}-${index}`}
                      className={`barcode-legend-item${layout.isStartStop ? " start-stop" : ""}`}
                      style={{
                        left: `${layout.start}px`,
                        width: `${layout.width}px`,
                      }}
                    >
                      <span>{layout.char}</span>
                    </div>
                  ))}
                </div>
              </div>

              <dl className="stats" aria-label="barcode statistics">
                <div>
                  <dt>Normalized</dt>
                  <dd>{model.normalized}</dd>
                </div>
                <div>
                  <dt>Encoded chars</dt>
                  <dd>{model.encodedCharacters.length}</dd>
                </div>
                <div>
                  <dt>Runs</dt>
                  <dd>{model.runs.length}</dd>
                </div>
                <div>
                  <dt>Bars</dt>
                  <dd>{barCount}</dd>
                </div>
                <div>
                  <dt>Spaces</dt>
                  <dd>{spaceCount}</dd>
                </div>
                <div>
                  <dt>Scene</dt>
                  <dd>
                    {model.width} x {model.height}
                  </dd>
                </div>
              </dl>
            </>
          )}
        </section>

        <section className="panel">
          <div className="panel-heading">
            <h2>Encoded Characters</h2>
            <p>Code 39 wraps the input in start and stop markers before the run expansion happens.</p>
          </div>

          {model === null ? (
            <div className="empty-state">Encoded characters will appear here once the input is valid.</div>
          ) : (
            <div className="token-grid">
              {model.encodedCharacters.map((encodedCharacter, index) => (
                <article className="token-card" key={`${encodedCharacter.char}-${index}`}>
                  <p className="token-label">
                    {encodedCharacter.isStartStop ? "Start / Stop" : `Character ${index}`}
                  </p>
                  <h3>{encodedCharacter.char}</h3>
                  <p className="token-pattern">{encodedCharacter.pattern}</p>
                </article>
              ))}
            </div>
          )}
        </section>

        <section className="panel">
          <div className="panel-heading">
            <h2>Run Stream</h2>
            <p>
              Every symbol becomes alternating bars and spaces. Inter-character gaps are represented as narrow spaces.
            </p>
          </div>

          {model === null ? (
            <div className="empty-state">The run stream becomes available once the input is valid.</div>
          ) : (
            <div className="run-strip" aria-label="barcode run stream">
              {model.runs.map((run, index) => (
                <div
                  key={`${run.sourceChar}-${index}`}
                  className={`run-chip ${run.color} ${run.width}`}
                  title={`${run.sourceChar} -> ${runLabel(run)}`}
                >
                  <span>{run.sourceChar}</span>
                  <small>{runLabel(run)}</small>
                  {run.isInterCharacterGap ? <em>gap</em> : null}
                </div>
              ))}
            </div>
          )}
        </section>
      </main>
    </div>
  );
}
