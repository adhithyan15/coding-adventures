/**
 * CombinationalLogic — Tab 3 container showing combinational circuit building blocks.
 *
 * === What are combinational circuits? ===
 *
 * Combinational circuits produce outputs that depend ONLY on the current
 * inputs — no memory, no state, no clock. They sit between primitive gates
 * and full arithmetic units in the digital design hierarchy:
 *
 *     Primitive gates (Tab 1)
 *         ↓
 *     Combinational circuits (THIS TAB)
 *         ↓
 *     Arithmetic circuits (half adder, full adder, ALU)
 *         ↓
 *     CPU, FPGA, memory controllers
 *
 * === Layout ===
 *
 * Three circuit visualizations stacked vertically:
 *
 *   1. 2:1 Multiplexer (MUX) — data selector/router
 *   2. 2-to-4 Decoder — binary to one-hot conversion
 *   3. Priority Encoder — highest-priority input wins
 */

import { useTranslation } from "@coding-adventures/ui-components";
import { MuxDiagram } from "./MuxDiagram.js";
import { DecoderDiagram } from "./DecoderDiagram.js";
import { EncoderDiagram } from "./EncoderDiagram.js";

export function CombinationalLogic() {
  const { t } = useTranslation();

  return (
    <section className="combinational">
      <div className="combinational__intro">
        <p>{t("comb.intro")}</p>
      </div>

      <div className="combinational__circuits">
        <MuxDiagram />
        <DecoderDiagram />
        <EncoderDiagram />
      </div>
    </section>
  );
}
