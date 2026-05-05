# board-vm-uno-r4

Abstract Arduino Uno R4 target descriptor and HAL adapter for Board VM.

This crate does not simulate the Renesas RA4M1 or Arm Cortex-M4 ISA. It only
describes the board target and adapts a board-specific backend to the portable
`board-vm-runtime` HAL traits.
