/**
 * BinaryAdders — Tab 1 container.
 *
 * Stacks three adder visualizations vertically, building from simple to
 * complex: half adder → full adder → ripple-carry adder.
 *
 * === Educational arc ===
 *
 * 1. Half adder: "Addition is just XOR + AND"
 * 2. Full adder: "Chain half adders to handle carry"
 * 3. Ripple-carry: "Chain full adders for multi-bit numbers"
 *
 * Each section builds on the previous one, showing how simple gates
 * compose into increasingly powerful circuits.
 */

import { useTranslation } from "@coding-adventures/ui-components";
import { HalfAdderDiagram } from "./HalfAdderDiagram.js";
import { FullAdderDiagram } from "./FullAdderDiagram.js";
import { RippleCarryDiagram } from "./RippleCarryDiagram.js";

export function BinaryAdders() {
  const { t } = useTranslation();

  return (
    <div className="adders">
      <p className="adders__intro">{t("adders.intro")}</p>
      <HalfAdderDiagram />
      <FullAdderDiagram />
      <RippleCarryDiagram />
    </div>
  );
}
