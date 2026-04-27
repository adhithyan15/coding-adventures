/**
 * @coding-adventures/markov-chain — Public API
 * =============================================
 *
 * This module re-exports the `MarkovChain` class, which is the sole
 * public surface of this package.
 *
 * Usage:
 *
 *   import { MarkovChain } from "@coding-adventures/markov-chain";
 *
 *   const chain = new MarkovChain(2, 1.0);
 *   chain.trainString("abcabcabc");
 *   console.log(chain.generateString("ab", 9));  // "abcabcabc"
 */

export { MarkovChain } from "./markov-chain.js";
