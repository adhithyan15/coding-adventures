/**
 * Voltage Transfer Characteristic (VTC) Chart.
 *
 * Plots the CMOS inverter's input-output voltage relationship as an
 * SVG line chart. The VTC shows the defining feature of CMOS: the
 * output snaps sharply from HIGH to LOW over a very narrow input
 * voltage range, making it ideal for digital logic.
 *
 * === The VTC Shape ===
 *
 *   Output (V)
 *   Vdd ┤ ━━━━━━━━━┓
 *       │           ┃ ← Sharp transition
 *       │           ┃   (switching threshold)
 *     0 ┤           ┗━━━━━━━━━━
 *       └──────────────────────
 *       0                   Vdd
 *                 Input (V)
 *
 * This sharp transition means:
 *   - Clear distinction between logic 0 and 1
 *   - High noise immunity (can tolerate voltage fluctuations)
 *   - Very little time spent in the "undefined" transition region
 */

import { useTranslation } from "@coding-adventures/ui-components";

interface VtcChartProps {
  /** Array of [inputV, outputV] points from the CMOS inverter model. */
  vtc: [number, number][];
}

export function VtcChart({ vtc }: VtcChartProps) {
  const { t } = useTranslation();

  // Chart dimensions within the SVG viewBox
  const chartX = 60;
  const chartY = 20;
  const chartW = 280;
  const chartH = 180;

  // Find the voltage range from the data
  const maxVin = vtc.length > 0 ? vtc[vtc.length - 1]![0] : 3.3;
  const maxVout = maxVin; // VTC is typically Vdd on both axes

  // Convert data points to SVG path coordinates
  const pathD = vtc
    .map(([vin, vout], i) => {
      const x = chartX + (vin / maxVin) * chartW;
      const y = chartY + chartH - (vout / maxVout) * chartH;
      return `${i === 0 ? "M" : "L"} ${x.toFixed(1)} ${y.toFixed(1)}`;
    })
    .join(" ");

  // Find the switching threshold — the point where Vout crosses Vdd/2
  const thresholdPoint = vtc.find(([, vout]) => vout <= maxVout / 2);
  const thresholdX = thresholdPoint
    ? chartX + (thresholdPoint[0] / maxVin) * chartW
    : chartX + chartW / 2;

  return (
    <div className="vtc-chart">
      <h3 className="vtc-chart__title">{t("era4.vtc.title")}</h3>
      <svg
        viewBox="0 0 380 240"
        className="vtc-chart__svg"
        role="img"
        aria-label={t("era4.vtc.title")}
      >
        {/* Axes */}
        <line
          x1={chartX}
          y1={chartY + chartH}
          x2={chartX + chartW}
          y2={chartY + chartH}
          stroke="#666"
          strokeWidth="1.5"
        />
        <line
          x1={chartX}
          y1={chartY}
          x2={chartX}
          y2={chartY + chartH}
          stroke="#666"
          strokeWidth="1.5"
        />

        {/* X axis label */}
        <text
          x={chartX + chartW / 2}
          y={chartY + chartH + 35}
          textAnchor="middle"
          fontSize="11"
          fill="#666"
        >
          {t("era4.vtc.inputAxis")}
        </text>

        {/* Y axis label */}
        <text
          x={15}
          y={chartY + chartH / 2}
          textAnchor="middle"
          fontSize="11"
          fill="#666"
          transform={`rotate(-90, 15, ${chartY + chartH / 2})`}
        >
          {t("era4.vtc.outputAxis")}
        </text>

        {/* Tick marks and labels */}
        {[0, maxVin / 2, maxVin].map((v, i) => {
          const x = chartX + (v / maxVin) * chartW;
          return (
            <g key={`x-${i}`}>
              <line x1={x} y1={chartY + chartH} x2={x} y2={chartY + chartH + 5} stroke="#666" />
              <text x={x} y={chartY + chartH + 18} textAnchor="middle" fontSize="9" fill="#888">
                {v.toFixed(1)}
              </text>
            </g>
          );
        })}
        {[0, maxVout / 2, maxVout].map((v, i) => {
          const y = chartY + chartH - (v / maxVout) * chartH;
          return (
            <g key={`y-${i}`}>
              <line x1={chartX - 5} y1={y} x2={chartX} y2={y} stroke="#666" />
              <text x={chartX - 8} y={y + 3} textAnchor="end" fontSize="9" fill="#888">
                {v.toFixed(1)}
              </text>
            </g>
          );
        })}

        {/* Switching threshold marker — dashed vertical line */}
        <line
          x1={thresholdX}
          y1={chartY}
          x2={thresholdX}
          y2={chartY + chartH}
          stroke="#cc6666"
          strokeWidth="1"
          strokeDasharray="4 3"
          opacity="0.6"
        />

        {/* VTC curve — the characteristic sharp transition of CMOS */}
        <path
          d={pathD}
          fill="none"
          stroke="#4488cc"
          strokeWidth="2.5"
          strokeLinejoin="round"
        />
      </svg>
    </div>
  );
}
