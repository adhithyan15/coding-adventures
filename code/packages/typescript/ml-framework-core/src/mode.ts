/**
 * # mode.ts — global train/eval mode toggle
 *
 * A handful of ops (Dropout, BatchNorm) behave differently during
 * training vs inference:
 *
 *   - **Dropout** randomly zeros activations in train mode; in eval mode
 *     it's a no-op.
 *   - **BatchNorm** uses the current batch's mean/var in train mode and
 *     UPDATES running statistics; in eval mode it uses the frozen running
 *     statistics computed during training.
 *
 * PyTorch handles this per-module: every `nn.Module` has a `.train()` /
 * `.eval()` method that propagates a flag down the tree.  We don't have
 * a Module abstraction yet (Phase A.6 introduces Linear/Sequential), so
 * for v1.4 we use a single global flag that all mode-sensitive ops
 * consult.  This is the simplest API that makes the train/eval
 * distinction explicit and reproducible.
 *
 * ## Trade-off
 *
 * A global is awkward if you want to evaluate one sub-network in eval
 * mode while another stays in train mode — you can't.  In practice
 * training scripts toggle the global once at the start of evaluation
 * and once back when resuming training, so this is fine.  A.6 will
 * introduce per-Module mode if needed.
 *
 * ## Usage
 *
 * ```ts
 * import { setMode, getMode } from "@coding-adventures/ml-framework-core";
 *
 * // (default is "train" — no setup needed for training loops)
 * setMode("eval");
 * const predictions = model.forward(testInput);
 * setMode("train");
 * ```
 */

export type Mode = "train" | "eval";

let _mode: Mode = "train";

/** Returns the current global training mode.  Default: `"train"`. */
export function getMode(): Mode {
  return _mode;
}

/**
 * Sets the global training mode.  Affects `Dropout` (random masking
 * vs passthrough) and `BatchNorm` (batch stats + running update vs
 * frozen running stats).
 */
export function setMode(m: Mode): void {
  if (m !== "train" && m !== "eval") {
    throw new TypeError(`mode must be "train" or "eval", got ${JSON.stringify(m)}`);
  }
  _mode = m;
}
