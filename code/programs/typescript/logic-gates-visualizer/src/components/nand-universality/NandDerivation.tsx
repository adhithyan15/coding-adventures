/**
 * NandDerivation — interactive visualization of a gate built entirely from NAND gates.
 *
 * === Functional completeness ===
 *
 * A set of gates is "functionally complete" if any Boolean function can be
 * expressed using only gates from that set. NAND alone is functionally complete:
 * every other gate — NOT, AND, OR, XOR — can be constructed from NAND gates.
 *
 * This is a profound result. It means:
 *   1. You only need ONE type of gate to build ANY digital circuit.
 *   2. Since NAND costs only 4 transistors in CMOS, you can build entire
 *      processors from a single, cheap building block.
 *   3. In practice, chip designers use a mix of gates for efficiency, but
 *      NAND is the workhorse of digital design.
 *
 * === What this component shows ===
 *
 * For each derived gate, the component renders:
 *   1. A title and explanation of the construction
 *   2. An interactive SVG wiring diagram showing how NAND gates connect
 *   3. Toggle buttons for inputs — flip them and watch signals propagate
 *   4. Intermediate wire values labeled on the diagram
 *   5. A transistor cost comparison (NAND-only vs native implementation)
 *
 * === How signals flow ===
 *
 * Each NAND gate in the diagram receives inputs, computes NAND, and passes
 * the result to the next gate. We use the actual `NAND` function from the
 * logic-gates package, so the computed values are guaranteed correct.
 *
 * Wire colors follow the standard convention:
 *   - GREEN (#4caf50): HIGH (1)
 *   - GRAY  (#777):    LOW  (0)
 */

import { useState } from "react";
import type { Bit } from "@coding-adventures/logic-gates";
import { NAND } from "@coding-adventures/logic-gates";
import { useTranslation } from "@coding-adventures/ui-components";
import { BitToggle } from "../shared/BitToggle.js";
import { WireLabel } from "../shared/WireLabel.js";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type DerivationType = "not" | "and" | "or" | "xor";

export interface NandDerivationProps {
  /** Which gate derivation to show. */
  type: DerivationType;
}

// ---------------------------------------------------------------------------
// Helper: wire color
// ---------------------------------------------------------------------------

function wireColor(bit: Bit): string {
  return bit === 1 ? "#4caf50" : "#777";
}

// ---------------------------------------------------------------------------
// NAND→NOT: 1 NAND gate
//
//   a ---+---+
//        | D |---o--- output
//   a ---+---+
//
//   NAND(a, a) = NOT(a)
// ---------------------------------------------------------------------------

function NotDerivation() {
  const { t } = useTranslation();
  const [a, setA] = useState<Bit>(0);

  const output = NAND(a, a);

  return (
    <div className="nand-derivation">
      <div className="nand-derivation__header">
        <h3 className="nand-derivation__title">{t("nand.not.title")}</h3>
        <span className="nand-derivation__formula">{t("nand.not.formula")}</span>
      </div>

      <p className="nand-derivation__description">{t("nand.not.description")}</p>

      <div className="nand-derivation__diagram">
        <div className="nand-derivation__inputs">
          <BitToggle value={a} onChange={setA} label="A" />
        </div>

        <svg viewBox="0 0 220 80" className="nand-derivation__svg" role="img" aria-label={t("nand.not.ariaLabel")}>
          {/* Input wire splits into both NAND inputs */}
          <line x1="0" y1="40" x2="50" y2="40" stroke={wireColor(a)} strokeWidth="2" />
          <line x1="50" y1="40" x2="50" y2="25" stroke={wireColor(a)} strokeWidth="2" />
          <line x1="50" y1="40" x2="50" y2="55" stroke={wireColor(a)} strokeWidth="2" />
          <line x1="50" y1="25" x2="70" y2="25" stroke={wireColor(a)} strokeWidth="2" />
          <line x1="50" y1="55" x2="70" y2="55" stroke={wireColor(a)} strokeWidth="2" />

          {/* NAND gate box */}
          <rect x="70" y="15" width="60" height="50" rx="5" fill="rgba(233,69,96,0.1)" stroke="#e94560" strokeWidth="1.5" />
          <text x="100" y="44" textAnchor="middle" fill="#e94560" fontSize="13" fontWeight="700">NAND</text>

          {/* Output wire */}
          <line x1="130" y1="40" x2="200" y2="40" stroke={wireColor(output)} strokeWidth="2" />

          {/* Wire value labels */}
          <text x="30" y="55" textAnchor="middle" fill={wireColor(a)} fontSize="11" fontWeight="600">{a}</text>
          <text x="165" y="35" textAnchor="middle" fill={wireColor(output)} fontSize="11" fontWeight="600">{output}</text>
        </svg>

        <div className="nand-derivation__output">
          <WireLabel value={output} label="Out" />
        </div>
      </div>

      <div className="nand-derivation__cost">
        <span className="nand-derivation__cost-item nand-derivation__cost-item--nand">
          {t("nand.cost.nandOnly")}: 1 NAND = 4T
        </span>
        <span className="nand-derivation__cost-item nand-derivation__cost-item--native">
          {t("nand.cost.native")}: 2T
        </span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// NAND→AND: 2 NAND gates
//
//   a ---+---+
//        | D |---o--- wire1 ---+---+
//   b ---+---+                 | D |---o--- output
//                   wire1 ---+---+
//
//   AND(a,b) = NAND(NAND(a,b), NAND(a,b)) = NOT(NAND(a,b))
// ---------------------------------------------------------------------------

function AndDerivation() {
  const { t } = useTranslation();
  const [a, setA] = useState<Bit>(0);
  const [b, setB] = useState<Bit>(0);

  const wire1 = NAND(a, b);      // Gate 1
  const output = NAND(wire1, wire1); // Gate 2 (acts as NOT)

  return (
    <div className="nand-derivation">
      <div className="nand-derivation__header">
        <h3 className="nand-derivation__title">{t("nand.and.title")}</h3>
        <span className="nand-derivation__formula">{t("nand.and.formula")}</span>
      </div>

      <p className="nand-derivation__description">{t("nand.and.description")}</p>

      <div className="nand-derivation__diagram">
        <div className="nand-derivation__inputs">
          <BitToggle value={a} onChange={setA} label="A" />
          <BitToggle value={b} onChange={setB} label="B" />
        </div>

        <svg viewBox="0 0 340 100" className="nand-derivation__svg" role="img" aria-label={t("nand.and.ariaLabel")}>
          {/* Input wires to Gate 1 */}
          <line x1="0" y1="30" x2="60" y2="30" stroke={wireColor(a)} strokeWidth="2" />
          <line x1="0" y1="70" x2="60" y2="70" stroke={wireColor(b)} strokeWidth="2" />

          {/* Gate 1: NAND(a, b) */}
          <rect x="60" y="18" width="60" height="64" rx="5" fill="rgba(233,69,96,0.1)" stroke="#e94560" strokeWidth="1.5" />
          <text x="90" y="54" textAnchor="middle" fill="#e94560" fontSize="12" fontWeight="700">NAND</text>
          <text x="90" y="14" textAnchor="middle" fill="#888" fontSize="9">Gate 1</text>

          {/* Wire1: output of Gate 1 → both inputs of Gate 2 */}
          <line x1="120" y1="50" x2="170" y2="50" stroke={wireColor(wire1)} strokeWidth="2" />
          <line x1="170" y1="50" x2="170" y2="35" stroke={wireColor(wire1)} strokeWidth="2" />
          <line x1="170" y1="50" x2="170" y2="65" stroke={wireColor(wire1)} strokeWidth="2" />
          <line x1="170" y1="35" x2="190" y2="35" stroke={wireColor(wire1)} strokeWidth="2" />
          <line x1="170" y1="65" x2="190" y2="65" stroke={wireColor(wire1)} strokeWidth="2" />

          {/* Wire1 value label */}
          <text x="145" y="45" textAnchor="middle" fill={wireColor(wire1)} fontSize="10" fontWeight="600">{wire1}</text>

          {/* Gate 2: NAND(wire1, wire1) = NOT(wire1) */}
          <rect x="190" y="23" width="60" height="54" rx="5" fill="rgba(233,69,96,0.1)" stroke="#e94560" strokeWidth="1.5" />
          <text x="220" y="54" textAnchor="middle" fill="#e94560" fontSize="12" fontWeight="700">NAND</text>
          <text x="220" y="19" textAnchor="middle" fill="#888" fontSize="9">Gate 2</text>

          {/* Output wire */}
          <line x1="250" y1="50" x2="320" y2="50" stroke={wireColor(output)} strokeWidth="2" />

          {/* Output value label */}
          <text x="285" y="45" textAnchor="middle" fill={wireColor(output)} fontSize="10" fontWeight="600">{output}</text>
        </svg>

        <div className="nand-derivation__output">
          <WireLabel value={output} label="Out" />
        </div>
      </div>

      <div className="nand-derivation__cost">
        <span className="nand-derivation__cost-item nand-derivation__cost-item--nand">
          {t("nand.cost.nandOnly")}: 2 NAND = 8T
        </span>
        <span className="nand-derivation__cost-item nand-derivation__cost-item--native">
          {t("nand.cost.native")}: 6T
        </span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// NAND→OR: 3 NAND gates (De Morgan's Law)
//
//   a ---+---+
//        | D |---o--- notA ---+
//   a ---+---+                |---+
//                             |   | D |---o--- output
//   b ---+---+                |---+
//        | D |---o--- notB ---+
//   b ---+---+
//
//   OR(a,b) = NAND(NOT(a), NOT(b)) = NAND(NAND(a,a), NAND(b,b))
// ---------------------------------------------------------------------------

function OrDerivation() {
  const { t } = useTranslation();
  const [a, setA] = useState<Bit>(0);
  const [b, setB] = useState<Bit>(0);

  const notA = NAND(a, a);       // Gate 1: NOT(a)
  const notB = NAND(b, b);       // Gate 2: NOT(b)
  const output = NAND(notA, notB); // Gate 3: NAND(NOT(a), NOT(b))

  return (
    <div className="nand-derivation">
      <div className="nand-derivation__header">
        <h3 className="nand-derivation__title">{t("nand.or.title")}</h3>
        <span className="nand-derivation__formula">{t("nand.or.formula")}</span>
      </div>

      <p className="nand-derivation__description">{t("nand.or.description")}</p>

      <div className="nand-derivation__diagram">
        <div className="nand-derivation__inputs">
          <BitToggle value={a} onChange={setA} label="A" />
          <BitToggle value={b} onChange={setB} label="B" />
        </div>

        <svg viewBox="0 0 380 130" className="nand-derivation__svg" role="img" aria-label={t("nand.or.ariaLabel")}>
          {/* Input A splits to Gate 1 */}
          <line x1="0" y1="30" x2="30" y2="30" stroke={wireColor(a)} strokeWidth="2" />
          <line x1="30" y1="30" x2="30" y2="20" stroke={wireColor(a)} strokeWidth="2" />
          <line x1="30" y1="30" x2="30" y2="40" stroke={wireColor(a)} strokeWidth="2" />
          <line x1="30" y1="20" x2="50" y2="20" stroke={wireColor(a)} strokeWidth="2" />
          <line x1="30" y1="40" x2="50" y2="40" stroke={wireColor(a)} strokeWidth="2" />

          {/* Gate 1: NAND(a,a) = NOT(a) */}
          <rect x="50" y="10" width="55" height="40" rx="5" fill="rgba(233,69,96,0.1)" stroke="#e94560" strokeWidth="1.5" />
          <text x="78" y="34" textAnchor="middle" fill="#e94560" fontSize="11" fontWeight="700">NAND</text>
          <text x="78" y="7" textAnchor="middle" fill="#888" fontSize="9">Gate 1</text>

          {/* notA wire */}
          <line x1="105" y1="30" x2="200" y2="30" stroke={wireColor(notA)} strokeWidth="2" />
          <text x="150" y="24" textAnchor="middle" fill={wireColor(notA)} fontSize="10" fontWeight="600">¬A={notA}</text>

          {/* Input B splits to Gate 2 */}
          <line x1="0" y1="100" x2="30" y2="100" stroke={wireColor(b)} strokeWidth="2" />
          <line x1="30" y1="100" x2="30" y2="90" stroke={wireColor(b)} strokeWidth="2" />
          <line x1="30" y1="100" x2="30" y2="110" stroke={wireColor(b)} strokeWidth="2" />
          <line x1="30" y1="90" x2="50" y2="90" stroke={wireColor(b)} strokeWidth="2" />
          <line x1="30" y1="110" x2="50" y2="110" stroke={wireColor(b)} strokeWidth="2" />

          {/* Gate 2: NAND(b,b) = NOT(b) */}
          <rect x="50" y="80" width="55" height="40" rx="5" fill="rgba(233,69,96,0.1)" stroke="#e94560" strokeWidth="1.5" />
          <text x="78" y="104" textAnchor="middle" fill="#e94560" fontSize="11" fontWeight="700">NAND</text>
          <text x="78" y="77" textAnchor="middle" fill="#888" fontSize="9">Gate 2</text>

          {/* notB wire */}
          <line x1="105" y1="100" x2="200" y2="100" stroke={wireColor(notB)} strokeWidth="2" />
          <text x="150" y="94" textAnchor="middle" fill={wireColor(notB)} fontSize="10" fontWeight="600">¬B={notB}</text>

          {/* Gate 3: NAND(notA, notB) = OR(a,b) */}
          <line x1="200" y1="30" x2="220" y2="45" stroke={wireColor(notA)} strokeWidth="2" />
          <line x1="200" y1="100" x2="220" y2="85" stroke={wireColor(notB)} strokeWidth="2" />
          <rect x="220" y="33" width="60" height="64" rx="5" fill="rgba(233,69,96,0.1)" stroke="#e94560" strokeWidth="1.5" />
          <text x="250" y="69" textAnchor="middle" fill="#e94560" fontSize="12" fontWeight="700">NAND</text>
          <text x="250" y="29" textAnchor="middle" fill="#888" fontSize="9">Gate 3</text>

          {/* Output wire */}
          <line x1="280" y1="65" x2="360" y2="65" stroke={wireColor(output)} strokeWidth="2" />
          <text x="320" y="59" textAnchor="middle" fill={wireColor(output)} fontSize="10" fontWeight="600">{output}</text>
        </svg>

        <div className="nand-derivation__output">
          <WireLabel value={output} label="Out" />
        </div>
      </div>

      <div className="nand-derivation__cost">
        <span className="nand-derivation__cost-item nand-derivation__cost-item--nand">
          {t("nand.cost.nandOnly")}: 3 NAND = 12T
        </span>
        <span className="nand-derivation__cost-item nand-derivation__cost-item--native">
          {t("nand.cost.native")}: 6T
        </span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// NAND→XOR: 4 NAND gates
//
//   a ---+------------ Gate 2: NAND(a, N) ---+
//        |                                    |
//        +--- Gate 1: NAND(a,b) = N --+      Gate 4: NAND(w1, w2) --- output
//        |                            |      |
//   b ---+------------ Gate 3: NAND(b, N) ---+
//
//   XOR(a,b) = NAND(NAND(a, NAND(a,b)), NAND(b, NAND(a,b)))
// ---------------------------------------------------------------------------

function XorDerivation() {
  const { t } = useTranslation();
  const [a, setA] = useState<Bit>(0);
  const [b, setB] = useState<Bit>(0);

  const n = NAND(a, b);           // Gate 1: NAND(a,b)
  const w1 = NAND(a, n);          // Gate 2: NAND(a, N)
  const w2 = NAND(b, n);          // Gate 3: NAND(b, N)
  const output = NAND(w1, w2);    // Gate 4: NAND(w1, w2)

  return (
    <div className="nand-derivation">
      <div className="nand-derivation__header">
        <h3 className="nand-derivation__title">{t("nand.xor.title")}</h3>
        <span className="nand-derivation__formula">{t("nand.xor.formula")}</span>
      </div>

      <p className="nand-derivation__description">{t("nand.xor.description")}</p>

      <div className="nand-derivation__diagram">
        <div className="nand-derivation__inputs">
          <BitToggle value={a} onChange={setA} label="A" />
          <BitToggle value={b} onChange={setB} label="B" />
        </div>

        <svg viewBox="0 0 440 160" className="nand-derivation__svg" role="img" aria-label={t("nand.xor.ariaLabel")}>
          {/* Input A main wire */}
          <line x1="0" y1="40" x2="40" y2="40" stroke={wireColor(a)} strokeWidth="2" />
          {/* Input B main wire */}
          <line x1="0" y1="120" x2="40" y2="120" stroke={wireColor(b)} strokeWidth="2" />

          {/* A branches: to Gate 1 and Gate 2 */}
          <line x1="40" y1="40" x2="40" y2="75" stroke={wireColor(a)} strokeWidth="2" />
          <line x1="40" y1="75" x2="60" y2="75" stroke={wireColor(a)} strokeWidth="2" />
          <line x1="40" y1="40" x2="140" y2="40" stroke={wireColor(a)} strokeWidth="2" />
          <circle cx="40" cy="40" r="3" fill={wireColor(a)} />

          {/* B branches: to Gate 1 and Gate 3 */}
          <line x1="40" y1="120" x2="40" y2="90" stroke={wireColor(b)} strokeWidth="2" />
          <line x1="40" y1="90" x2="60" y2="90" stroke={wireColor(b)} strokeWidth="2" />
          <line x1="40" y1="120" x2="140" y2="120" stroke={wireColor(b)} strokeWidth="2" />
          <circle cx="40" cy="120" r="3" fill={wireColor(b)} />

          {/* Gate 1: NAND(a, b) = N (center) */}
          <rect x="60" y="63" width="55" height="38" rx="5" fill="rgba(233,69,96,0.1)" stroke="#e94560" strokeWidth="1.5" />
          <text x="88" y="86" textAnchor="middle" fill="#e94560" fontSize="11" fontWeight="700">NAND</text>
          <text x="88" y="60" textAnchor="middle" fill="#888" fontSize="9">Gate 1</text>

          {/* N wire: Gate 1 output → Gate 2 and Gate 3 */}
          <line x1="115" y1="82" x2="130" y2="82" stroke={wireColor(n)} strokeWidth="2" />
          <line x1="130" y1="82" x2="130" y2="50" stroke={wireColor(n)} strokeWidth="2" />
          <line x1="130" y1="82" x2="130" y2="112" stroke={wireColor(n)} strokeWidth="2" />
          <line x1="130" y1="50" x2="140" y2="50" stroke={wireColor(n)} strokeWidth="2" />
          <line x1="130" y1="112" x2="140" y2="112" stroke={wireColor(n)} strokeWidth="2" />
          <circle cx="130" cy="82" r="3" fill={wireColor(n)} />

          {/* N value label */}
          <text x="122" y="76" textAnchor="middle" fill={wireColor(n)} fontSize="9" fontWeight="600">N={n}</text>

          {/* Gate 2: NAND(a, N) (top) */}
          <rect x="140" y="28" width="55" height="38" rx="5" fill="rgba(233,69,96,0.1)" stroke="#e94560" strokeWidth="1.5" />
          <text x="168" y="51" textAnchor="middle" fill="#e94560" fontSize="11" fontWeight="700">NAND</text>
          <text x="168" y="25" textAnchor="middle" fill="#888" fontSize="9">Gate 2</text>

          {/* Gate 3: NAND(b, N) (bottom) */}
          <rect x="140" y="100" width="55" height="38" rx="5" fill="rgba(233,69,96,0.1)" stroke="#e94560" strokeWidth="1.5" />
          <text x="168" y="123" textAnchor="middle" fill="#e94560" fontSize="11" fontWeight="700">NAND</text>
          <text x="168" y="97" textAnchor="middle" fill="#888" fontSize="9">Gate 3</text>

          {/* w1 wire: Gate 2 → Gate 4 */}
          <line x1="195" y1="47" x2="260" y2="47" stroke={wireColor(w1)} strokeWidth="2" />
          <line x1="260" y1="47" x2="280" y2="68" stroke={wireColor(w1)} strokeWidth="2" />
          <text x="228" y="42" textAnchor="middle" fill={wireColor(w1)} fontSize="9" fontWeight="600">w1={w1}</text>

          {/* w2 wire: Gate 3 → Gate 4 */}
          <line x1="195" y1="119" x2="260" y2="119" stroke={wireColor(w2)} strokeWidth="2" />
          <line x1="260" y1="119" x2="280" y2="95" stroke={wireColor(w2)} strokeWidth="2" />
          <text x="228" y="114" textAnchor="middle" fill={wireColor(w2)} fontSize="9" fontWeight="600">w2={w2}</text>

          {/* Gate 4: NAND(w1, w2) = XOR output */}
          <rect x="280" y="55" width="60" height="54" rx="5" fill="rgba(233,69,96,0.1)" stroke="#e94560" strokeWidth="1.5" />
          <text x="310" y="86" textAnchor="middle" fill="#e94560" fontSize="12" fontWeight="700">NAND</text>
          <text x="310" y="52" textAnchor="middle" fill="#888" fontSize="9">Gate 4</text>

          {/* Output wire */}
          <line x1="340" y1="82" x2="420" y2="82" stroke={wireColor(output)} strokeWidth="2" />
          <text x="380" y="76" textAnchor="middle" fill={wireColor(output)} fontSize="10" fontWeight="600">{output}</text>
        </svg>

        <div className="nand-derivation__output">
          <WireLabel value={output} label="Out" />
        </div>
      </div>

      <div className="nand-derivation__cost">
        <span className="nand-derivation__cost-item nand-derivation__cost-item--nand">
          {t("nand.cost.nandOnly")}: 4 NAND = 16T
        </span>
        <span className="nand-derivation__cost-item nand-derivation__cost-item--native">
          {t("nand.cost.native")}: 8-12T
        </span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Public component: dispatches to the correct derivation diagram
// ---------------------------------------------------------------------------

export function NandDerivation({ type }: NandDerivationProps) {
  switch (type) {
    case "not":
      return <NotDerivation />;
    case "and":
      return <AndDerivation />;
    case "or":
      return <OrDerivation />;
    case "xor":
      return <XorDerivation />;
  }
}
