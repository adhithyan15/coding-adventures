/**
 * ResultDisplay — shows the ALU result as bits + decimal + hex + flags.
 *
 * Displays the 8-bit result value in three formats (binary, decimal, hex)
 * and the four condition flags (Zero, Carry, Negative, Overflow).
 */

import type { ALUResult } from "@coding-adventures/arithmetic";
import type { Bit } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";
import { FlagIndicator } from "../shared/FlagIndicator.js";

export interface ResultDisplayProps {
  result: ALUResult;
}

/** Convert LSB-first bit array to decimal. */
function bitsToDecimal(bits: Bit[]): number {
  return bits.reduce<number>((acc, bit, i) => acc + (bit << i), 0);
}

/** Convert decimal to hex string. */
function toHex(n: number, width: number): string {
  return "0x" + n.toString(16).toUpperCase().padStart(Math.ceil(width / 4), "0");
}

export function ResultDisplay({ result }: ResultDisplayProps) {
  const { t } = useTranslation();
  const decimal = bitsToDecimal(result.value);
  const hex = toHex(decimal, result.value.length);

  return (
    <div className="result-display" aria-live="polite">
      <h4 className="result-display__title">{t("alu.result")}</h4>

      {/* Bit cells (MSB first) */}
      <div className="result-display__bits">
        {[...result.value].reverse().map((bit, i) => (
          <span
            key={i}
            className={`result-display__bit ${bit ? "result-display__bit--high" : "result-display__bit--low"}`}
          >
            {bit}
          </span>
        ))}
      </div>

      {/* Decimal and hex */}
      <div className="result-display__values">
        <span className="result-display__decimal">= {decimal}</span>
        <span className="result-display__hex">{hex}</span>
      </div>

      {/* Condition flags */}
      <div className="result-display__flags">
        <FlagIndicator
          abbreviation="Z"
          name={t("alu.flag.zero")}
          active={result.zero}
          description={t("alu.flag.zeroDesc")}
        />
        <FlagIndicator
          abbreviation="C"
          name={t("alu.flag.carry")}
          active={result.carry}
          description={t("alu.flag.carryDesc")}
        />
        <FlagIndicator
          abbreviation="N"
          name={t("alu.flag.negative")}
          active={result.negative}
          description={t("alu.flag.negativeDesc")}
        />
        <FlagIndicator
          abbreviation="V"
          name={t("alu.flag.overflow")}
          active={result.overflow}
          description={t("alu.flag.overflowDesc")}
        />
      </div>
    </div>
  );
}
