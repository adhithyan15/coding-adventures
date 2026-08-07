import { useMemo, useState } from "react";
import { cAbiCatalog, traceCAbi, type CAbiProbeId } from "./rust-c-abi.js";

const PROBES: readonly CAbiProbeId[] = ["success", "null-input", "empty-input", "short-output", "non-finite", "overlapping-output"];

function label(probe: CAbiProbeId): string {
  return probe === "success" ? "paper example" : probe.replaceAll("-", " ");
}
export function RustCAbiWorkbench() {
  const [probeId, setProbeId] = useState<CAbiProbeId>("success");
  const trace = useMemo(() => traceCAbi(probeId), [probeId]);

  return (
    <main className="workspace workspace--language-consumers workspace--rust-c-abi">
      <section className="consumer-stage">
        <header className="consumer-intro">
          <div>
            <p className="eyebrow">NN35 - stable foreign-function boundary</p>
            <h2>Rust C ABI</h2>
            <p>{trace.catalog.question}</p>
          </div>
          <span className="consumer-chip">ABI {trace.catalog.versionHex}</span>
        </header>

        <section className="consumer-paper" aria-label="Rust C ABI hand calculation">
          <div className="panel-heading">
            <div><p className="eyebrow">1 - keep the arithmetic visible</p><h2>The same trace crosses the boundary</h2></div>
            <span className="consumer-result consumer-result--pass">status 0</span>
          </div>
          <div className="consumer-products">
            <div><small>contribution 1</small><code>2.0 * 0.5</code><strong>= 1.0</strong></div>
            <div><small>contribution 2</small><code>-1.0 * -0.25</code><strong>= 0.25</strong></div>
            <div><small>bias</small><code>1.0 + 0.25 + 0.1</code><strong>= 1.35</strong></div>
            <div><small>identity</small><code>identity(1.35)</code><strong>= 1.35</strong></div>
          </div>
        </section>

        <section className="consumer-selected" aria-label="Versioned C function contract">
          <div className="panel-heading">
            <div><p className="eyebrow">2 - freeze the seam</p><h2>One versioned compute function</h2></div>
          </div>
          <code className="consumer-command">{trace.catalog.functions[2]}</code>
          <div className="consumer-receipt">
            <span>caller owns <code>inputs</code></span>
            <span>caller owns <code>weights</code></span>
            <span>caller owns <code>contributions_out</code></span>
            <span>caller owns <code>prediction_out</code></span>
          </div>
        </section>

        <section className="consumer-lanes" aria-label="C ABI success and failure probes">
          <div className="panel-heading">
            <div><p className="eyebrow">3 - probe the boundary</p><h2>Select a deterministic call</h2></div>
            <span className={trace.status.code === 0 ? "consumer-result consumer-result--pass" : "consumer-result consumer-result--fail"}>
              status {trace.status.code}
            </span>
          </div>
          <div className="consumer-lane-grid rust-c-abi-probes">
            {PROBES.map((probe) => (
              <button
                aria-label={`Inspect ${label(probe)} C ABI probe`}
                aria-pressed={probe === probeId}
                key={probe}
                onClick={() => setProbeId(probe)}
                type="button"
              >
                <small>{probe === "success" ? "writes after validation" : "fails before writes"}</small>
                <strong>{label(probe)}</strong>
                <span>{probe === "success" ? "NEURAL_LEARNING_OK" : cAbiCatalog.statuses[cAbiCatalog.probes.find((item) => item.id === probe)?.expectedStatus ?? 0]?.symbol}</span>
              </button>
            ))}
          </div>
        </section>

        <section className="consumer-protocol" aria-label="Selected C ABI trace">
          <div className="panel-heading">
            <div><p className="eyebrow">4 - inspect the result</p><h2>{trace.status.symbol}</h2></div>
            <code>{trace.status.message}</code>
          </div>
          <ol>
            <li><span>1</span><p>{trace.boundaryCheck}</p></li>
            <li><span>2</span><p>return status <strong>{trace.status.code}</strong></p></li>
            <li><span>3</span><p>contribution outputs {trace.outputsWritten ? "become [1.0, 0.25]" : "stay byte-for-byte unchanged"}</p></li>
            <li><span>4</span><p>prediction output {trace.outputsWritten ? "becomes 1.35" : "stays byte-for-byte unchanged"}</p></li>
          </ol>
        </section>
      </section>

      <aside className="consumer-controls">
        <p className="eyebrow">Stable means boring on purpose</p>
        <h2>No Rust layout leaks out</h2>
        <code className="consumer-command">{trace.catalog.header}</code>
        <section><p className="eyebrow">Fixed vocabulary</p><p>Only C pointers, IEEE-754 doubles, `uint64_t` lengths, and `uint32_t` statuses cross the seam.</p></section>
        <section><p className="eyebrow">Closed writes</p><p>The core validates the complete call and arithmetic before touching caller-owned output memory.</p></section>
        <section><p className="eyebrow">No allocator handshake</p><p>The caller allocates and frees every buffer, so Rust and foreign runtimes never mix allocators.</p></section>
        <section><p className="eyebrow">Browser evidence</p><p>This workbench recomputes the committed catalog. The Python validator dynamically loads the compiled native library.</p></section>
      </aside>
    </main>
  );
}
