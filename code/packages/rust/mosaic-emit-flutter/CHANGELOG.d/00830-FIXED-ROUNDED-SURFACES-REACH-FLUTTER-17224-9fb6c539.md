### Fixed — rounded surfaces reach Flutter (#17224)

Supported fixed `border-radius` declarations now lower through Flutter
decorations, clips, and input borders across draggable surfaces, text chips,
borderless inputs, and flex containers. Non-uniform per-edge borders keep their
safe inner decoration and receive an outer `ClipRRect`; rounded text surfaces
paint their background in the same native decoration.

Fresh TaskApp generation removes all 15 Flutter `border-radius` drops plus
eight coupled text-surface `background` drops, reducing its measured Flutter
style-degradation inventory from 144 to 121. Relative, negative, and otherwise
unsafe values remain reported.
