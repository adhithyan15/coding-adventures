# board-vm-device

Generic board-side protocol device core for Board VM firmware.

The crate turns validated raw protocol frames into runtime operations over a
caller-provided `BoardHal`. It owns the in-RAM uploaded program, upload CRC
checks, capability reporting, run/stop responses, and structured board error
frames.

Board targets such as Uno R4 supply the concrete HAL and descriptor. They can
either feed decoded raw frames into `BoardVmDevice` directly or wrap a UART, USB
CDC, BLE, TCP tunnel, or simulator byte stream with `DeviceStreamEndpoint`. The
stream endpoint reads one zero-terminated COBS wire frame, dispatches it through
the device, writes the encoded response frame, and keeps the transport itself
board-specific.
