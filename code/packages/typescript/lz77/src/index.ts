/**
 * @coding-adventures/lz77
 *
 * LZ77 lossless compression algorithm (Lempel & Ziv, 1977). Part of the CMP
 * compression series in the coding-adventures monorepo.
 *
 * @example
 * ```ts
 * import { compress, decompress, encode, decode, token } from "@coding-adventures/lz77";
 *
 * const data = new TextEncoder().encode("hello hello hello");
 * const compressed = compress(data);
 * const original = decompress(compressed);
 * ```
 */

export {
  type Token,
  token,
  encode,
  decode,
  compress,
  decompress,
  serialiseTokens,
  deserialiseTokens,
} from "./lz77.js";
