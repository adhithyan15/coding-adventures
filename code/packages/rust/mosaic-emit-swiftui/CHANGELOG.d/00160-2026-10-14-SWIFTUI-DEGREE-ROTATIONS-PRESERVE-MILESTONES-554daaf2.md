## 2026-10-14 (SwiftUI degree rotations preserve milestones)

Lower single finite degree rotations to centered rotationEffect modifiers after
surface paint and before elevation shadows. Base/state precedence remains intact;
unsupported units, transform chains, invalid and non-finite angles remain drops.
TaskApp's two milestone diamonds retire two transform drops, reducing its measured
SwiftUI inventory from 59 to 57 (#17502).
