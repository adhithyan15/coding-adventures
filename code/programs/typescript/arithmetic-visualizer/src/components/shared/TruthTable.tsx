/**
 * TruthTable — renders a complete truth table for an arithmetic circuit.
 *
 * Supports multiple output columns (unlike the logic-gates version which
 * has a single output), since adders produce both Sum and Carry outputs.
 *
 * === Interactive highlighting ===
 *
 * The `activeRow` prop highlights the row matching the current input values.
 * As the user toggles inputs, the corresponding truth table row lights up.
 *
 * === Accessibility ===
 *
 * Uses proper HTML table semantics with caption, thead/tbody, th scope.
 */

import type { Bit } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";

export interface TruthTableRow {
  inputs: Bit[];
  outputs: Bit[];
}

export interface TruthTableProps {
  /** Column headers for inputs (e.g., ["A", "B"]). */
  inputHeaders: string[];
  /** Column headers for outputs (e.g., ["Sum", "Carry"]). */
  outputHeaders: string[];
  /** All rows of the truth table. */
  rows: TruthTableRow[];
  /** Index of the currently active row (highlighted). */
  activeRow?: number;
}

export function TruthTable({ inputHeaders, outputHeaders, rows, activeRow }: TruthTableProps) {
  const { t } = useTranslation();

  return (
    <table className="truth-table">
      <caption>{t("truthTable.title")}</caption>
      <thead>
        <tr>
          {inputHeaders.map((header) => (
            <th key={header} scope="col">
              {header}
            </th>
          ))}
          {outputHeaders.map((header) => (
            <th key={header} scope="col">
              {header}
            </th>
          ))}
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
                <td key={`in-${i}`}>{val}</td>
              ))}
              {row.outputs.map((val, i) => (
                <td key={`out-${i}`}>{val}</td>
              ))}
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
