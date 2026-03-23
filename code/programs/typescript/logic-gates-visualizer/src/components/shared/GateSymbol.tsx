/**
 * GateSymbol — IEEE/IEC standard logic gate symbol SVGs.
 *
 * === Gate shape conventions ===
 *
 * Logic gates have standardized graphical symbols defined by IEEE Std 91-1984.
 * Each shape immediately tells an engineer what Boolean operation the gate performs:
 *
 *   NOT:  Triangle pointing right with a small circle (bubble) at the tip.
 *         The bubble means "inversion." A NOT gate is the simplest gate.
 *
 *   AND:  Flat left side, curved right side (D-shape). Think of the "D"
 *         as standing for "both must be true."
 *
 *   OR:   Curved left side, pointed right side. The curved input side
 *         suggests "either one flows through."
 *
 *   XOR:  Same as OR but with an extra curve on the input side. The double
 *         curve visually distinguishes it from plain OR.
 *
 *   NAND: AND shape + bubble at output. "AND then invert."
 *   NOR:  OR shape + bubble at output. "OR then invert."
 *
 * === Color coding ===
 *
 * Input and output wires are color-coded by signal level:
 *   - HIGH (1): green (#4caf50)
 *   - LOW  (0): gray (#777)
 *
 * This matches the BitToggle and WireLabel color conventions.
 */

import type { Bit } from "@coding-adventures/logic-gates";

export type GateType = "not" | "and" | "or" | "xor" | "nand" | "nor";

export interface GateSymbolProps {
  /** Which gate shape to render. */
  type: GateType;
  /** Value on input A (shown on top input wire). */
  inputA?: Bit;
  /** Value on input B (shown on bottom input wire). Only used for 2-input gates. */
  inputB?: Bit;
  /** Value on the output wire. */
  output?: Bit;
  /** SVG width in pixels. Defaults to 80. */
  width?: number;
  /** SVG height in pixels. Defaults to 60. */
  height?: number;
}

/** Map a bit value to a wire color. */
function wireColor(bit?: Bit): string {
  if (bit === 1) return "#4caf50";
  return "#777";
}

/**
 * Renders the NOT gate: triangle + bubble.
 *
 * Circuit shape:
 *     |\
 *  ---|  >o---
 *     |/
 */
function NotGate({ inputA, output }: { inputA?: Bit; output?: Bit }) {
  return (
    <>
      {/* Input wire */}
      <line x1="0" y1="30" x2="20" y2="30" stroke={wireColor(inputA)} strokeWidth="2" />
      {/* Triangle body */}
      <polygon points="20,10 20,50 55,30" fill="none" stroke="#ccc" strokeWidth="2" />
      {/* Inversion bubble */}
      <circle cx="59" cy="30" r="4" fill="none" stroke="#ccc" strokeWidth="2" />
      {/* Output wire */}
      <line x1="63" y1="30" x2="80" y2="30" stroke={wireColor(output)} strokeWidth="2" />
    </>
  );
}

/**
 * Renders the AND gate body: flat left, curved right (D-shape).
 *
 * Circuit shape:
 *     +---\
 *  ---|    )---
 *  ---|    )
 *     +---/
 */
function AndBody() {
  return (
    <path
      d="M20,10 L20,50 L40,50 A20,20 0 0,0 40,10 Z"
      fill="none"
      stroke="#ccc"
      strokeWidth="2"
    />
  );
}

/**
 * Renders the OR gate body: curved left, pointed right.
 *
 * The input side has a concave curve, and the output side
 * comes to a point — like a shield or arrowhead.
 */
function OrBody() {
  return (
    <path
      d="M20,10 Q30,30 20,50 Q40,50 55,30 Q40,10 20,10 Z"
      fill="none"
      stroke="#ccc"
      strokeWidth="2"
    />
  );
}

/**
 * Renders the XOR gate body: OR body + extra input curve.
 *
 * The extra curve on the input side is what distinguishes XOR from OR.
 * It's drawn as a second concave arc slightly to the left of the main body.
 */
function XorBody() {
  return (
    <>
      {/* Extra input curve (the XOR distinguishing mark) */}
      <path
        d="M16,10 Q26,30 16,50"
        fill="none"
        stroke="#ccc"
        strokeWidth="2"
      />
      {/* Main OR body */}
      <path
        d="M20,10 Q30,30 20,50 Q40,50 55,30 Q40,10 20,10 Z"
        fill="none"
        stroke="#ccc"
        strokeWidth="2"
      />
    </>
  );
}

/**
 * Standard 2-input gate wires: two input lines on the left, one output on the right.
 */
function TwoInputWires({
  inputA,
  inputB,
  output,
  outputX,
}: {
  inputA?: Bit;
  inputB?: Bit;
  output?: Bit;
  outputX: number;
}) {
  return (
    <>
      {/* Input A (top) */}
      <line x1="0" y1="20" x2="20" y2="20" stroke={wireColor(inputA)} strokeWidth="2" />
      {/* Input B (bottom) */}
      <line x1="0" y1="40" x2="20" y2="40" stroke={wireColor(inputB)} strokeWidth="2" />
      {/* Output */}
      <line x1={outputX} y1="30" x2="80" y2="30" stroke={wireColor(output)} strokeWidth="2" />
    </>
  );
}

export function GateSymbol({
  type,
  inputA,
  inputB,
  output,
  width = 80,
  height = 60,
}: GateSymbolProps) {
  // Build a human-readable label for screen readers.
  const gateLabel = `${type.toUpperCase()} gate symbol`;

  return (
    <svg
      viewBox="0 0 80 60"
      width={width}
      height={height}
      role="img"
      aria-label={gateLabel}
    >
      {type === "not" && <NotGate inputA={inputA} output={output} />}

      {type === "and" && (
        <>
          <TwoInputWires inputA={inputA} inputB={inputB} output={output} outputX={60} />
          <AndBody />
        </>
      )}

      {type === "or" && (
        <>
          <TwoInputWires inputA={inputA} inputB={inputB} output={output} outputX={55} />
          <OrBody />
        </>
      )}

      {type === "xor" && (
        <>
          <TwoInputWires inputA={inputA} inputB={inputB} output={output} outputX={55} />
          <XorBody />
        </>
      )}

      {type === "nand" && (
        <>
          <TwoInputWires inputA={inputA} inputB={inputB} output={output} outputX={67} />
          <AndBody />
          {/* Inversion bubble */}
          <circle cx="64" cy="30" r="4" fill="none" stroke="#ccc" strokeWidth="2" />
        </>
      )}

      {type === "nor" && (
        <>
          <TwoInputWires inputA={inputA} inputB={inputB} output={output} outputX={63} />
          <OrBody />
          {/* Inversion bubble */}
          <circle cx="59" cy="30" r="4" fill="none" stroke="#ccc" strokeWidth="2" />
        </>
      )}
    </svg>
  );
}
