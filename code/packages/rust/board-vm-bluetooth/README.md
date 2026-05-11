# board-vm-bluetooth

Zero-dependency Bluetooth endpoint parsing and discovery planning for Board VM
host clients.

The crate keeps Bluetooth connection endpoint shapes in Rust so language
frontends can stay thin. It validates and normalizes endpoint strings for the
BLE GATT and Bluetooth Classic RFCOMM transports, and it exposes a backend
opening trait so OS-specific adapters can return concrete Board VM raw-frame
transports without moving endpoint policy into Ruby/Python/Lua.

OS-specific scanners can also pass discovered device metadata into
`board_vm_endpoint_candidates`. The Rust planner filters for Board VM BLE
service/characteristic UUIDs and RFCOMM channels, then returns concrete endpoint
candidates so language frontends can present boards instead of asking users for
UUIDs or Bluetooth channels.
