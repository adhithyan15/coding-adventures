/**
 * # English Letter Frequencies
 *
 * These are the standard frequencies of each letter (A-Z) in typical
 * English text. They are derived from large corpus analysis and are
 * widely used in cryptanalysis.
 *
 * ## Usage in Cryptanalysis
 *
 * - **Caesar cipher breaking:** Compare ciphertext frequencies against
 *   these expected frequencies using chi-squared to find the shift.
 * - **Vigenere cipher breaking:** Use index of coincidence to find key
 *   length, then chi-squared on each column to find each key letter.
 * - **Substitution cipher breaking:** Map the most common ciphertext
 *   letters to E, T, A, O, I, N, S, etc.
 *
 * ## Truth Table (sorted by frequency)
 *
 * | Letter | Frequency | Rank |
 * |--------|-----------|------|
 * | E      | 0.12702   | 1    |
 * | T      | 0.09056   | 2    |
 * | A      | 0.08167   | 3    |
 * | O      | 0.07507   | 4    |
 * | I      | 0.06966   | 5    |
 * | N      | 0.06749   | 6    |
 * | S      | 0.06327   | 7    |
 * | H      | 0.06094   | 8    |
 * | R      | 0.05987   | 9    |
 * | ...    | ...       | ...  |
 *
 * The mnemonic "ETAOIN SHRDLU" captures the most common letters in order.
 */
export const ENGLISH_FREQUENCIES: Record<string, number> = {
  A: 0.08167,
  B: 0.01492,
  C: 0.02782,
  D: 0.04253,
  E: 0.12702,
  F: 0.02228,
  G: 0.02015,
  H: 0.06094,
  I: 0.06966,
  J: 0.00153,
  K: 0.00772,
  L: 0.04025,
  M: 0.02406,
  N: 0.06749,
  O: 0.07507,
  P: 0.01929,
  Q: 0.00095,
  R: 0.05987,
  S: 0.06327,
  T: 0.09056,
  U: 0.02758,
  V: 0.00978,
  W: 0.02360,
  X: 0.00150,
  Y: 0.01974,
  Z: 0.00074,
};
