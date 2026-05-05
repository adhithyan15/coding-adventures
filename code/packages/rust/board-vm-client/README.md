# board-vm-client

Transport-independent host client for Board VM sessions.

This crate turns the lower-level host frame builders into a blocking request /
response client over a tiny raw-frame transport trait. Serial, WebSerial, BLE,
TCP, and test loopbacks can all implement the same trait without changing the
session logic.
