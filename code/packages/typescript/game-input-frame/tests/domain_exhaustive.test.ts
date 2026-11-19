import assert from "node:assert/strict";
import test from "node:test";

import { FrameError, reduce_frame, type ActionChange, type InputState } from "../src/index.js";

const zeroState = (overrides: Partial<InputState> = {}): InputState => ({
  player_id: 0,
  held_buttons: 0n,
  axes: Array<number>(16).fill(0),
  ...overrides,
});

test("all 256 raw slots preserve width and reject only outside the fixed profiles", () => {
  for (let slot = 0; slot <= 0xff; slot += 1) {
    const button = reduce_frame(zeroState(), 0n, [{ kind: "button", slot, down: true }]);
    if (slot < 64) {
      assert.equal(button.ok, true, `button slot ${slot}`);
      if (button.ok) assert.equal(button.value.held_buttons, 1n << BigInt(slot));
    } else {
      assert.deepEqual(button, {
        ok: false,
        error: { code: FrameError.InvalidButtonActionId, change_index: 0, action_id: slot },
      });
    }

    const axis = reduce_frame(zeroState(), 0n, [{ kind: "axis", slot, value: 0 }]);
    if (slot < 16) {
      assert.equal(axis.ok, true, `axis slot ${slot}`);
    } else {
      assert.deepEqual(axis, {
        ok: false,
        error: { code: FrameError.InvalidAxisActionId, change_index: 0, action_id: slot },
      });
    }
  }
});

test("all raw i16 values visit all prior and update axis slots (2,097,152 reducer calls)", () => {
  let priorCalls = 0;
  let updateCalls = 0;

  for (let slot = 0; slot < 16; slot += 1) {
    for (let raw = -0x8000; raw <= 0x7fff; raw += 1) {
      const previousAxes = Array<number>(16).fill(0);
      previousAxes[slot] = raw;
      const prior = reduce_frame(zeroState({ axes: previousAxes }), 0n, []);
      priorCalls += 1;
      if (raw === -0x8000) {
        if (
          prior.ok ||
          prior.error.code !== FrameError.InvalidPreviousAxisValue ||
          prior.error.axis_slot !== slot ||
          prior.error.value !== raw
        ) {
          throw new Error(`prior-axis mismatch slot=${slot} value=${raw}`);
        }
      } else if (!prior.ok || prior.value.axes[slot] !== raw) {
        throw new Error(`prior-axis acceptance mismatch slot=${slot} value=${raw}`);
      }

      const update = reduce_frame(zeroState(), 0n, [{ kind: "axis", slot, value: raw }]);
      updateCalls += 1;
      if (raw === -0x8000) {
        if (
          update.ok ||
          update.error.code !== FrameError.InvalidAxisValue ||
          update.error.change_index !== 0 ||
          update.error.axis_slot !== slot ||
          update.error.value !== raw
        ) {
          throw new Error(`update-axis mismatch slot=${slot} value=${raw}`);
        }
      } else if (!update.ok || update.value.axes[slot] !== raw) {
        throw new Error(`update-axis acceptance mismatch slot=${slot} value=${raw}`);
      }
    }
  }

  assert.equal(priorCalls, 16 * 65_536);
  assert.equal(updateCalls, 16 * 65_536);
  assert.equal(priorCalls + updateCalls, 2_097_152);
});

test("255 and 256 changes are admitted, 257 refuses before inspecting the sequence", () => {
  const noops: ActionChange[] = Array.from({ length: 256 }, () => ({
    kind: "button",
    slot: 0,
    down: false,
  }));
  assert.equal(reduce_frame(zeroState(), 1n, noops.slice(0, 255)).ok, true);
  assert.equal(reduce_frame(zeroState(), 1n, noops).ok, true);

  const unreadable = Array.from({ length: 257 }, () => {
    return Object.defineProperty({}, "kind", {
      get() {
        throw new Error("over-limit event was inspected");
      },
    }) as ActionChange;
  });
  assert.deepEqual(reduce_frame(zeroState(), 1n, unreadable), {
    ok: false,
    error: { code: FrameError.TooManyChanges },
  });
});
