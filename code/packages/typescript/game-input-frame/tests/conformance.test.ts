import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import test from "node:test";

import { reduce_frame, type ActionChange, type InputState } from "../src/index.js";

const CORPUS_SHA256 = "8db2c0fdc9bb6ab93785f3bd6d5dc52183b67682ca0e8d693fcdbfaa30aff785";
const EXPECTED_CASE_IDS = [
  "empty_frame_preserves_levels_and_clears_edges",
  "press_from_up",
  "release_from_down",
  "duplicate_down_is_noop",
  "duplicate_up_is_noop",
  "same_tick_tap_sets_both_edges",
  "repeated_taps_are_presence_masks",
  "down_up_down_finishes_held",
  "axis_update_and_persistence",
  "axis_last_change_wins",
  "button_slot_63_and_full_width_tick",
  "u32_player_max_and_axis_lower_bound",
  "too_many_precedes_invalid_state_and_event",
  "lowest_invalid_prior_axis_wins",
  "invalid_button_slot",
  "invalid_axis_slot_precedes_axis_value",
  "invalid_axis_value",
  "first_invalid_change_wins",
  "accept_255_changes",
  "accept_256_changes",
  "reject_257_changes",
] as const;

interface CorpusCase {
  id: string;
  input: {
    player_id: number;
    tick: string;
    state: { held_buttons: string; axes: number[] };
    changes: ActionChange[];
  };
  expected:
    | { frame: Omit<Record<string, unknown>, "tick"> & { tick: string } }
    | { error: Record<string, unknown> };
}

function loadCorpus(): { bytes: Buffer; cases: CorpusCase[] } {
  const path = resolve(process.cwd(), "../../../specs/fixtures/game-input-frame-v1/cases.json");
  const bytes = readFileSync(path);
  assert.equal(createHash("sha256").update(bytes).digest("hex"), CORPUS_SHA256);
  const parsed = JSON.parse(bytes.toString("utf8")) as { cases: CorpusCase[] };
  return { bytes, cases: parsed.cases };
}

function encodeResult(result: ReturnType<typeof reduce_frame>): Record<string, unknown> {
  if (!result.ok) return { error: result.error };
  const frame = result.value;
  return {
    frame: {
      player_id: frame.player_id,
      tick: frame.tick.toString(),
      held_buttons: frame.held_buttons.toString(),
      pressed_buttons: frame.pressed_buttons.toString(),
      released_buttons: frame.released_buttons.toString(),
      axes: [...frame.axes],
    },
  };
}

test("the public reducer matches every canonical game-input-frame-v1 vector", () => {
  const { cases } = loadCorpus();
  assert.equal(cases.length, EXPECTED_CASE_IDS.length);
  assert.deepEqual(
    cases.map(({ id }) => id).sort(),
    [...EXPECTED_CASE_IDS].sort(),
  );

  for (const item of cases) {
    const previous: InputState = {
      player_id: item.input.player_id,
      held_buttons: BigInt(item.input.state.held_buttons),
      axes: item.input.state.axes,
    };
    const actual = encodeResult(
      reduce_frame(previous, BigInt(item.input.tick), item.input.changes),
    );
    assert.deepEqual(actual, item.expected, item.id);
  }
});
