# modbus-protocol

`modbus-protocol` implements the small, reusable wire boundary needed by local
Modbus TCP clients. It encodes function `0x03` and `0x04` read requests and
strictly decodes their responses. It also owns the fixed basic Read Device
Identification (`0x2B/0x0E`) exchange for mandatory vendor name, product code,
and revision objects, including bounded pagination and exact correlation.

The package intentionally exposes no register-write encoder, caller-selected
regular or extended identity object, or generic MEI function. Modbus TCP does
not authenticate peers by itself, so command policy belongs in a separately
secured host rather than this read-only breadth slice.

```rust
use modbus_protocol::{decode_read_response, encode_read_request, ReadRegistersRequest, RegisterTable};

let request = ReadRegistersRequest::new(RegisterTable::Holding, 0, 2)?;
let bytes = encode_read_request(7, 1, request);
assert_eq!(bytes.len(), 12);
# Ok::<(), modbus_protocol::ModbusError>(())
```
