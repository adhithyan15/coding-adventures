# board-vm-bluetooth

Zero-dependency Bluetooth endpoint parsing for Board VM host clients.

The crate keeps Bluetooth connection endpoint shapes in Rust so language
frontends can stay thin. It does not open OS Bluetooth stacks yet; it validates
and normalizes endpoint strings for the BLE GATT and Bluetooth Classic RFCOMM
transports that later host backends will use.
