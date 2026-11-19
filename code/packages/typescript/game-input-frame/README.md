# `@coding-adventures/game-input-frame`

`game-input-frame` builds one deterministic logical input snapshot for a
player and a caller-assigned simulation tick. It starts after a host has
normalized controls into stable action IDs. It does not poll devices, assign
ticks, bind keys, merge devices, or handle focus and disconnect behavior.

The pure `reduce_frame(previous, tick, changes)` function supports a u32 player
ID, u64 BigInt tick and held-button mask, 64 button slots, 16 absolute signed
axes, and at most 256 ordered changes. It preserves within-tick tap edges while
keeping final button levels and absolute axis values. Invalid semantic input
returns a frozen portable error result; wrong JavaScript types and values
outside their representation widths raise `TypeError` or `RangeError`.

```ts
import { reduce_frame, type ActionChange, type InputState } from "@coding-adventures/game-input-frame";

const previous: InputState = {
  player_id: 3,
  held_buttons: 0n,
  axes: Array<number>(16).fill(0),
};
const changes: ActionChange[] = [
  { kind: "button", slot: 63, down: true },
  { kind: "axis", slot: 2, value: -1200 },
];
const result = reduce_frame(previous, 42n, changes);
if (result.ok) {
  console.log(result.value.held_buttons.toString());
}
```

The implementation has no runtime dependencies. `BUILD` and `BUILD_windows`
run TypeScript compilation, canonical vector tests, exhaustive input-domain
checks, seeded property/fuzz tests, and a bounded SAT miter. A separate
development-time Rust proof command binds `src/digital.ts` bytes to the shared
TypeScript AST/WIR bridge and SAT solver; it does not enter the published
runtime dependency graph. The helper proof compares the three lowered
production helpers over all u64 masks and six-bit slots, and guarded helper
mutants are replayed through the public reducer. Separately, a bounded reducer
miter covers three ordered transitions to one fixed slot for every slot and
held mask. Neither result is a whole-reducer source proof.

See [`GAME input action frame v1`](../../../specs/GAME-input-action-frame.md)
and [the TypeScript binding/proof acceptance](../../../specs/GBL-050a-typescript-input-frame.md)
for the exact boundary, precedence, evidence limits, and follow-up contracts.
