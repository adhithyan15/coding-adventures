/**
 * OperationSelector — row of buttons to select the ALU operation.
 *
 * Groups operations into two categories:
 *   - Arithmetic: ADD, SUB (use the adder)
 *   - Logic: AND, OR, XOR, NOT (bitwise gates)
 *
 * The selected operation is highlighted with a colored border.
 */

import { ALUOp } from "@coding-adventures/arithmetic";
import { useTranslation } from "@coding-adventures/ui-components";

export interface OperationSelectorProps {
  /** Currently selected operation. */
  selected: ALUOp;
  /** Called when the user selects a different operation. */
  onSelect: (op: ALUOp) => void;
}

const ARITHMETIC_OPS = [ALUOp.ADD, ALUOp.SUB];
const LOGIC_OPS = [ALUOp.AND, ALUOp.OR, ALUOp.XOR, ALUOp.NOT];

const OP_LABELS: Record<ALUOp, string> = {
  [ALUOp.ADD]: "ADD",
  [ALUOp.SUB]: "SUB",
  [ALUOp.AND]: "AND",
  [ALUOp.OR]: "OR",
  [ALUOp.XOR]: "XOR",
  [ALUOp.NOT]: "NOT",
};

export function OperationSelector({ selected, onSelect }: OperationSelectorProps) {
  const { t } = useTranslation();

  return (
    <div className="op-selector" role="radiogroup" aria-label={t("alu.opSelector")}>
      <div className="op-selector__group">
        <span className="op-selector__group-label">{t("alu.arithmetic")}</span>
        {ARITHMETIC_OPS.map((op) => (
          <button
            key={op}
            className={`op-selector__btn ${selected === op ? "op-selector__btn--active" : ""}`}
            onClick={() => onSelect(op)}
            role="radio"
            aria-checked={selected === op}
            type="button"
          >
            {OP_LABELS[op]}
          </button>
        ))}
      </div>
      <div className="op-selector__group">
        <span className="op-selector__group-label">{t("alu.logic")}</span>
        {LOGIC_OPS.map((op) => (
          <button
            key={op}
            className={`op-selector__btn ${selected === op ? "op-selector__btn--active" : ""}`}
            onClick={() => onSelect(op)}
            role="radio"
            aria-checked={selected === op}
            type="button"
          >
            {OP_LABELS[op]}
          </button>
        ))}
      </div>
    </div>
  );
}
