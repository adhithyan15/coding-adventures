/**
 * A deterministic, caller-ticked snapshot of one player's logical input.
 *
 * This reducer begins after a host has normalized controls into stable action
 * slots. It owns no devices, clock, player registry, or key mapping. The fixed
 * profile contains 64 button levels, 16 absolute axes, and at most 256 ordered
 * changes for one caller-supplied tick.
 */

import { bit_is_set, clear_bit, set_bit } from "./digital.js";

export const FrameError = Object.freeze({
  TooManyChanges: "TooManyChanges",
  InvalidPreviousAxisValue: "InvalidPreviousAxisValue",
  InvalidButtonActionId: "InvalidButtonActionId",
  InvalidAxisActionId: "InvalidAxisActionId",
  InvalidAxisValue: "InvalidAxisValue",
} as const);

export type FrameErrorCode = (typeof FrameError)[keyof typeof FrameError];

export type FrameErrorValue =
  | Readonly<{ code: typeof FrameError.TooManyChanges }>
  | Readonly<{
      code: typeof FrameError.InvalidPreviousAxisValue;
      axis_slot: number;
      value: number;
    }>
  | Readonly<{
      code: typeof FrameError.InvalidButtonActionId;
      change_index: number;
      action_id: number;
    }>
  | Readonly<{
      code: typeof FrameError.InvalidAxisActionId;
      change_index: number;
      action_id: number;
    }>
  | Readonly<{
      code: typeof FrameError.InvalidAxisValue;
      change_index: number;
      axis_slot: number;
      value: number;
    }>;

export interface InputState {
  readonly player_id: number;
  readonly held_buttons: bigint;
  readonly axes: readonly number[];
}

export type ActionChange =
  | Readonly<{ kind: "button"; slot: number; down: boolean }>
  | Readonly<{ kind: "axis"; slot: number; value: number }>;

export interface ActionFrame {
  readonly player_id: number;
  readonly tick: bigint;
  readonly held_buttons: bigint;
  readonly pressed_buttons: bigint;
  readonly released_buttons: bigint;
  readonly axes: readonly number[];
}

export type Result<T> =
  | Readonly<{ ok: true; value: T }>
  | Readonly<{ ok: false; error: FrameErrorValue }>;

export const BUTTON_SLOT_COUNT = 64;
export const AXIS_SLOT_COUNT = 16;
export const MAX_CHANGES = 256;
export const MAX_U64 = (1n << 64n) - 1n;
export const MAX_U32 = 0xffff_ffff;

function record(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object record`);
  }
  return value as Record<string, unknown>;
}

function ownField(input: Record<string, unknown>, field: string, label: string): unknown {
  if (!Object.hasOwn(input, field)) throw new TypeError(`${label}.${field} must be present`);
  return input[field];
}

function u64(value: unknown, label: string): bigint {
  if (typeof value !== "bigint") throw new TypeError(`${label} must be a bigint`);
  if (value < 0n || value > MAX_U64) throw new RangeError(`${label} must be in 0..=u64::MAX`);
  return value;
}

function numberInteger(value: unknown, low: number, high: number, label: string): number {
  if (typeof value !== "number") throw new TypeError(`${label} must be a number`);
  if (!Number.isFinite(value) || !Number.isInteger(value) || value < low || value > high) {
    throw new RangeError(`${label} must be a finite integer in ${low}..=${high}`);
  }
  // JavaScript considers -0 an integer and Rust's integer zero has no sign.
  return value === 0 ? 0 : value;
}

function readPrevious(value: unknown): {
  player_id: number;
  held_buttons: bigint;
  axes: number[];
} {
  const input = record(value, "previous");
  const player_id = numberInteger(
    ownField(input, "player_id", "previous"),
    0,
    MAX_U32,
    "previous.player_id",
  );
  const held_buttons = u64(ownField(input, "held_buttons", "previous"), "previous.held_buttons");
  const rawAxes = ownField(input, "axes", "previous");
  if (!Array.isArray(rawAxes)) throw new TypeError("previous.axes must be an array");
  if (rawAxes.length !== AXIS_SLOT_COUNT) {
    throw new RangeError(`previous.axes must contain exactly ${AXIS_SLOT_COUNT} values`);
  }

  const axes: number[] = [];
  for (let slot = 0; slot < AXIS_SLOT_COUNT; slot += 1) {
    if (!Object.hasOwn(rawAxes, slot)) throw new TypeError(`previous.axes[${slot}] must be present`);
    const valueAtSlot = numberInteger(
      rawAxes[slot],
      -0x8000,
      0x7fff,
      `previous.axes[${slot}]`,
    );
    axes.push(valueAtSlot);
    if (valueAtSlot < -0x7fff || valueAtSlot > 0x7fff) {
      return {
        player_id,
        held_buttons,
        axes,
      };
    }
  }
  return { player_id, held_buttons, axes };
}

function resultError(error: FrameErrorValue): Result<never> {
  return Object.freeze({ ok: false, error: Object.freeze(error) });
}

/**
 * Reduce an ordered batch without mutating caller-owned inputs.
 *
 * The batch bound is checked before tick or prior-state fields are read. Prior
 * axes are checked from slot zero upward; each event is then visited in order,
 * and its slot is validated before its level/value.
 */
export function reduce_frame(
  previousValue: InputState,
  tickValue: bigint,
  changesValue: readonly ActionChange[],
): Result<ActionFrame> {
  if (!Array.isArray(changesValue)) throw new TypeError("changes must be an array");
  const changeCount: unknown = changesValue.length;
  if (typeof changeCount !== "number") throw new TypeError("changes.length must be a number");
  if (!Number.isInteger(changeCount) || changeCount < 0 || changeCount > 0xffff_ffff) {
    throw new RangeError("changes.length must be a finite array-length integer");
  }
  if (changeCount > MAX_CHANGES) {
    return resultError({ code: FrameError.TooManyChanges });
  }

  const tick = u64(tickValue, "tick");
  const previous = readPrevious(previousValue);
  for (let axisSlot = 0; axisSlot < previous.axes.length; axisSlot += 1) {
    const value = previous.axes[axisSlot];
    if (value < -0x7fff || value > 0x7fff) {
      return resultError({
        code: FrameError.InvalidPreviousAxisValue,
        axis_slot: axisSlot,
        value,
      });
    }
  }

  let held = previous.held_buttons;
  let pressed = 0n;
  let released = 0n;
  const axes = [...previous.axes];

  for (let changeIndex = 0; changeIndex < changeCount; changeIndex += 1) {
    if (!Object.hasOwn(changesValue, changeIndex)) {
      throw new TypeError(`changes[${changeIndex}] must be present`);
    }
    const change = record(changesValue[changeIndex], `changes[${changeIndex}]`);
    const kind = ownField(change, "kind", `changes[${changeIndex}]`);
    if (kind === "button") {
      const slot = numberInteger(
        ownField(change, "slot", `changes[${changeIndex}]`),
        0,
        0xff,
        `changes[${changeIndex}].slot`,
      );
      if (slot >= BUTTON_SLOT_COUNT) {
        return resultError({
          code: FrameError.InvalidButtonActionId,
          change_index: changeIndex,
          action_id: slot,
        });
      }
      const down = ownField(change, "down", `changes[${changeIndex}]`);
      if (typeof down !== "boolean") throw new TypeError(`changes[${changeIndex}].down must be a boolean`);
      const wasDown = bit_is_set(held, slot);
      if (down && !wasDown) {
        held = set_bit(held, slot);
        pressed = set_bit(pressed, slot);
      } else if (!down && wasDown) {
        held = clear_bit(held, slot);
        released = set_bit(released, slot);
      }
    } else if (kind === "axis") {
      const slot = numberInteger(
        ownField(change, "slot", `changes[${changeIndex}]`),
        0,
        0xff,
        `changes[${changeIndex}].slot`,
      );
      if (slot >= AXIS_SLOT_COUNT) {
        return resultError({
          code: FrameError.InvalidAxisActionId,
          change_index: changeIndex,
          action_id: slot,
        });
      }
      const value = numberInteger(
        ownField(change, "value", `changes[${changeIndex}]`),
        -0x8000,
        0x7fff,
        `changes[${changeIndex}].value`,
      );
      if (value < -0x7fff || value > 0x7fff) {
        return resultError({
          code: FrameError.InvalidAxisValue,
          change_index: changeIndex,
          axis_slot: slot,
          value,
        });
      }
      axes[slot] = value;
    } else {
      throw new TypeError(`changes[${changeIndex}].kind must be button or axis`);
    }
  }

  const frame = Object.freeze({
    player_id: previous.player_id,
    tick,
    held_buttons: held,
    pressed_buttons: pressed,
    released_buttons: released,
    axes: Object.freeze(axes),
  });
  return Object.freeze({ ok: true, value: frame });
}
