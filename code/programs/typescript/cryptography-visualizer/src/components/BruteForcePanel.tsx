/**
 * # BruteForcePanel -- All 25 Caesar Decryptions
 *
 * The Caesar cipher has only 25 possible non-trivial keys (shifts 1 through 25).
 * This component displays every possible decryption, highlighting the one that
 * frequency analysis identifies as the most likely correct shift.
 *
 * ## Why Brute Force Works
 *
 * With only 25 possible keys, a computer can try all of them in microseconds.
 * This is the fundamental weakness of the Caesar cipher: its key space is tiny.
 * Modern ciphers like AES have key spaces of 2^128 or 2^256, making brute force
 * computationally infeasible.
 *
 * ## Visual Layout
 *
 * Each row shows:
 *   - The shift number (1-25)
 *   - The resulting plaintext if that shift were used to decrypt
 *
 * The row matching the frequency analysis best guess is highlighted with an
 * accent border so users can see which decryption looks like real English.
 *
 * @module BruteForcePanel
 */

interface BruteForcePanelProps {
  /** Array of all 25 brute-force decryptions. */
  results: Array<{ shift: number; plaintext: string }>;
  /** The shift that frequency analysis identifies as most likely correct. */
  bestShift: number;
}

/**
 * Renders a scrollable list of all 25 brute-force decryption results.
 *
 * The result matching `bestShift` is highlighted as the frequency analysis
 * best guess.
 */
export function BruteForcePanel({ results, bestShift }: BruteForcePanelProps) {
  return (
    <div className="brute-list" role="list" aria-label="brute force results">
      {results.map((result) => (
        <div
          key={result.shift}
          className={`brute-item${result.shift === bestShift ? " best" : ""}`}
          role="listitem"
        >
          <span className="brute-shift">
            Shift {result.shift}
            {result.shift === bestShift ? " *" : ""}
          </span>
          <span className="brute-text">{result.plaintext}</span>
        </div>
      ))}
    </div>
  );
}
