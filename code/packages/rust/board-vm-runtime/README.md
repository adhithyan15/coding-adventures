# board-vm-runtime

No-heap interpreter runtime for Board VM bytecode.

The runtime is board-neutral. Targets such as Uno R4 or ESP32 provide a
descriptor and a HAL implementation; the VM, stack semantics, handle table, and
capability dispatch remain shared.

`RunCursor` lets firmware execute bytecode in bounded slices and resume from the
last instruction pointer. That keeps the interpreter board-neutral while giving
board-specific transports a way to stay responsive during long-running programs.
