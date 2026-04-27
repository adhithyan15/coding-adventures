/**
 * # App -- Cryptography Visualizer Main Component
 *
 * This is the top-level React component for the Cryptography Visualizer.
 * It orchestrates the full pipeline:
 *
 * 1. Accept user input (plaintext, cipher selection, shift amount)
 * 2. Compute the cipher mapping and encrypt the plaintext
 * 3. Render interactive panels showing each step of the process
 *
 * ## Supported Ciphers
 *
 * ### Caesar Cipher
 *
 * The oldest known substitution cipher. Each letter is shifted forward by a
 * fixed number of positions in the alphabet. With shift 3 (Caesar's own
 * choice), A becomes D, B becomes E, and so on. The shift wraps around: X
 * becomes A, Y becomes B, Z becomes C.
 *
 * ### Atbash Cipher
 *
 * An ancient Hebrew cipher that reverses the alphabet. A becomes Z, B becomes
 * Y, C becomes X, and so on. The mapping is its own inverse: encrypting twice
 * returns the original text. Atbash has no key -- the mapping is fixed.
 *
 * ## Architecture
 *
 * The Caesar cipher operations (encrypt, decrypt, brute force, frequency
 * analysis) come from the `@coding-adventures/caesar-cipher` package. The
 * Atbash cipher operations come from the
 * `@coding-adventures/atbash-cipher` package.
 *
 * @module App
 */

import { useEffect, useState } from "react";
import {
  encrypt as caesarEncrypt,
  bruteForce,
  frequencyAnalysis,
  ENGLISH_FREQUENCIES,
} from "@coding-adventures/caesar-cipher";
import { encrypt as atbashEncrypt } from "@coding-adventures/atbash-cipher";
import { SubstitutionTable } from "./components/SubstitutionTable.js";
import { StepByStep } from "./components/StepByStep.js";
import { FrequencyChart } from "./components/FrequencyChart.js";
import { BruteForcePanel } from "./components/BruteForcePanel.js";

// ─── Constants ────────────────────────────────────────────────────────────────

const DEFAULT_PLAINTEXT = "HELLO WORLD";
const DEFAULT_SHIFT = 3;
const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

// ─── Cipher Type ──────────────────────────────────────────────────────────────

type CipherType = "caesar" | "atbash";

// ─── Mapping Builders ─────────────────────────────────────────────────────────
//
// These functions build the full A-Z substitution mapping for the currently
// selected cipher. The mapping is used both by the SubstitutionTable component
// and by the step-by-step display.

/**
 * Builds the Caesar cipher substitution mapping for a given shift.
 *
 * @param shift - The number of positions to shift each letter forward.
 * @returns A record mapping each uppercase letter to its cipher equivalent.
 */
function buildCaesarMapping(shift: number): Record<string, string> {
  const mapping: Record<string, string> = {};
  for (let i = 0; i < 26; i++) {
    const plain = ALPHABET[i];
    const cipherIndex = ((i + shift) % 26 + 26) % 26;
    mapping[plain] = ALPHABET[cipherIndex];
  }
  return mapping;
}

/**
 * Builds the Atbash cipher substitution mapping.
 *
 * The mapping is always the same: A -> Z, B -> Y, C -> X, etc.
 *
 * @returns A record mapping each uppercase letter to its Atbash equivalent.
 */
function buildAtbashMapping(): Record<string, string> {
  const mapping: Record<string, string> = {};
  for (let i = 0; i < 26; i++) {
    mapping[ALPHABET[i]] = ALPHABET[25 - i];
  }
  return mapping;
}

// ─── Frequency Counter ────────────────────────────────────────────────────────

/**
 * Counts letter frequencies in a string and returns them as proportions.
 *
 * Only alphabetic characters are counted. The result maps each lowercase
 * letter to its frequency as a proportion of the total letter count.
 *
 * @param text - The text to analyze.
 * @returns A record mapping each lowercase letter to its frequency proportion.
 */
function computeFrequencies(text: string): Record<string, number> {
  const counts: Record<string, number> = {};
  let total = 0;

  for (const char of text.toLowerCase()) {
    if (char >= "a" && char <= "z") {
      counts[char] = (counts[char] ?? 0) + 1;
      total++;
    }
  }

  const frequencies: Record<string, number> = {};
  for (const letter of "abcdefghijklmnopqrstuvwxyz") {
    frequencies[letter] = total > 0 ? (counts[letter] ?? 0) / total : 0;
  }
  return frequencies;
}

// ─── App Component ────────────────────────────────────────────────────────────

export function App() {
  const [plaintext, setPlaintext] = useState(DEFAULT_PLAINTEXT);
  const [selectedCipher, setSelectedCipher] = useState<CipherType>("caesar");
  const [shift, setShift] = useState(DEFAULT_SHIFT);
  const [copyStatus, setCopyStatus] = useState<"idle" | "success" | "error">("idle");

  // ---- Compute the cipher output ----

  const ciphertext =
    selectedCipher === "caesar"
      ? caesarEncrypt(plaintext, shift)
      : atbashEncrypt(plaintext);

  const mapping =
    selectedCipher === "caesar"
      ? buildCaesarMapping(shift)
      : buildAtbashMapping();

  // Build the set of active letters (uppercase) from the plaintext.
  const activeLetters = new Set(
    plaintext
      .toUpperCase()
      .split("")
      .filter((ch) => ch >= "A" && ch <= "Z"),
  );

  // ---- Caesar-specific analysis ----

  const ciphertextFrequencies = computeFrequencies(ciphertext);
  const bruteForceResults = bruteForce(ciphertext);
  const freqAnalysis = frequencyAnalysis(ciphertext);

  useEffect(() => {
    if (copyStatus === "idle") {
      return undefined;
    }

    const timeoutId = window.setTimeout(() => {
      setCopyStatus("idle");
    }, 1800);

    return () => {
      window.clearTimeout(timeoutId);
    };
  }, [copyStatus]);

  async function handleCopyCiphertext(): Promise<void> {
    if (typeof navigator === "undefined" || navigator.clipboard?.writeText === undefined) {
      setCopyStatus("error");
      return;
    }

    try {
      await navigator.clipboard.writeText(ciphertext);
      setCopyStatus("success");
    } catch {
      setCopyStatus("error");
    }
  }

  return (
    <div className="app">
      <header className="hero">
        <div>
          <p className="eyebrow">Classical Cryptography Explorer</p>
          <h1>Cryptography Visualizer</h1>
          <p className="lede">
            Explore how classical substitution ciphers transform text character by character.
            Watch each letter shift through the alphabet, examine the substitution table,
            and see why these ancient ciphers are easily broken.
          </p>
        </div>

        <dl className="hero-metadata">
          <div>
            <dt>Ciphers</dt>
            <dd>Caesar, Atbash</dd>
          </div>
          <div>
            <dt>Analysis</dt>
            <dd>Frequency Analysis, Brute Force</dd>
          </div>
          <div>
            <dt>Pipeline</dt>
            <dd>Input &rarr; Mapping &rarr; Substitution &rarr; Output</dd>
          </div>
        </dl>
      </header>

      <main className="layout">
        {/* ── Controls Panel ──────────────────────────────────────────── */}
        <section className="panel controls-panel">
          <div className="panel-heading">
            <h2>Input</h2>
            <p>
              Enter any text to encrypt. Non-alphabetic characters (spaces, digits,
              punctuation) pass through unchanged. Letter case is preserved.
            </p>
          </div>

          <label className="field" htmlFor="plaintext-input">
            <span>Plaintext</span>
            <input
              id="plaintext-input"
              name="plaintext-input"
              type="text"
              value={plaintext}
              onChange={(event) => setPlaintext(event.target.value)}
              spellCheck={false}
              autoComplete="off"
            />
          </label>

          <div className="controls-row">
            <label className="field" htmlFor="cipher-select">
              <span>Cipher</span>
              <select
                id="cipher-select"
                name="cipher-select"
                value={selectedCipher}
                onChange={(event) => setSelectedCipher(event.target.value as CipherType)}
              >
                <option value="caesar">Caesar Cipher</option>
                <option value="atbash">Atbash Cipher</option>
              </select>
            </label>

            {selectedCipher === "caesar" && (
              <div className="slider-field">
                <span>Shift: {shift}</span>
                <div className="slider-row">
                  <input
                    type="range"
                    id="shift-slider"
                    name="shift-slider"
                    min={1}
                    max={25}
                    value={shift}
                    onChange={(event) => setShift(Number(event.target.value))}
                    aria-label="shift amount"
                  />
                  <span className="shift-value">{shift}</span>
                </div>
              </div>
            )}
          </div>

          {selectedCipher === "caesar" && (
            <div className="quick-actions">
              <button
                type="button"
                onClick={() => setShift(13)}
                aria-label="Apply ROT13"
              >
                ROT13 (shift 13)
              </button>
              <button
                type="button"
                onClick={() => setShift(3)}
                aria-label="Apply Caesar's original shift"
              >
                Caesar&apos;s Original (shift 3)
              </button>
            </div>
          )}
        </section>

        {/* ── Output Panel ────────────────────────────────────────────── */}
        <section className="panel output-panel">
          <div className="panel-heading">
            <h2>Ciphertext Output</h2>
            <p>
              The encrypted result after applying the {selectedCipher === "caesar" ? `Caesar cipher with shift ${shift}` : "Atbash cipher"}.
            </p>
          </div>
          <div className="output-actions">
            <button
              type="button"
              className="copy-button"
              onClick={() => void handleCopyCiphertext()}
              aria-label="Copy ciphertext"
            >
              Copy ciphertext
            </button>
            {copyStatus === "success" && (
              <span className="copy-feedback success" role="status">
                Ciphertext copied to clipboard.
              </span>
            )}
            {copyStatus === "error" && (
              <span className="copy-feedback error" role="status">
                Clipboard copy is unavailable in this browser.
              </span>
            )}
          </div>
          <div className="output-text" data-testid="ciphertext-output">
            {ciphertext || "\u00A0"}
          </div>
        </section>

        {/* ── Substitution Table ───────────────────────────────────────── */}
        <section className="panel">
          <div className="panel-heading">
            <h2>Substitution Table</h2>
            <p>
              The complete A&ndash;Z mapping for the current cipher settings.
              Highlighted columns show letters present in your plaintext.
            </p>
          </div>
          <SubstitutionTable mapping={mapping} activeLetters={activeLetters} />
        </section>

        {/* ── Step-by-Step ─────────────────────────────────────────────── */}
        <section className="panel">
          <div className="panel-heading">
            <h2>Step-by-Step</h2>
            <p>
              Each character transformation shown individually. Alphabetic characters
              are shifted; everything else passes through unchanged.
            </p>
          </div>
          <StepByStep
            plaintext={plaintext}
            ciphertext={ciphertext}
            cipher={selectedCipher}
            shift={shift}
          />
        </section>

        {/* ── Frequency Analysis (Caesar only) ─────────────────────────── */}
        {selectedCipher === "caesar" && (
          <section className="panel">
            <div className="panel-heading">
              <h2>Frequency Analysis</h2>
              <p>
                Comparing the letter frequency distribution of the ciphertext against
                expected English frequencies. The shift that best aligns these
                distributions is the most likely key.
              </p>
            </div>
            <FrequencyChart
              frequencies={ciphertextFrequencies}
              expectedFrequencies={ENGLISH_FREQUENCIES}
            />
            <div className="stats">
              <div>
                <dt>Detected Shift</dt>
                <dd>{freqAnalysis.shift}</dd>
              </div>
              <div>
                <dt>Recovered Plaintext</dt>
                <dd>{freqAnalysis.plaintext}</dd>
              </div>
              <div>
                <dt>Actual Shift</dt>
                <dd>{shift}</dd>
              </div>
            </div>
          </section>
        )}

        {/* ── Brute Force (Caesar only) ────────────────────────────────── */}
        {selectedCipher === "caesar" && (
          <section className="panel">
            <div className="panel-heading">
              <h2>Brute Force Attack</h2>
              <p>
                All 25 possible decryptions. The frequency analysis best guess is
                highlighted. With such a tiny key space, brute force is trivial.
              </p>
            </div>
            <BruteForcePanel
              results={bruteForceResults}
              bestShift={freqAnalysis.shift}
            />
          </section>
        )}
      </main>
    </div>
  );
}
