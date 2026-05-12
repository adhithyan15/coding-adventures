# board-vm-bluetooth

Zero-dependency Bluetooth endpoint parsing and discovery planning for Board VM
host clients.

The crate keeps Bluetooth connection endpoint shapes in Rust so language
frontends can stay thin. It validates and normalizes endpoint strings for the
BLE GATT and Bluetooth Classic RFCOMM transports, and it exposes a backend
opening trait so OS-specific adapters can return concrete Board VM raw-frame
transports without moving endpoint policy into Ruby/Python/Lua.

The first host adapter is `MacosBluetoothBackend`, which resolves Bluetooth
Classic RFCOMM endpoints onto macOS `/dev/cu.*` serial devices. BLE GATT opening
routes through an injectable `MacosCoreBluetoothBleConnector`; the default
connector still reports an explicit backend error, while
`MacosCoreBluetoothRuntimeBleConnector` wires the macOS CoreBluetooth
delegate/run-loop adapter for service-filtered BLE GATT connections,
characteristic writes, and notification reads. The BLE raw-frame transport
buffers split notifications, preserves extra bytes for the next response, and
rejects empty or unterminated notification streams. BLE GATT and RFCOMM raw-frame
adapters report oversized or unterminated responses as `ResponseTooLarge` through
the shared client transport trait.

OS-specific scanners can also pass discovered device metadata into
`board_vm_endpoint_candidates`. The macOS scanner reads `system_profiler`, the
Linux scanner reads BlueZ metadata through `bluetoothctl devices` and
`bluetoothctl info`, and the Windows scanner reads Bluetooth PnP rows through
PowerShell/CIM. The Rust planner filters for Board VM BLE service/characteristic
UUIDs and RFCOMM channels, then returns concrete endpoint candidates so language
frontends can present boards instead of asking users for UUIDs or Bluetooth
channels.
