/**
 * EverythingIsAddition — Tab 2 container.
 *
 * Stacks two major visualizations showing how "different" arithmetic
 * operations are really just addition in disguise:
 *
 *   1. Subtraction = A + NOT(B) + 1 (two's complement)
 *   2. Multiplication = shift-and-add (conditional additions of shifted values)
 *
 * === Educational arc ===
 *
 * This is the central insight of computer arithmetic: the adder is the
 * only arithmetic circuit you truly need. Subtraction and multiplication
 * both reduce to sequences of additions. Division (not shown) also reduces
 * to repeated subtraction, which is itself addition.
 */

import { useTranslation } from "@coding-adventures/ui-components";
import { SubtractionView } from "./SubtractionView.js";
import { MultiplicationView } from "./MultiplicationView.js";

export function EverythingIsAddition() {
  const { t } = useTranslation();

  return (
    <div className="addition">
      <p className="addition__intro">{t("addition.intro")}</p>
      <SubtractionView />
      <MultiplicationView />
    </div>
  );
}
