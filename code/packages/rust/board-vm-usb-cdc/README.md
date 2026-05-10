# board-vm-usb-cdc

Reusable no-heap USB CDC byte stream adapter for Board VM device firmware.

This crate does not own a USB device controller or descriptors. Board-specific
packages provide that lower layer and implement `BlockingUsbCdc`; this crate
adapts that CDC byte pipe into `board-vm-device`'s `DeviceByteStream` so the
same `DeviceStreamEndpoint` can serve Board VM protocol frames over USB CDC,
UART, loopback streams, or simulator transports.

The first expected hardware consumer is the Arduino Uno R4 WiFi built-in USB
serial route. Its Arduino core exposes the USB-C port as `SerialUSB`, while
the earlier UART server uses `Serial1` on D22/D23. Keeping CDC as a separate
package lets the Uno R4 USB backend share framing/session logic with other
boards that expose a CDC ACM serial device.

Validation:

```sh
cargo test -p board-vm-usb-cdc -- --nocapture
```
