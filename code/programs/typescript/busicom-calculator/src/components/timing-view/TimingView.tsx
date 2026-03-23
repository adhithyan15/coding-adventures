/**
 * Timing View — Layer 6: Clock and instruction timing.
 *
 * === The Intel 4004's Clock ===
 *
 * The 4004 uses a two-phase non-overlapping clock (Phi1 and Phi2). Each
 * instruction takes exactly 8 clock phases organized into a machine cycle:
 *
 *   A1 → A2 → A3 → M1 → M2 → X1 → X2 → X3
 *
 * - A1-A3: Address phase — PC sends address to ROM
 * - M1-M2: Memory phase — ROM returns instruction byte
 * - X1-X3: Execute phase — instruction is decoded and executed
 *
 * Two-byte instructions (FIM, JUN, JMS, JCN, ISZ) take two machine cycles
 * (16 clock phases) — one cycle per byte.
 *
 * This view shows:
 *   1. Clock waveform (Phi1/Phi2) as SVG
 *   2. Phase activity table for the current instruction
 *   3. Multi-instruction timeline from trace history
 */

import { useTranslation } from "../../i18n/index.js";
import type { DetailedTrace } from "../../cpu/types.js";

/** Props for the Timing view. */
interface TimingViewProps {
  trace: DetailedTrace | undefined;
  traceHistory: readonly DetailedTrace[];
}

/**
 * Determines if an instruction is two bytes (two machine cycles).
 * Two-byte instructions: FIM, JUN, JMS, JCN, ISZ.
 */
function isTwoByte(trace: DetailedTrace): boolean {
  const decoded = trace.decoded;
  return !!(decoded.isFim || decoded.isJun || decoded.isJms ||
            decoded.isJcn || decoded.isIsz);
}

/**
 * Get phase activity descriptions for an instruction.
 * Returns 8 or 16 phase labels depending on instruction size.
 */
function getPhaseActivities(trace: DetailedTrace, t: (key: string) => string): string[] {
  const twoByte = isTwoByte(trace);
  const phases: string[] = [
    t("timing.phase.a1.desc"),
    t("timing.phase.a2.desc"),
    t("timing.phase.a3.desc"),
    t("timing.phase.m1.desc"),
    t("timing.phase.m2.desc"),
    t("timing.phase.x1.desc"),
    t("timing.phase.x2.desc"),
    t("timing.phase.x3.desc"),
  ];

  if (twoByte) {
    phases.push(
      t("timing.phase.a1.desc2"),
      t("timing.phase.a2.desc2"),
      t("timing.phase.a3.desc2"),
      t("timing.phase.m1.desc2"),
      t("timing.phase.m2.desc2"),
      t("timing.phase.x1.desc2"),
      t("timing.phase.x2.desc2"),
      t("timing.phase.x3.desc2"),
    );
  }

  return phases;
}

/** Phase names for the 8-phase machine cycle. */
const PHASE_NAMES = ["A1", "A2", "A3", "M1", "M2", "X1", "X2", "X3"];

/** Phase group labels. */
const PHASE_GROUPS = [
  { name: "Address", start: 0, end: 3, color: "var(--wire-high)" },
  { name: "Memory", start: 3, end: 5, color: "var(--accent)" },
  { name: "Execute", start: 5, end: 8, color: "#6699ff" },
];

/**
 * SVG clock waveform showing Phi1 and Phi2 non-overlapping clocks.
 *
 * The 4004's two-phase clock ensures no overlapping: when Phi1 is high,
 * Phi2 is low and vice versa, with brief dead times between transitions.
 */
function ClockWaveform({ cycles, t }: { cycles: number; t: (key: string) => string }) {
  const phaseWidth = 28;
  const totalWidth = cycles * 8 * phaseWidth + 60;
  const height = 120;
  const topY = 20;
  const bottomY = 50;
  const waveHeight = 20;
  const labelX = 4;

  /**
   * Generate the SVG path for a clock signal.
   * Phi1 is high during even phases (0,2,4,6), Phi2 during odd (1,3,5,7).
   */
  function clockPath(phase: 1 | 2): string {
    const parts: string[] = [];
    const baseY = phase === 1 ? topY + waveHeight : bottomY + waveHeight;
    const startX = 50;
    parts.push(`M ${startX} ${baseY}`);

    for (let c = 0; c < cycles; c++) {
      for (let p = 0; p < 8; p++) {
        const x = startX + (c * 8 + p) * phaseWidth;
        const isHigh = phase === 1 ? p % 2 === 0 : p % 2 === 1;
        if (isHigh) {
          parts.push(`L ${x} ${baseY}`);
          parts.push(`L ${x} ${baseY - waveHeight}`);
          parts.push(`L ${x + phaseWidth} ${baseY - waveHeight}`);
          parts.push(`L ${x + phaseWidth} ${baseY}`);
        } else {
          parts.push(`L ${x + phaseWidth} ${baseY}`);
        }
      }
    }

    return parts.join(" ");
  }

  return (
    <svg
      viewBox={`0 0 ${totalWidth} ${height}`}
      className="clock-waveform"
      role="img"
      aria-label={t("timing.waveform.label")}
    >
      {/* Phase group backgrounds */}
      {Array.from({ length: cycles }).map((_, c) =>
        PHASE_GROUPS.map((group) => (
          <rect
            key={`${c}-${group.name}`}
            x={50 + (c * 8 + group.start) * phaseWidth}
            y={0}
            width={(group.end - group.start) * phaseWidth}
            height={height}
            fill={group.color}
            opacity={0.04}
          />
        )),
      )}

      {/* Phase labels */}
      {Array.from({ length: cycles }).map((_, c) =>
        PHASE_NAMES.map((name, p) => (
          <text
            key={`${c}-${name}`}
            x={50 + (c * 8 + p) * phaseWidth + phaseWidth / 2}
            y={height - 4}
            textAnchor="middle"
            fill="#8899aa"
            fontSize="9"
            fontFamily="var(--mono)"
          >
            {name}
          </text>
        )),
      )}

      {/* Clock signal labels */}
      <text x={labelX} y={topY + waveHeight / 2 + 4} fill="var(--wire-high)" fontSize="11" fontFamily="var(--mono)">
        {t("timing.phi1")}
      </text>
      <text x={labelX} y={bottomY + waveHeight / 2 + 4} fill="var(--accent)" fontSize="11" fontFamily="var(--mono)">
        {t("timing.phi2")}
      </text>

      {/* Clock waveforms */}
      <path d={clockPath(1)} stroke="var(--wire-high)" strokeWidth="2" fill="none" />
      <path d={clockPath(2)} stroke="var(--accent)" strokeWidth="2" fill="none" />
    </svg>
  );
}

/**
 * Phase activity table for the current instruction.
 *
 * Shows what happens during each phase of the machine cycle.
 */
function PhaseTable({ trace, t }: { trace: DetailedTrace; t: (key: string) => string }) {
  const activities = getPhaseActivities(trace, t);
  const twoByte = isTwoByte(trace);
  const cycleCount = twoByte ? 2 : 1;

  return (
    <div className="phase-table" role="table" aria-label={t("timing.phaseTable.label")}>
      <div className="phase-table-header" role="row">
        <div className="phase-table-cell phase-table-cell--header" role="columnheader">
          {t("timing.phase")}
        </div>
        <div className="phase-table-cell phase-table-cell--header" role="columnheader">
          {t("timing.group")}
        </div>
        <div className="phase-table-cell phase-table-cell--header phase-table-cell--wide" role="columnheader">
          {t("timing.activity")}
        </div>
      </div>
      {Array.from({ length: cycleCount }).map((_, cycle) =>
        PHASE_NAMES.map((name, idx) => {
          const phaseIdx = cycle * 8 + idx;
          const group = PHASE_GROUPS.find((g) => idx >= g.start && idx < g.end);
          return (
            <div
              key={`${cycle}-${name}`}
              className="phase-table-row"
              role="row"
            >
              <div className="phase-table-cell phase-table-cell--phase" role="cell">
                {twoByte ? `C${cycle + 1}:${name}` : name}
              </div>
              <div
                className="phase-table-cell phase-table-cell--group"
                role="cell"
                style={{ color: group?.color }}
              >
                {group?.name}
              </div>
              <div className="phase-table-cell phase-table-cell--activity" role="cell">
                {activities[phaseIdx] ?? ""}
              </div>
            </div>
          );
        }),
      )}
    </div>
  );
}

/**
 * Instruction timeline — shows the last N instructions with their timing.
 */
function InstructionTimeline({ traceHistory, t }: {
  traceHistory: readonly DetailedTrace[];
  t: (key: string) => string;
}) {
  // Show last 8 instructions
  const recent = traceHistory.slice(-8);

  return (
    <div className="instruction-timeline" aria-label={t("timing.timeline.label")}>
      <h3>{t("timing.timeline.title")}</h3>
      <div className="timeline-grid">
        {recent.map((trace, i) => {
          const twoByte = isTwoByte(trace);
          return (
            <div key={i} className="timeline-entry">
              <div className="timeline-addr">
                0x{trace.snapshot.pc.toString(16).padStart(3, "0")}
              </div>
              <div className="timeline-mnemonic">{trace.mnemonic}</div>
              <div className="timeline-cycles">
                {/* Visual cycle blocks */}
                <div className="cycle-blocks">
                  {PHASE_GROUPS.map((group) => (
                    <div
                      key={group.name}
                      className="cycle-block"
                      style={{
                        width: `${((group.end - group.start) / 8) * 100}%`,
                        background: group.color,
                        opacity: 0.6,
                      }}
                      title={group.name}
                    />
                  ))}
                  {twoByte && PHASE_GROUPS.map((group) => (
                    <div
                      key={`2-${group.name}`}
                      className="cycle-block"
                      style={{
                        width: `${((group.end - group.start) / 8) * 100}%`,
                        background: group.color,
                        opacity: 0.3,
                      }}
                      title={`${group.name} (cycle 2)`}
                    />
                  ))}
                </div>
              </div>
              <div className="timeline-duration">
                {twoByte ? "2" : "1"} {t("timing.cycles")}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

/**
 * Main Timing view component.
 */
export function TimingView({ trace, traceHistory }: TimingViewProps) {
  const { t } = useTranslation();

  return (
    <div className="timing-view" role="region" aria-label={t("timing.title")}>
      <h2>{t("timing.title")}</h2>
      <p>{t("timing.description")}</p>

      {/* Clock waveform */}
      <div className="timing-section">
        <h3>{t("timing.clock.title")}</h3>
        <p className="timing-section-desc">{t("timing.clock.description")}</p>
        <ClockWaveform cycles={2} t={t} />
      </div>

      {/* Phase activity for current instruction */}
      {trace ? (
        <div className="timing-section">
          <h3>
            {t("timing.currentInstruction")}: <code>{trace.mnemonic}</code>
          </h3>
          <PhaseTable trace={trace} t={t} />
        </div>
      ) : (
        <div className="timing-section">
          <p className="timing-no-instruction">{t("timing.noInstruction")}</p>
        </div>
      )}

      {/* Instruction timeline */}
      {traceHistory.length > 0 && (
        <div className="timing-section">
          <InstructionTimeline traceHistory={traceHistory} t={t} />
        </div>
      )}
    </div>
  );
}
