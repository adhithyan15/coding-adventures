/**
 * ALUView — Tab 3 container: the Arithmetic Logic Unit.
 *
 * "One circuit to rule them all." The ALU takes two N-bit operands and
 * an operation code, and produces a result plus four condition flags.
 *
 * === Architecture ===
 *
 * - Operation selector (6 buttons: ADD/SUB/AND/OR/XOR/NOT)
 * - Two 8-bit operand inputs (A and B)
 * - Result display with bits, decimal, hex, and flags
 *
 * Uses the ALU class from @coding-adventures/arithmetic which routes
 * through real logic gate functions internally.
 */

import { useState, useMemo } from "react";
import { ALU, ALUOp } from "@coding-adventures/arithmetic";
import type { Bit } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";
import { BitGroup } from "../shared/BitGroup.js";
import { OperationSelector } from "./OperationSelector.js";
import { ResultDisplay } from "./ResultDisplay.js";

const BIT_WIDTH = 8;

/** Create a zero-filled bit array of the given width. */
function zeroBits(width: number): Bit[] {
  return new Array<Bit>(width).fill(0 as Bit);
}

/** Convert an integer to LSB-first bit array. */
function intToBits(n: number, width: number): Bit[] {
  return Array.from({ length: width }, (_, i) => ((n >> i) & 1) as Bit);
}

export function ALUView() {
  const { t } = useTranslation();
  const [op, setOp] = useState<ALUOp>(ALUOp.ADD);
  const [aBits, setABits] = useState<Bit[]>(intToBits(42, BIT_WIDTH)); // 42
  const [bBits, setBBits] = useState<Bit[]>(intToBits(15, BIT_WIDTH)); // 15

  // Create ALU instance (memoized — it's stateless)
  const alu = useMemo(() => new ALU(BIT_WIDTH), []);

  // Execute the operation
  const bInput = op === ALUOp.NOT ? zeroBits(BIT_WIDTH) : bBits;
  const result = alu.execute(op, aBits, bInput);

  const isUnary = op === ALUOp.NOT;

  return (
    <div className="alu-view">
      <p className="alu-view__intro">{t("alu.intro")}</p>

      <section className="alu-view__panel" aria-label={t("alu.panelLabel")}>
        <h3 className="alu-view__title">{t("alu.title")}</h3>

        {/* Operation selector */}
        <OperationSelector selected={op} onSelect={setOp} />

        {/* Operand inputs */}
        <div className="alu-view__operands">
          <BitGroup bits={aBits} onChange={setABits} label="A" />
          {!isUnary && (
            <>
              <span className="alu-view__op-symbol">
                {op === ALUOp.ADD ? "+" : op === ALUOp.SUB ? "−" : op.toUpperCase()}
              </span>
              <BitGroup bits={bBits} onChange={setBBits} label="B" />
            </>
          )}
          {isUnary && (
            <span className="alu-view__unary-note">{t("alu.unaryNote")}</span>
          )}
        </div>

        {/* Result display with flags */}
        <ResultDisplay result={result} />
      </section>
    </div>
  );
}
