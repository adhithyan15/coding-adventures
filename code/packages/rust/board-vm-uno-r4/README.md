# board-vm-uno-r4

Abstract Arduino Uno R4 target descriptor and HAL adapter for Board VM.

This crate does not simulate the Renesas RA4M1 or Arm Cortex-M4 ISA. It only
describes the board target and adapts a board-specific backend to the portable
`board-vm-runtime` HAL traits.

It also provides the first Uno R4 `board-vm-device` wrapper, so firmware can
instantiate the generic Board VM protocol core with an Uno R4 HAL backend while
UART/USB setup remains outside this crate.
