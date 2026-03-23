/**
 * NandUniversality — Tab 2 container demonstrating NAND functional completeness.
 *
 * === The big idea ===
 *
 * NAND is "functionally complete" — meaning ANY Boolean function can be built
 * using NAND gates alone. This is one of the most important results in digital
 * logic, discovered by Charles Sanders Peirce in 1880 and independently by
 * Henry Sheffer in 1913 (hence the "Sheffer stroke" notation: A|B = NAND).
 *
 * === Why it matters for hardware ===
 *
 * In CMOS technology, a NAND gate requires only 4 transistors:
 *   - 2 PMOS in parallel (pull-up network)
 *   - 2 NMOS in series (pull-down network)
 *
 * Since NAND is both cheap AND universal, chip designers can (theoretically)
 * build entire processors using nothing but NAND gates. In practice, they
 * use a mix of gate types for optimization, but NAND remains the most
 * commonly used gate in modern chip design.
 *
 * === Layout ===
 *
 * This tab shows four derivations, stacked vertically:
 *
 *   1. NAND → NOT   (1 gate,  4T)  — simplest: tie both inputs together
 *   2. NAND → AND   (2 gates, 8T)  — NAND then invert
 *   3. NAND → OR    (3 gates, 12T) — De Morgan's Law in action
 *   4. NAND → XOR   (4 gates, 16T) — most complex basic derivation
 *
 * Each derivation is interactive: toggle inputs and watch signals propagate
 * through the NAND gate wiring.
 */

import { useTranslation } from "@coding-adventures/ui-components";
import { NandDerivation } from "./NandDerivation.js";

export function NandUniversality() {
  const { t } = useTranslation();

  return (
    <section className="nand-universality">
      {/* Intro: why NAND universality matters */}
      <div className="nand-universality__intro">
        <p>{t("nand.intro")}</p>
      </div>

      {/* NAND gate explanation card */}
      <div className="nand-universality__nand-card">
        <h3 className="nand-universality__nand-title">{t("nand.nandGate.title")}</h3>
        <p className="nand-universality__nand-description">{t("nand.nandGate.description")}</p>
        <div className="nand-universality__nand-stats">
          <span className="nand-universality__stat">4 {t("cmos.transistorCount")}</span>
          <span className="nand-universality__stat nand-universality__stat--natural">
            {t("cmos.naturalGate")}
          </span>
        </div>
      </div>

      {/* Four derivations */}
      <div className="nand-universality__derivations">
        <NandDerivation type="not" />
        <NandDerivation type="and" />
        <NandDerivation type="or" />
        <NandDerivation type="xor" />
      </div>

      {/* Engineering tradeoff note */}
      <div className="nand-universality__tradeoff">
        <h3 className="nand-universality__tradeoff-title">{t("nand.tradeoff.title")}</h3>
        <p>{t("nand.tradeoff.description")}</p>
      </div>
    </section>
  );
}
