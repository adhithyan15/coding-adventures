# board-vm-uart

Reusable UART-facing byte-stream layer for Board VM firmware.

The crate keeps UART concepts out of board-specific firmware binaries. Board
targets implement `BlockingUart`, then wrap the driver in `UartByteStream` to
serve `board-vm-device` COBS wire frames over that UART.

It intentionally does not know about USB CDC, host serial ports, Arduino pin
maps, or any particular MCU register block.
