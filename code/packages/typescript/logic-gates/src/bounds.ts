/**
 * Shared resource bounds — a LEAF module (it imports nothing), so that
 * gates.ts, combinational.ts and sequential.ts can all depend on it without
 * forming an import cycle.  index.ts re-exports the public ones.
 */

/** Largest select/address width a decoder or demux accepts (2^16 outputs). */
export const MAX_SELECT_BITS = 16;

/** Largest input list muxN / encoder / priorityEncoder accept. */
export const MAX_INPUTS = 65536;

/** Largest width (flip-flop count) for register, shift register and counter;
 *  also the largest arity of andN / orN / xorN. */
export const MAX_WIDTH = 65536;

/** Largest counter width. */
export const MAX_COUNTER_WIDTH = 64;

/** Longest argument label validateBit will echo into an error message. */
export const MAX_NAME_LENGTH = 64;
