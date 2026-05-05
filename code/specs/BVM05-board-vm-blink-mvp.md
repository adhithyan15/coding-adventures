# BVM05 - Board VM Blink MVP

## Overview

The blink MVP is the first end-to-end Board VM scenario. It proves the whole
idea with the smallest useful hardware behavior:

1. Connect to a board.
2. Query capabilities.
3. Upload bytecode that controls a GPIO pin.
4. Run it and observe an LED blink.
5. Eject it so the board can blink without the host.

This is the vertical slice that turns the architecture from a nice idea into a
thing that breathes.

## Scope

In scope:

- One supported board.
- One transport, preferably USB serial / UART.
- `HELLO`, `CAPS_QUERY`, bytecode upload, `RUN`, and one eject path.
- GPIO output.
- Millisecond sleep.
- Fake-board tests.
- One hardware smoke test.

Out of scope for this MVP:

- PWM.
- ADC.
- I2C/SPI.
- Event subscriptions.
- Multi-language SDKs beyond the reference host.
- Full graphical app.
- Secure wireless pairing.

## Required Capabilities

The target board must advertise:

| Capability | ID | Required operation |
|---|---:|---|
| `gpio.digital` | `0x01..0x04` | open output and write level |
| `time.sleep` | `0x10` | sleep for milliseconds |
| `program.ram_exec` | `0x7001` | upload and run bytecode |

For eject, it must support at least one:

| Eject support | Mechanism |
|---|---|
| `program.store` (`0x7002`) | store bytecode in nonvolatile memory |
| firmware embedding | host builds runtime with embedded bytecode |

## User Story

CLI flow:

```
$ bvm connect /dev/tty.usbmodem101
connected board=arduino runtime=board-vm-rust protocol=1

bvm> caps
gpio.digital@1
time.sleep@1
program.ram_exec@1
program.store@1

bvm> blink 13 --high-ms 250 --low-ms 250
uploaded program 1, 26 code bytes
running

bvm> eject blink --slot 0 --boot run
stored program 1 in slot 0
```

After reset, the board blinks the same LED without the host attached.

## Protocol Flow

### 1. Handshake

Host sends `HELLO`:

```
min_version = 1
max_version = 1
host_name = "board-vm-host"
host_nonce = random u32
```

Board replies `HELLO_ACK`:

```
selected_version = 1
board_name = "<board id>"
runtime_name = "board-vm-rust"
host_nonce = echoed
board_nonce = random u32
max_frame_payload = board-specific
```

### 2. Capabilities

Host sends `CAPS_QUERY`.

Board replies with at least:

```
gpio.digital@1
time.sleep@1
program.ram_exec@1
```

If there is a known onboard LED, the board descriptor should include it:

```
metadata:
  onboard_led_pin: 13
  pin_numbering: "arduino-digital"
```

The first version may expose this metadata as a descriptor extension string
until BVM01 grows a formal metadata section.

### 3. Upload Blink Program

Host builds the BVM02 bytecode:

```
12 0D       PUSH_U8 13
12 01       PUSH_U8 1
40 01       CALL_U8 gpio.open
20          DUP
11          PUSH_TRUE
40 02       CALL_U8 gpio.write
13 FA 00    PUSH_U16 250
40 10       CALL_U8 time.sleep_ms
20          DUP
10          PUSH_FALSE
40 02       CALL_U8 gpio.write
13 FA 00    PUSH_U16 250
40 10       CALL_U8 time.sleep_ms
30 EC       JUMP_S8 -20
```

Raw code bytes:

```
12 0D 12 01 40 01 20 11 40 02 13 FA 00 40 10 20
10 40 02 13 FA 00 40 10 30 EC
```

For upload, the host wraps the code in a BVM1 module with
`program_may_run_forever` set, `max_stack = 4`, and an empty constant pool.
That full module is 36 bytes:

```
42 56 4D 31 01 01 04 00 1A
12 0D 12 01 40 01 20 11 40 02 13 FA 00 40 10 20
10 40 02 13 FA 00 40 10 30 EC
00
```

Host sends:

```
PROGRAM_BEGIN(program_id=1, format=BVM_MODULE, total_len=36, crc32=...)
PROGRAM_CHUNK(program_id=1, offset=0, bytes=module)
PROGRAM_END(program_id=1)
```

The board validates and acknowledges the program.

### 4. Run

Host sends:

```
RUN(
  program_id = 1,
  flags = background_run | reset_vm_before_run,
  instruction_budget = 1000,
  time_budget_ms = 0
)
```

`time_budget_ms = 0` means no wall-clock limit for background execution.

Board replies:

```
RUN_REPORT(
  status = running,
  instructions_executed = small nonzero count,
  open_handles = 1
)
```

The LED should now blink.

### 5. Stop

Host sends `STOP`.

Board closes the GPIO handle, stops the program, and replies with a final
`RUN_REPORT(status=stopped)`.

### 6. Eject

If the board supports storage:

```
STORE_PROGRAM(program_id=1, slot=0, boot_policy=run_at_boot)
```

If the board does not support storage, the host creates a firmware artifact:

```
blink/
  manifest.json
  blink.bvm
  firmware.<board-specific extension>
```

## Fake Board Expected Trace

Running the blink bytecode for one loop on a fake board should produce:

```
gpio.open(pin=13, mode=output) -> handle 1
gpio.write(handle=1, level=true)
time.sleep_ms(250)
gpio.write(handle=1, level=false)
time.sleep_ms(250)
jump(loop_start)
```

For tests, the fake clock should not actually sleep. It records requested
durations and advances virtual time.

## Implementation Milestones

### Milestone 1: bytecode only

- Implement BVM02 decoder.
- Implement fake runtime with GPIO and clock.
- Execute blink bytecode in memory.
- Assert fake trace.

### Milestone 2: protocol loopback

- Implement BVM01 raw frame codec.
- Implement COBS + CRC.
- Upload blink over in-memory transport.
- Run blink against fake board through request/response messages.

### Milestone 3: hardware foreground

- Flash runtime firmware manually.
- Connect over serial.
- Handshake and capability query.
- Upload one-shot GPIO high/low programs.

### Milestone 4: hardware background blink

- Upload the 26-byte blink loop.
- Run in background.
- Verify board still responds to `PING` and `STOP`.

### Milestone 5: eject

- Store bytecode on board, or generate embedded firmware.
- Reset board.
- Verify blink starts without host commands.

## Public Test Fixtures

Create:

```
code/fixtures/board-vm/bytecode/blink.hex
code/fixtures/board-vm/bytecode/blink.json
code/fixtures/board-vm/protocol/run-blink-session.json
```

`blink.json`:

```json
{
  "name": "blink",
  "pin": 13,
  "high_ms": 250,
  "low_ms": 250,
  "code_hex": "120d120140012011400213fa0040102010400213fa00401030ec",
  "module_hex": "42564d31010104001a120d120140012011400213fa0040102010400213fa00401030ec00"
}
```

## Done Criteria

- Clean repo build/tests for the new Rust packages.
- Fake-board blink test passes.
- Protocol loopback blink test passes.
- A supported physical board blinks an LED through uploaded bytecode.
- `STOP` recovers control without reflashing.
- Eject path produces either stored boot behavior or a flashable firmware
  artifact.
- Specs BVM00 through BVM05 are updated with any discoveries from hardware.

## Future Extensions

- `motor` helper built on GPIO + PWM.
- `tone` helper built on PWM.
- `watch` command for pin-change events.
- Browser REPL using WebSerial.
- Multi-language SDK parity for blink.
