/// Block RAM: SRAM cells, arrays, and configurable RAM modules.
///
/// The hierarchy follows real hardware: [SramCell] (1 bit) → [SramArray]
/// (2D grid) → [SinglePortRAM]/[DualPortRAM] (synchronous modules) →
/// [ConfigurableBRAM] (FPGA block RAM with reconfigurable aspect ratio).
library;

export 'src/bram.dart';
export 'src/ram.dart';
export 'src/sram.dart';
