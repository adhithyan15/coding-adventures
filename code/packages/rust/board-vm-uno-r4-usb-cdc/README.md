# board-vm-uno-r4-usb-cdc

Uno R4 WiFi built-in USB CDC backend for Board VM firmware transports.

The Arduino Renesas core exposes the board's USB-C port as `SerialUSB`. The
runtime CDC device enumerates as `0x2341:0x006D` with product name `UNO R4
WiFi`; the upload bootloader path uses `0x2341:0x1002`.

This crate keeps that board-specific USB path outside the generic
`board-vm-usb-cdc` transport crate. It adapts a TinyUSB-style CDC interface to
`BlockingUsbCdc`, while tests use a fake CDC API so the transport semantics stay
host-testable. The ARM FFI backend targets TinyUSB's `tud_cdc_n_*` symbols and
`tud_task_ext`; a flashable firmware binary still needs to link the Arduino/FSP
USB startup and descriptor stack.
