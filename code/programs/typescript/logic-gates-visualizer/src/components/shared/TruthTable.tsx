/**
 * TruthTable — renders a complete truth table for a logic gate.
 *
 * === What is a truth table? ===
 *
 * A truth table is the most fundamental way to define a logic gate. It
 * exhaustively lists every possible input combination and the corresponding
 * output. Since each input is binary (0 or 1), a gate with N inputs has
 * exactly 2^N rows in its truth table.
 *
 *   - NOT gate (1 input):  2^1 = 2 rows
 *   - AND gate (2 inputs): 2^2 = 4 rows
 *   - 3-input AND:         2^3 = 8 rows
 *
 * === Interactive highlighting ===
 *
 * The `activeRow` prop highlights the row matching the current input values.
 * As the user toggles inputs on the gate card, the corresponding truth table
 * row lights up green — connecting the abstract table to the live circuit.
 *
 * === Accessibility ===
 *
 * Uses proper HTML table semantics:
 *   - <caption> for the table title (visible to screen readers)
 *   - <thead>/<tbody> for structure
 *   - <th scope="col"> for column headers
 *   - aria-current="true" on the active row
 */

import type { Bit } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";

export interface TruthTableRow {
  inputs: Bit[];
  output: Bit;
}

export interface TruthTableProps {
  /** Column headers for inputs (e.g., ["A"] for NOT, ["A", "B"] for AND). */
  inputs: string[];
  /** Column header for the output (e.g., "Out"). */
  output: string;
  /** All rows of the truth table. */
  rows: TruthTableRow[];
  /** Index of the currently active row (highlighted). */
  activeRow?: number;
}

export function TruthTable({ inputs, output, rows, activeRow }: TruthTableProps) {
  const { t } = useTranslation();

  return (
    <table className="truth-table">
      <caption>{t("truthTable.title")}</caption>
      <thead>
        <tr>
          {inputs.map((header) => (
            <th key={header} scope="col">
              {header}
            </th>
          ))}
          <th scope="col">{output}</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((row, index) => {
          const isActive = index === activeRow;
          return (
            <tr
              key={index}
              className={isActive ? "truth-table__row--active" : ""}
              aria-current={isActive ? "true" : undefined}
            >
              {row.inputs.map((val, i) => (
                <td key={i}>{val}</td>
              ))}
              <td>{row.output}</td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
