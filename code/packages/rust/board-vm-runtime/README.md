# board-vm-runtime

No-heap interpreter runtime for Board VM bytecode.

The runtime is board-neutral. Targets such as Uno R4 or ESP32 provide a
descriptor and a HAL implementation; the VM, stack semantics, handle table, and
capability dispatch remain shared.
