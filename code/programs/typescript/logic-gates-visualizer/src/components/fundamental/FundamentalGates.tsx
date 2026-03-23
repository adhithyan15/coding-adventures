/**
 * FundamentalGates — Tab 1 container showing the four basic logic gates.
 *
 * === The four fundamental gates ===
 *
 * Digital logic is built from a surprisingly small set of basic operations.
 * This tab presents the four most important gates:
 *
 *   NOT — inverts a single input (1 becomes 0, 0 becomes 1)
 *   AND — outputs 1 only when BOTH inputs are 1
 *   OR  — outputs 1 when EITHER input is 1
 *   XOR — outputs 1 when inputs DIFFER
 *
 * Every other logic function can be constructed from these four gates.
 * In fact, as the "NAND Universality" tab will show, you only need NAND
 * to build all of them — but these four are the ones humans think about
 * when designing circuits.
 *
 * === Layout ===
 *
 * The gates are arranged in a responsive 2x2 grid:
 *
 *   ┌─────────┐  ┌─────────┐
 *   │   NOT   │  │   AND   │
 *   └─────────┘  └─────────┘
 *   ┌─────────┐  ┌─────────┐
 *   │   OR    │  │   XOR   │
 *   └─────────┘  └─────────┘
 *
 * On mobile (< 768px), the grid collapses to a single column.
 */

import { NOT, AND, OR, XOR } from "@coding-adventures/logic-gates";
import type { Bit } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";
import { GateCard } from "./GateCard.js";

export function FundamentalGates() {
  const { t } = useTranslation();

  return (
    <section>
      {/* Educational intro paragraph */}
      <p className="fundamental-intro">{t("fundamental.intro")}</p>

      {/* 2x2 gate card grid */}
      <div className="gate-grid">
        <GateCard
          gateType="not"
          gateFn={(a: Bit) => NOT(a)}
          inputLabels={["A"]}
          i18nPrefix="gate.not"
        />
        <GateCard
          gateType="and"
          gateFn={(a: Bit, b: Bit) => AND(a, b)}
          inputLabels={["A", "B"]}
          i18nPrefix="gate.and"
        />
        <GateCard
          gateType="or"
          gateFn={(a: Bit, b: Bit) => OR(a, b)}
          inputLabels={["A", "B"]}
          i18nPrefix="gate.or"
        />
        <GateCard
          gateType="xor"
          gateFn={(a: Bit, b: Bit) => XOR(a, b)}
          inputLabels={["A", "B"]}
          i18nPrefix="gate.xor"
        />
      </div>
    </section>
  );
}
