# BVM04 - Board VM Host SDKs

## Overview

Board VM is useful only if it feels natural from many programming languages.
The host SDK contract defines how JS, Python, Ruby, Java, Lua, and CLI clients
talk to the same board through the same binary protocol and bytecode IR.

The SDKs are language-native at the surface, but boring underneath:

```
language API -> shared operation model -> BVM bytecode/protocol -> transport
```

The board must not know which language produced the bytes.

## Layer Position

```
User code in JS / Python / Ruby / Java / Lua
        |
        v
Language-specific SDK facade
        |
        v
Shared BVM client model
        |
        v
BVM01 protocol + BVM02 bytecode
        |
        v
Serial / WebSerial / BLE / TCP
        |
        v
Board VM runtime
```

## SDK Layers

Each SDK should be split internally into five layers.

| Layer | Responsibility |
|---|---|
| Transport | Open/read/write bytes for serial, WebSerial, BLE, TCP, etc. |
| Codec | Encode/decode BVM01 frames and payloads |
| Client | Request/response, timeouts, retries, board descriptor cache |
| Compiler | Turn high-level operations into BVM02 bytecode |
| Facade | Language-native `board.gpio(13).output()` style API |

Only the facade should feel different across languages. The codec and compiler
must be validated by shared golden vectors.

## Reference API Shape

The language-specific spelling can vary, but the conceptual API should be the
same.

```typescript
const board = await BoardVm.connect({ path: "/dev/tty.usbmodem101" });
const led = await board.gpio(13).output();

await led.high();
await board.sleepMs(250);
await led.low();

await board.eject("blink", async program => {
  const led = await program.gpio(13).output();
  while (true) {
    await led.high();
    await program.sleepMs(250);
    await led.low();
    await program.sleepMs(250);
  }
});
```

Equivalent Python:

```python
board = await BoardVm.connect(path="/dev/tty.usbmodem101")
led = await board.gpio(13).output()

await led.high()
await board.sleep_ms(250)
await led.low()
```

Equivalent Ruby:

```ruby
board = BoardVm.connect(path: "/dev/tty.usbmodem101")
led = board.gpio(13).output

led.high
board.sleep_ms(250)
led.low
```

All three examples must eventually encode the same operations.

## REPL Contract

The first CLI REPL can be line-oriented:

```
bvm> connect /dev/tty.usbmodem101
connected arduino-uno runtime=bvm-rust-0.1
bvm> caps
bvm> gpio 13 output
handle 1
bvm> write 1 high
ok
bvm> sleep 250
ok
bvm> write 1 low
ok
bvm> eject blink examples/blink.bvm
stored slot 0 boot=run
```

The REPL is not the protocol. It is a friendly host client that uses the same
messages and bytecode as every SDK.

## Board Discovery

SDKs should support explicit and assisted connection.

Explicit:

```
connect(path="/dev/tty.usbmodem101", baud=115200)
```

Assisted:

```
discover()
  -> list serial ports
  -> try HELLO on likely ports
  -> return BoardDescriptor entries
```

Discovery must be conservative. It should not spam arbitrary serial devices
with long messages. A single short `HELLO` with timeout is acceptable.

## Golden Vectors

The repository should contain shared fixtures:

```
code/fixtures/board-vm/protocol/
  hello.json
  hello.raw.hex
  caps-report.json
  caps-report.raw.hex

code/fixtures/board-vm/bytecode/
  blink.json
  blink.hex
  gpio-write-high.json
  gpio-write-high.hex
```

Every SDK must prove:

1. JSON model -> bytes equals `.hex`.
2. bytes -> JSON model equals `.json`.
3. High-level blink builder -> `blink.hex`.

This is the mechanism that keeps Ruby, Python, JS, Java, Lua, and Rust honest.

## Eject Contract

An SDK can eject in three ways, depending on board support and host tooling:

| Eject mode | Requirement | SDK action |
|---|---|---|
| `store` | board reports `supports_store_program` | upload module and send `STORE_PROGRAM` |
| `firmware` | board package supports firmware generation | emit or flash runtime + embedded module |
| `artifact` | user wants files only | write bytecode module and manifest |

The eject manifest:

```json
{
  "format": "board-vm-eject-v1",
  "board_id": "arduino-uno",
  "runtime": "board-vm-rust",
  "program": "blink.bvm",
  "bytecode_sha256": "...",
  "capabilities": ["gpio.digital@1", "time.sleep@1"],
  "entry": "main"
}
```

SDKs must validate the program against the board descriptor before ejecting.

## Error Handling

Every SDK maps protocol errors to language-native exceptions or result types,
but preserves:

```
code
name
request_id
program_id
bytecode_offset
message
```

High-level APIs should fail early where possible. If the board does not report
PWM, `board.pwm(...)` should fail before sending bytecode.

## Concurrency

The protocol does not require pipelining in v1. SDKs should serialize requests
unless the board descriptor reports `transport.pipelining`.

Language-specific async support:

| Language | Default |
|---|---|
| JS/TypeScript | Promise-based |
| Python | asyncio first, sync wrapper optional |
| Ruby | sync first, fiber support later |
| Java | blocking client first, CompletableFuture later |
| Lua | sync first |
| Rust host | sync client first, async feature later |

## Public API

The conceptual client interface:

```rust
pub trait BoardVmClient {
    fn connect(target: ConnectTarget) -> Result<Self, ConnectError>
    where
        Self: Sized;

    fn descriptor(&self) -> &BoardDescriptor;
    fn upload(&mut self, program: BytecodeModule) -> Result<ProgramId, ClientError>;
    fn run(&mut self, program: ProgramId, options: RunOptions) -> Result<RunReport, ClientError>;
    fn store(&mut self, program: ProgramId, slot: u8, boot: BootPolicy) -> Result<(), ClientError>;
    fn reset_vm(&mut self) -> Result<(), ClientError>;
}
```

The high-level hardware facade:

```rust
pub trait BoardFacade {
    fn gpio(&mut self, pin: u16) -> GpioBuilder;
    fn sleep_ms(&mut self, ms: u16) -> Result<(), ClientError>;
    fn compile_program<F>(&mut self, build: F) -> Result<BytecodeModule, CompileError>;
}
```

## Test Strategy

- Golden protocol vectors in every SDK.
- Golden bytecode vectors in every SDK.
- Fake transport tests for request/response and timeouts.
- Descriptor-driven unsupported-operation tests.
- REPL smoke tests using a fake board.
- Cross-language parity test: JS, Python, Ruby, Java, Lua builders all emit the
  same blink bytecode.

## Future Extensions

- Generated SDK codecs from a compact schema.
- Browser WebSerial client.
- Notebook integrations for Python and JS.
- Graphical pin explorer.
- Timeline view of events/logs returned by the board.
- Source maps from host code to bytecode offsets for debugging.
