# game-input-frame

`game-input-frame` reduces one player's prior logical button and axis levels
plus a bounded ordered batch of normalized action changes into one deterministic
simulation-tick frame. It does not poll devices, map physical controls, assign
ticks, or depend on windows or clocks. See
[`GAME-input-action-frame.md`](../../../specs/GAME-input-action-frame.md) for
the v1 contract and [`game-input-frame-v1`](../../../specs/fixtures/game-input-frame-v1/README.md)
for the independent vectors.

```rust
use game_input_frame::{reduce_frame, ActionChange, AxisActionId, ButtonActionId, InputState, PlayerId};

let before = InputState { player_id: PlayerId::new(7), held_buttons: 0, axes: [0; 16] };
let changes = [
    ActionChange::Button { action_id: ButtonActionId::new(0), down: true },
    ActionChange::Button { action_id: ButtonActionId::new(0), down: false },
    ActionChange::Axis { action_id: AxisActionId::new(2), value: 4096 },
];
let frame = reduce_frame(&before, 42, &changes).expect("valid bounded frame");
assert_eq!(frame.pressed_buttons, 1);
assert_eq!(frame.released_buttons, 1);
assert_eq!(frame.held_buttons, 0);
assert_eq!(frame.axes[2], 4096);
```

The edge masks record whether each transition occurred during the tick. They
do not preserve repeated-tap counts or event ordering; keep the ordered changes
when an application needs that history. One frame accepts 64 button slots, 16
signed absolute axes and at most 256 changes. Wider layouts and device
aggregation require separate contracts.
