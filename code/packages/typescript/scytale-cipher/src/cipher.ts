/**
 * cipher.ts — Core Scytale cipher implementation.
 *
 * The Scytale Cipher
 * ==================
 *
 * The Scytale cipher is a *transposition* cipher from ancient Sparta (~700 BCE).
 * Unlike substitution ciphers (Caesar, Atbash) which replace each character,
 * the Scytale rearranges the order of characters without changing them.
 *
 * How Encryption Works
 * --------------------
 *
 * Given plaintext and a key (number of columns):
 *
 * 1. Write text row-by-row into a grid with `key` columns.
 * 2. Pad the last row with spaces if needed.
 * 3. Read the grid column-by-column to produce ciphertext.
 *
 * Example: encrypt("HELLO WORLD", 3)
 *
 *     Grid (4 rows x 3 cols):
 *         H E L
 *         L O ' '
 *         W O R
 *         L D ' '
 *
 *     Columns: HLWL + EOOD + L R  = "HLWLEOODL R "
 *
 * How Decryption Works
 * --------------------
 *
 * 1. Calculate rows = ceil(len / key).
 * 2. Write ciphertext column-by-column into the grid.
 * 3. Read row-by-row and strip trailing padding spaces.
 *
 * Why It's Insecure
 * -----------------
 *
 * The key space is tiny: for a message of length n, there are only
 * about n/2 possible keys. An attacker can try every key in milliseconds.
 */

/**
 * Encrypt text using the Scytale transposition cipher.
 *
 * @param text - The plaintext string to encrypt.
 * @param key - The number of columns (>= 2, <= text length).
 * @returns The transposed ciphertext string.
 * @throws {Error} If key is out of the valid range.
 */
export function encrypt(text: string, key: number): string {
  if (text === "") return "";
  if (key < 2) throw new Error(`Key must be >= 2, got ${key}`);
  if (key > text.length)
    throw new Error(`Key must be <= text length (${text.length}), got ${key}`);

  // Calculate grid dimensions and pad
  const numRows = Math.ceil(text.length / key);
  const padded = text.padEnd(numRows * key, " ");

  // Read column-by-column
  let result = "";
  for (let col = 0; col < key; col++) {
    for (let row = 0; row < numRows; row++) {
      result += padded[row * key + col];
    }
  }

  return result;
}

/**
 * Decrypt ciphertext that was encrypted with the Scytale cipher.
 *
 * @param text - The ciphertext string to decrypt.
 * @param key - The number of columns used during encryption.
 * @returns The decrypted plaintext (trailing padding stripped).
 * @throws {Error} If key is out of the valid range.
 */
export function decrypt(text: string, key: number): string {
  if (text === "") return "";
  if (key < 2) throw new Error(`Key must be >= 2, got ${key}`);
  if (key > text.length)
    throw new Error(`Key must be <= text length (${text.length}), got ${key}`);

  const n = text.length;
  const numRows = Math.ceil(n / key);

  // Handle uneven grids (when n % key != 0, e.g. during brute-force)
  const fullCols = n % key === 0 ? key : n % key;

  // Compute column start indices and lengths
  const colStarts: number[] = [];
  const colLens: number[] = [];
  let offset = 0;
  for (let c = 0; c < key; c++) {
    colStarts.push(offset);
    const len = n % key === 0 || c < fullCols ? numRows : numRows - 1;
    colLens.push(len);
    offset += len;
  }

  // Read row-by-row
  let result = "";
  for (let row = 0; row < numRows; row++) {
    for (let col = 0; col < key; col++) {
      if (row < colLens[col]) {
        result += text[colStarts[col] + row];
      }
    }
  }

  // Strip trailing padding spaces
  return result.trimEnd();
}

/** A single brute-force decryption result. */
export interface BruteForceResult {
  key: number;
  text: string;
}

/**
 * Try all possible Scytale keys and return the decrypted results.
 *
 * @param text - The ciphertext to brute-force.
 * @returns Array of {key, text} results for keys 2 through floor(len/2).
 */
export function bruteForce(text: string): BruteForceResult[] {
  if (text.length < 4) return [];

  const maxKey = Math.floor(text.length / 2);
  const results: BruteForceResult[] = [];

  for (let candidateKey = 2; candidateKey <= maxKey; candidateKey++) {
    results.push({
      key: candidateKey,
      text: decrypt(text, candidateKey),
    });
  }

  return results;
}
