/**
 * # broadcasting.ts — NumPy-style broadcasting for Tensor
 *
 * **NumPy broadcasting rules** (which PyTorch, JAX, TF all follow):
 *
 *   1. Right-align the two shapes.  Pad the shorter one on the LEFT
 *      with 1s until both have the same rank.
 *   2. For each dim, the sizes must EITHER be equal, OR one must be 1
 *      (which then "stretches" to match the other), OR one must be
 *      missing (handled by step 1 padding).
 *   3. The broadcast output shape is the max along each dim.
 *
 * Examples:
 *
 *   (3,)        + (2, 3)     → (2, 3)        # (3,) → (1, 3) → stretched on axis 0
 *   (5, 1, 3)   + (2, 3)     → (5, 2, 3)     # (2, 3) → (1, 2, 3); axis 1 stretches
 *   (1, 4)      + (3, 4)     → (3, 4)        # axis 0 stretches
 *   (2, 3)      + (3, 2)     → ERROR          # dim 0 mismatch (2 vs 3)
 *
 * ## What lives here
 *
 * - `broadcastShapes(a, b)` — pure shape math; returns the broadcast
 *   shape or throws.
 * - `broadcastDataTo(data, fromShape, toShape)` — given a Float32Array
 *   in `fromShape` layout, materialize a Float32Array in `toShape`
 *   layout.  Used by `BroadcastOp.forward`.
 * - `unbroadcastDataTo(data, fromShape, toShape)` — the INVERSE for
 *   backward: given a gradient in `fromShape` layout, sum it back down
 *   to `toShape`.  Used by `BroadcastOp.backward`.
 *
 * ## Why these are pure helpers (not Tensor methods)
 *
 * Broadcasting is graph-aware in the forward path — a `BroadcastOp`
 * autograd Function uses these helpers and attaches its own backward.
 * Keeping the helpers pure (input: typed arrays + shape arrays;
 * output: typed arrays) means the dispatch logic in `ops.ts` can call
 * them without paying a Tensor allocation cost when it just needs the
 * raw bytes.
 */

/**
 * Compute the broadcast shape for two input shapes.  Returns the broadcast
 * shape on success, throws RangeError on incompatibility.
 *
 * Implements steps 1-3 from the file header.
 */
export function broadcastShapes(
  a: readonly number[],
  b: readonly number[],
): number[] {
  // Step 1: right-align by walking from the end.  We don't actually need
  // to create padded arrays — we can just index `a[a.length - 1 - i]`
  // and treat missing entries as 1.
  const ndim = Math.max(a.length, b.length);
  const out = new Array<number>(ndim);

  for (let i = 0; i < ndim; i++) {
    const aDim = i < a.length ? a[a.length - 1 - i]! : 1;
    const bDim = i < b.length ? b[b.length - 1 - i]! : 1;

    // Step 2: each dim must match exactly OR one must be 1.
    if (aDim !== bDim && aDim !== 1 && bDim !== 1) {
      throw new RangeError(
        `cannot broadcast shapes [${a.join(", ")}] and [${b.join(", ")}]: ` +
          `dim ${ndim - 1 - i} (size ${aDim} vs ${bDim}) is neither equal nor 1`,
      );
    }

    // Step 3: output dim is the max (the non-1 one wins).
    out[ndim - 1 - i] = Math.max(aDim, bDim);
  }
  return out;
}

/**
 * Materialize a broadcasted view as a fresh Float32Array.
 *
 * Given input data laid out in `fromShape`, produce a new array laid out
 * in `toShape` where any dim that was 1 (or implicitly 1 via left-padding)
 * gets "stretched" by repeating along that axis.
 *
 * ## Implementation: strided iteration
 *
 * The trick is computing the source index for each destination index.
 * For each output position `(i_0, i_1, ..., i_{D-1})`:
 *
 *   - Pad the source shape on the left with 1s to match `toShape`.
 *   - For each axis where source dim is 1, FORCE the source index to 0
 *     (that's the "stretch" — every output position along that axis
 *     reads from the same source row).
 *   - For other axes, use the destination index directly.
 *   - Convert the source multi-index to a flat offset using the source
 *     strides (computed from `fromShape`, not `toShape`).
 *
 * This is O(numel(toShape)) — one source read per output cell.  No
 * intermediate allocation other than the output buffer.
 *
 * @throws RangeError if `fromShape` cannot broadcast to `toShape`.
 */
export function broadcastDataTo(
  data: Float32Array,
  fromShape: readonly number[],
  toShape: readonly number[],
): Float32Array {
  // Fast path: shapes already match → just copy.
  if (shapesEqual(fromShape, toShape)) {
    const out = new Float32Array(data.length);
    out.set(data);
    return out;
  }

  // Validate that this broadcast is actually possible.  We call
  // broadcastShapes for its side-effect (the error message); the
  // computed shape MUST equal toShape if valid.
  const computed = broadcastShapes(fromShape, toShape);
  if (!shapesEqual(computed, toShape)) {
    throw new RangeError(
      `broadcastDataTo: [${fromShape.join(", ")}] broadcasts to [${computed.join(", ")}], ` +
        `not the requested [${toShape.join(", ")}]`,
    );
  }

  // Left-pad fromShape with 1s to match toShape's rank.
  const padded = leftPadShape(fromShape, toShape.length);

  // Source strides over the PADDED source shape.  stride[i] = product of
  // padded[i+1..].  This is the standard row-major stride formula.
  const srcStrides = computeStrides(padded);

  // numel of the output.
  const dstNumel = toShape.reduce((acc, d) => acc * d, 1);

  const out = new Float32Array(dstNumel);

  // Walk every output flat index, compute the corresponding source flat
  // index, and copy.
  //
  // Destination strides aren't precomputed — we unravel `dstFlat` on the
  // fly using a divmod walk.  Equivalent and avoids a second strides
  // array allocation.
  for (let dstFlat = 0; dstFlat < dstNumel; dstFlat++) {
    let remaining = dstFlat;
    let srcFlat = 0;

    // Walk axes left-to-right (most-significant first), peeling off the
    // dst multi-index digit by digit.
    for (let axis = 0; axis < toShape.length; axis++) {
      // dst[axis] = remaining / (product of toShape[axis+1..]); but
      // since we're walking left-to-right and need that denominator,
      // it's easier to use the precomputed source strides which carry
      // the same products from the SOURCE side.  Cleaner approach:
      // compute the dst stride for this axis once per iteration.
      let dstStrideThisAxis = 1;
      for (let j = axis + 1; j < toShape.length; j++) {
        dstStrideThisAxis *= toShape[j]!;
      }
      const dstIdx = Math.floor(remaining / dstStrideThisAxis);
      remaining -= dstIdx * dstStrideThisAxis;

      // For the source: if the padded source dim is 1, this axis is
      // being stretched — force the source index to 0.  Otherwise use
      // the destination index (they must match since we validated the
      // broadcast).
      const srcIdx = padded[axis] === 1 ? 0 : dstIdx;
      srcFlat += srcIdx * srcStrides[axis]!;
    }
    out[dstFlat] = data[srcFlat]!;
  }
  return out;
}

/**
 * Inverse of `broadcastDataTo`.  Given a gradient in `fromShape` layout
 * (which is the broadcast output shape), sum it back down to `toShape`
 * (the original input shape pre-broadcast).
 *
 * ## Why summing?
 *
 * Broadcasting takes one cell and "copies" it to N output positions.
 * In autograd, every output gradient flows back to the same input cell,
 * so the input gradient is the SUM of all the output gradients at the
 * positions where that input cell was used.
 *
 * For dims that were stretched (source dim 1 → target dim N), sum along
 * that axis.  For dims that were padded on the left (added by the rule),
 * sum that whole axis out (the input had no such axis at all).
 *
 * @throws RangeError if `toShape` cannot have broadcast to `fromShape`.
 */
export function unbroadcastDataTo(
  data: Float32Array,
  fromShape: readonly number[],
  toShape: readonly number[],
): Float32Array {
  // Fast path: shapes already match → just copy.
  if (shapesEqual(fromShape, toShape)) {
    const out = new Float32Array(data.length);
    out.set(data);
    return out;
  }

  // Validate that toShape could broadcast to fromShape (the inverse
  // direction).
  const computed = broadcastShapes(fromShape, toShape);
  if (!shapesEqual(computed, fromShape)) {
    throw new RangeError(
      `unbroadcastDataTo: [${toShape.join(", ")}] cannot broadcast to [${fromShape.join(", ")}]`,
    );
  }

  // Left-pad toShape to fromShape's rank.  Axes where padded == 1 but
  // fromShape > 1 are stretched dims → sum along them.  Axes where
  // toShape had no entry at all (the left-padded ones) are also reduced.
  const padded = leftPadShape(toShape, fromShape.length);
  const srcStrides = computeStrides(fromShape);

  // We'll accumulate into a Float32Array of size numel(padded), which
  // equals numel(toShape).  Then return that buffer directly — its
  // shape IS toShape (post-pad-removal is a no-op for the data; the
  // caller already knows the shape).
  const dstNumel = padded.reduce((acc, d) => acc * d, 1);
  const out = new Float32Array(dstNumel);

  // Walk every SOURCE flat index, compute the corresponding dst index,
  // and accumulate.
  const srcNumel = fromShape.reduce((acc, d) => acc * d, 1);
  for (let srcFlat = 0; srcFlat < srcNumel; srcFlat++) {
    // Unravel srcFlat into a multi-index over fromShape.
    let remaining = srcFlat;
    let dstFlat = 0;
    let dstStrideProd = 1;
    // We need the dst strides over padded.  Compute on the fly: walk
    // axes right-to-left, accumulating the stride product, and at the
    // same time peel digits off srcFlat by dividing by srcStrides.
    //
    // Cleaner: compute dst strides up front.
    const dstStrides = computeStrides(padded);
    for (let axis = 0; axis < fromShape.length; axis++) {
      const srcIdx = Math.floor(remaining / srcStrides[axis]!);
      remaining -= srcIdx * srcStrides[axis]!;
      // If padded[axis] == 1, this axis is being summed — clamp dst idx to 0.
      const dstIdx = padded[axis] === 1 ? 0 : srcIdx;
      dstFlat += dstIdx * dstStrides[axis]!;
    }
    // Silence the "dstStrideProd unused" — leftover from earlier algo iteration.
    void dstStrideProd;
    out[dstFlat]! += data[srcFlat]!;
  }
  return out;
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function shapesEqual(a: readonly number[], b: readonly number[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

/**
 * Right-align `shape` to `targetRank` by left-padding with 1s.
 * Returns a new array; doesn't mutate the input.
 */
function leftPadShape(shape: readonly number[], targetRank: number): number[] {
  if (shape.length >= targetRank) return shape.slice();
  const padding = new Array<number>(targetRank - shape.length).fill(1);
  return [...padding, ...shape];
}

/**
 * Row-major strides: `stride[i] = product(shape[i+1..])`.
 * `stride[ndim - 1]` is always 1.  Empty shape returns `[]`.
 */
function computeStrides(shape: readonly number[]): number[] {
  const strides = new Array<number>(shape.length);
  let acc = 1;
  for (let i = shape.length - 1; i >= 0; i--) {
    strides[i] = acc;
    acc *= shape[i]!;
  }
  return strides;
}
