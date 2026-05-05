# board-vm-uno-r4-usb-cdc

Uno R4 WiFi built-in USB CDC backend for Board VM firmware transports.

The Arduino Renesas core exposes the board's USB-C port as `SerialUSB`. The
runtime CDC device enumerates as `0x2341:0x006D` with product name `UNO R4
WiFi`; the upload bootloader path uses `0x2341:0x1002`.

This crate keeps that board-specific USB path outside the generic
`board-vm-usb-cdc` transport crate. It adapts a TinyUSB-style CDC interface to
`BlockingUsbCdc`, while tests use a fake CDC API so the transport semantics stay
host-testable. The ARM FFI backend targets TinyUSB's `tud_cdc_n_*` symbols and
`tud_task_ext`.

`begin()` calls Arduino Renesas `__USBStart()` through the core's C++ ABI symbol,
then marks the CDC stream as ready. The crate also provides the C++-mangled
`__USBInstallSerial()` hook so Arduino's descriptor builder includes the CDC
interface without linking the full `SerialUSB.cpp` object, and it overrides the
Uno R4 WiFi `configure_usb_mux()` weak hook so the USB-C connector is routed to
the RA4M1 USB peripheral. It also overrides the variant
`usb_post_initialization()` hook to enable the RA4M1 USB regulator bit after
TinyUSB initialization, matching the Arduino Uno R4 WiFi variant startup. The
mux hook calls the same FSP `R_IOPORT_PinCfg`/`R_IOPORT_PinWrite` path that
Arduino's `pinMode(21, OUTPUT); digitalWrite(21, HIGH);` sequence uses, without
linking the full WiFi `variant.cpp` object.

For hardware smoke tests, a board that still enumerates as VID/PID
`2341:1002` after upload has not handed USB-C off from the bridge. If the bridge
port disappears but the host sees an unmatched USB device stuck before CDC port
creation, the mux hook ran and the remaining failure is in RA4M1 USB device
enumeration.

The Arduino descriptor builder allocates its configuration descriptor at USB
startup. Because the Rust firmware links without a C runtime allocator, this
crate provides a tiny no-free `malloc`/`free` shim sized for that descriptor path
when built for ARM.

A flashable firmware binary still needs to compile and link the Arduino/FSP
TinyUSB startup objects listed by `board-vm-uno-r4-firmware`'s Arduino USB link
manifest.
