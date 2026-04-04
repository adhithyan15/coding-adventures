/**
 * # SubstitutionTable -- Visual Cipher Mapping Grid
 *
 * This component renders a 26-column grid showing how each letter of the
 * alphabet maps to its cipher equivalent under the current cipher settings.
 *
 * ## How It Works
 *
 * Each cell in the grid shows:
 *   - The original "plain" letter (A through Z) on top
 *   - A small arrow in the middle
 *   - The corresponding "cipher" letter on the bottom
 *
 * Letters that appear in the current plaintext input are highlighted with
 * an "active" style so users can trace individual characters through the
 * substitution process.
 *
 * ## Example
 *
 * For a Caesar cipher with shift 3:
 *
 *   A -> D    B -> E    C -> F    ...    X -> A    Y -> B    Z -> C
 *
 * If the plaintext is "HELLO", then H, E, L, and O cells would be highlighted.
 *
 * @module SubstitutionTable
 */

interface SubstitutionTableProps {
  /** A mapping from each uppercase letter to its cipher equivalent. */
  mapping: Record<string, string>;
  /** The set of uppercase letters that appear in the current plaintext. */
  activeLetters: Set<string>;
}

/**
 * Renders a 26-column substitution table showing the full alphabet mapping.
 *
 * Each column shows a plain letter, an arrow, and the corresponding cipher
 * letter. Columns whose plain letter appears in the current plaintext are
 * highlighted to help users trace the transformation.
 */
export function SubstitutionTable({ mapping, activeLetters }: SubstitutionTableProps) {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("");

  return (
    <div className="substitution-grid" role="table" aria-label="substitution table">
      {alphabet.map((letter) => {
        const isActive = activeLetters.has(letter);
        const cipherLetter = mapping[letter] ?? letter;

        return (
          <div
            key={letter}
            className={`sub-cell${isActive ? " active" : ""}`}
            role="cell"
          >
            <span className="plain-letter">{letter}</span>
            <span className="arrow" aria-hidden="true">&#8595;</span>
            <span className="cipher-letter">{cipherLetter}</span>
          </div>
        );
      })}
    </div>
  );
}
