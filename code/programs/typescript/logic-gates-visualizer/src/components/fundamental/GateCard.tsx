/**
 * GateCard — a self-contained visualization of a single logic gate.
 *
 * === What this component shows ===
 *
 * Each GateCard brings together every aspect of a logic gate in one place:
 *
 *   1. Name and description — what the gate does, in plain English
 *   2. Gate symbol (SVG) — the standard IEEE schematic symbol
 *   3. Input toggles — clickable buttons to set each input to 0 or 1
 *   4. Output display — the gate's output for the current inputs
 *   5. Truth table — every possible input/output combination, with the
 *      current row highlighted
 *   6. CMOS panel — expandable view of the transistor implementation
 *
 * === How it works internally ===
 *
 * The component manages input state via useState. When the user toggles
 * an input, React re-renders the component. The gate function (NOT, AND,
 * OR, XOR) is called with the new inputs to compute the output. The truth
 * table's active row is determined by converting the inputs to a row index:
 *
 *   For a 1-input gate (NOT): row = inputA (0 or 1)
 *   For a 2-input gate (AND): row = inputA * 2 + inputB (0, 1, 2, or 3)
 *
 * This works because truth tables list rows in binary counting order:
 *   Row 0: A=0, B=0
 *   Row 1: A=0, B=1
 *   Row 2: A=1, B=0
 *   Row 3: A=1, B=1
 */

import { useState } from "react";
import type { Bit } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";
import { BitToggle } from "../shared/BitToggle.js";
import { GateSymbol } from "../shared/GateSymbol.js";
import type { GateType } from "../shared/GateSymbol.js";
import { TruthTable } from "../shared/TruthTable.js";
import type { TruthTableRow } from "../shared/TruthTable.js";
import { WireLabel } from "../shared/WireLabel.js";
import { CmosPanel } from "../shared/CmosPanel.js";

export interface GateCardProps {
  /** The type of gate (determines symbol shape and CMOS diagram). */
  gateType: GateType;
  /** The gate function to compute output from inputs. */
  gateFn: (...args: Bit[]) => Bit;
  /** Labels for each input (e.g., ["A"] for NOT, ["A", "B"] for AND). */
  inputLabels: string[];
  /** i18n key prefix for this gate (e.g., "gate.not"). */
  i18nPrefix: string;
}

/**
 * Generate all truth table rows for a gate.
 *
 * For N inputs, generates 2^N rows. Each row lists the inputs in binary
 * counting order and the gate's output for those inputs.
 *
 * Example for AND (2 inputs):
 *   [0,0] -> 0
 *   [0,1] -> 0
 *   [1,0] -> 0
 *   [1,1] -> 1
 */
function generateTruthTable(
  inputCount: number,
  gateFn: (...args: Bit[]) => Bit,
): TruthTableRow[] {
  const rowCount = 1 << inputCount; // 2^inputCount
  const rows: TruthTableRow[] = [];

  for (let i = 0; i < rowCount; i++) {
    const inputs: Bit[] = [];
    // Extract each bit from the row index.
    // Most significant bit = first input.
    for (let bit = inputCount - 1; bit >= 0; bit--) {
      inputs.push(((i >> bit) & 1) as Bit);
    }
    const output = gateFn(...inputs);
    rows.push({ inputs, output });
  }

  return rows;
}

/**
 * Compute the active truth table row index from current inputs.
 *
 * Converts the input values to a binary number:
 *   [0]    -> 0
 *   [1]    -> 1
 *   [0, 0] -> 0
 *   [0, 1] -> 1
 *   [1, 0] -> 2
 *   [1, 1] -> 3
 */
function inputsToRowIndex(inputs: Bit[]): number {
  let index = 0;
  for (const bit of inputs) {
    index = (index << 1) | bit;
  }
  return index;
}

export function GateCard({ gateType, gateFn, inputLabels, i18nPrefix }: GateCardProps) {
  const { t } = useTranslation();
  const inputCount = inputLabels.length;

  // Initialize all inputs to 0.
  const [inputs, setInputs] = useState<Bit[]>(() =>
    new Array(inputCount).fill(0) as Bit[],
  );

  // Compute the gate's output from the current inputs.
  const output = gateFn(...inputs);

  // Generate the full truth table (static — doesn't change).
  const truthRows = generateTruthTable(inputCount, gateFn);

  // Find which truth table row matches the current inputs.
  const activeRow = inputsToRowIndex(inputs);

  // Handler: toggle a specific input by index.
  const toggleInput = (index: number, newValue: Bit) => {
    setInputs((prev) => {
      const next = [...prev];
      next[index] = newValue;
      return next;
    });
  };

  return (
    <div className="gate-card">
      {/* --- Header: gate name --- */}
      <div className="gate-card__header">
        <h3 className="gate-card__name">{t(`${i18nPrefix}.name`)}</h3>
      </div>

      {/* --- Description --- */}
      <p className="gate-card__description">{t(`${i18nPrefix}.description`)}</p>

      {/* --- Interactive diagram: inputs -> gate symbol -> output --- */}
      <div className="gate-card__diagram">
        <div className="gate-card__inputs">
          {inputLabels.map((label, i) => (
            <BitToggle
              key={label}
              value={inputs[i]}
              onChange={(v) => toggleInput(i, v)}
              label={label}
            />
          ))}
        </div>

        <GateSymbol
          type={gateType}
          inputA={inputs[0]}
          inputB={inputs[1]}
          output={output}
          width={100}
          height={75}
        />

        <div className="gate-card__output">
          <WireLabel value={output} label="Out" />
        </div>
      </div>

      {/* --- Truth table with active row highlight --- */}
      <TruthTable
        inputs={inputLabels}
        output="Out"
        rows={truthRows}
        activeRow={activeRow}
      />

      {/* --- CMOS transistor implementation (expandable) --- */}
      <CmosPanel
        gateType={gateType}
        inputA={inputs[0]}
        inputB={inputs[1]}
      />
    </div>
  );
}
