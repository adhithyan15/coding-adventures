/**
 * SequentialLogic — Tab 4 container demonstrating memory circuits.
 *
 * === The leap from combinational to sequential ===
 *
 * Combinational circuits (Tab 3) have no memory — their outputs depend
 * only on current inputs. Sequential circuits can REMEMBER. This is
 * the critical leap that makes computers possible.
 *
 * Memory arises from FEEDBACK: wiring a gate's output back into its own
 * input. This creates a stable loop that "latches" into a state and stays
 * there — even after the input that caused it is removed.
 *
 * === Layout ===
 *
 * Three circuit visualizations, building in complexity:
 *
 *   1. SR Latch — the simplest memory (2 cross-coupled NOR gates)
 *   2. D Flip-Flop — edge-triggered memory (master-slave design)
 *   3. 4-Bit Counter — register + incrementer (auto-step mode)
 */

import { useTranslation } from "@coding-adventures/ui-components";
import { SrLatchDiagram } from "./SrLatchDiagram.js";
import { DFlipFlopDiagram } from "./DFlipFlopDiagram.js";
import { CounterView } from "./CounterView.js";

export function SequentialLogic() {
  const { t } = useTranslation();

  return (
    <section className="sequential">
      <div className="sequential__intro">
        <p>{t("seq.intro")}</p>
      </div>

      <div className="sequential__circuits">
        <SrLatchDiagram />
        <DFlipFlopDiagram />
        <CounterView />
      </div>
    </section>
  );
}
