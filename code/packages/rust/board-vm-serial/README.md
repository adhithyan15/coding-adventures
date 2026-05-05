# board-vm-serial

Serial-port transport for Board VM host clients.

The crate opens an OS serial device with the `serialport` crate, wraps it in
`board-vm-stream` COBS framing, and implements the `board-vm-client`
`RawFrameTransport` trait.

Example host shape:

```rust
use board_vm_client::BoardVmClient;
use board_vm_serial::{BoardSerialTransport, SerialConfig};

let config = SerialConfig::new("/dev/ttyACM0").baud_rate(115_200);
let transport = BoardSerialTransport::<_, 1024>::open(&config)?;
let mut client: BoardVmClient<_, 512, 768, 768> = BoardVmClient::new(transport);
let hello = client.hello(0x1234_5678)?;
```

The crate does not assume Arduino, USB CDC, or UART semantics beyond "a byte
stream with read/write timeouts". Board detection and flashing remain separate.
