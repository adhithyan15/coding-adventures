## Unreleased — strict typed scalar CLR artifact execution (CLR06)

Exercise the opt-in typed scalar backend through actual simulator artifacts:
i64 overflow beyond i32, signed arithmetic, high-bit bitwise results, direct
calls, metadata, literal bytes and maximum short slots. Source compilation
routing is unchanged and still narrows scalar hints before legacy lowering.

