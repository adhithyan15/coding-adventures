# board-vm-bluetooth

Zero-dependency Bluetooth endpoint parsing and discovery planning for Board VM
host clients.

The crate keeps Bluetooth connection endpoint shapes in Rust so language
frontends can stay thin. It does not open OS Bluetooth stacks yet; it validates
and normalizes endpoint strings for the BLE GATT and Bluetooth Classic RFCOMM
transports that later host backends will use.

OS-specific scanners can also pass discovered device metadata into
`board_vm_endpoint_candidates`. The Rust planner filters for Board VM BLE
service/characteristic UUIDs and RFCOMM channels, then returns concrete endpoint
candidates so language frontends can present boards instead of asking users for
UUIDs or Bluetooth channels.
