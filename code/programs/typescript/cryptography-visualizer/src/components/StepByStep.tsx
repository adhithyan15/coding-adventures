/**
 * # StepByStep -- Character-by-Character Transformation Display
 *
 * This component shows how each individual character in the plaintext gets
 * transformed into its ciphertext equivalent. It provides the educational
 * "show your work" view that makes the cipher algorithm tangible.
 *
 * ## What Each Card Shows
 *
 * For alphabetic characters, the card displays the full transformation chain:
 *   - The original character and its position in the alphabet (0-25)
 *   - The operation being applied (e.g., "shift +3" or "mirror 25-pos")
 *   - The resulting cipher character and its position
 *
 * For non-alphabetic characters (spaces, digits, punctuation), the card
 * shows that the character passes through unchanged.
 *
 * ## Worked Example (Caesar, shift 3)
 *
 *   H (7)  -> shift +3  -> K (10)
 *   E (4)  -> shift +3  -> H (7)
 *   L (11) -> shift +3  -> O (14)
 *   L (11) -> shift +3  -> O (14)
 *   O (14) -> shift +3  -> R (17)
 *   !      -> (unchanged)
 *
 * @module StepByStep
 */

interface StepByStepProps {
  /** The original plaintext string. */
  plaintext: string;
  /** The encrypted ciphertext string (same length as plaintext). */
  ciphertext: string;
  /** Which cipher is active: "caesar" or "atbash". */
  cipher: string;
  /** The shift amount (only relevant for Caesar cipher). */
  shift?: number;
}

/**
 * Returns the 0-based alphabet position for an uppercase letter, or -1 if
 * the character is not a letter.
 */
function letterPosition(char: string): number {
  const code = char.toUpperCase().charCodeAt(0);
  if (code >= 65 && code <= 90) {
    return code - 65;
  }
  return -1;
}

/**
 * Renders a grid of cards showing each character's transformation step.
 */
export function StepByStep({ plaintext, ciphertext, cipher, shift }: StepByStepProps) {
  const steps = plaintext.split("").map((plainChar, index) => {
    const cipherChar = ciphertext[index] ?? plainChar;
    const pos = letterPosition(plainChar);

    if (pos === -1) {
      // Non-alphabetic character: passes through unchanged.
      return { plainChar, cipherChar, isAlpha: false, detail: "(unchanged)", pos: -1, resultPos: -1 };
    }

    const resultPos = letterPosition(cipherChar);
    let detail: string;

    if (cipher === "caesar") {
      detail = `shift +${shift ?? 0}`;
    } else {
      // Atbash: mirror the position. The formula is 25 - pos.
      detail = `mirror (25 - ${pos})`;
    }

    return { plainChar, cipherChar, isAlpha: true, detail, pos, resultPos };
  });

  return (
    <div className="step-grid">
      {steps.map((step, index) => (
        <div
          key={`${step.plainChar}-${index}`}
          className={`step-card${step.isAlpha ? "" : " unchanged"}`}
        >
          <span className="step-char">{step.plainChar}</span>
          {step.isAlpha ? (
            <>
              <span className="step-detail">
                {step.plainChar.toUpperCase()} ({step.pos}) &rarr; {step.detail}
              </span>
              <span className="step-result">
                {step.cipherChar} ({step.resultPos})
              </span>
            </>
          ) : (
            <span className="step-detail">{step.detail}</span>
          )}
        </div>
      ))}
    </div>
  );
}
