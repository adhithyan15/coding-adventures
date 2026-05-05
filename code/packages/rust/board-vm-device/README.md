# board-vm-device

Generic board-side protocol device core for Board VM firmware.

The crate turns validated raw protocol frames into runtime operations over a
caller-provided `BoardHal`. It owns the in-RAM uploaded program, upload CRC
checks, capability reporting, run/stop responses, and structured board error
frames.

Board targets such as Uno R4 supply the concrete HAL and descriptor. UART, USB
CDC, BLE, TCP tunnels, or simulator byte streams can sit outside this crate and
feed decoded raw frames into `BoardVmDevice`.
