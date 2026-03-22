/**
 * Scaling Timeline — visual display of CMOS technology scaling (Moore's Law).
 *
 * Shows how key metrics change as transistors shrink from 180nm to 3nm:
 *   - Supply voltage (Vdd) decreases -> less power per switch
 *   - Propagation delay decreases -> faster switching
 *   - Dynamic power decreases -> more energy efficient
 *   - But leakage current increases -> the "leakage wall"
 *
 * Rendered as a table for accessibility and clarity. Each row is a
 * technology node with its key metrics.
 */

import { useTranslation } from "@coding-adventures/ui-components";

interface ScalingTimelineProps {
  /** Array of scaling data from demonstrateCmosScaling(). */
  scaling: Record<string, number>[];
}

/**
 * Format a number in engineering notation with the appropriate SI prefix.
 * Handles the tiny numbers common in semiconductor physics.
 */
function formatEngineering(value: number, unit: string): string {
  if (value === 0) return `0 ${unit}`;
  const absVal = Math.abs(value);
  if (absVal >= 1) return `${value.toFixed(2)} ${unit}`;
  if (absVal >= 1e-3) return `${(value * 1e3).toFixed(2)} m${unit}`;
  if (absVal >= 1e-6) return `${(value * 1e6).toFixed(2)} u${unit}`;
  if (absVal >= 1e-9) return `${(value * 1e9).toFixed(2)} n${unit}`;
  if (absVal >= 1e-12) return `${(value * 1e12).toFixed(2)} p${unit}`;
  return `${(value * 1e15).toFixed(2)} f${unit}`;
}

export function ScalingTimeline({ scaling }: ScalingTimelineProps) {
  const { t } = useTranslation();

  return (
    <div className="scaling-timeline">
      <h3 className="scaling-timeline__title">{t("era4.scaling.title")}</h3>
      <div className="scaling-timeline__table-wrapper">
        <table className="scaling-timeline__table">
          <thead>
            <tr>
              <th>Node</th>
              <th>Vdd</th>
              <th>Delay</th>
              <th>Power</th>
            </tr>
          </thead>
          <tbody>
            {scaling.map((node, i) => (
              <tr key={i}>
                <td className="scaling-timeline__node">
                  {node.node_nm! >= 1
                    ? `${node.node_nm!.toFixed(0)} nm`
                    : `${(node.node_nm! * 1000).toFixed(0)} pm`}
                </td>
                <td>{node.vdd_v!.toFixed(2)} V</td>
                <td>{formatEngineering(node.propagation_delay_s!, "s")}</td>
                <td>{formatEngineering(node.dynamic_power_w!, "W")}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
