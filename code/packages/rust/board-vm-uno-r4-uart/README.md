# board-vm-uno-r4-uart

Uno R4 WiFi UART backend for Board VM firmware transports.

The Arduino Uno R4 WiFi core builds sketches with `NO_USB`, so Arduino
`Serial` maps to `_UART1_`, not native USB CDC. In the Uno R4 WiFi variant,
`_UART1_` uses D22/D23, which are RA4M1 P109/P110 on SCI9. This crate keeps
that board-specific setup outside the generic Board VM runtime and exposes it
as a `board-vm-uart` `BlockingUart`.

The first implementation supports the host/default `115200 8N1` profile.
