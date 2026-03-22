package blockram

// =========================================================================
// Configurable Block RAM — FPGA-style Memory with Reconfigurable Aspect Ratio
// =========================================================================
//
// # What is Block RAM?
//
// In an FPGA, Block RAM (BRAM) tiles are dedicated memory blocks separate
// from the configurable logic. Each tile has a fixed total storage (typically
// 18 Kbit or 36 Kbit) but can be configured with different width/depth ratios:
//
//	18 Kbit BRAM configurations:
//	┌───────────────┬───────┬───────┬────────────┐
//	│ Configuration │ Depth │ Width │ Total bits │
//	├───────────────┼───────┼───────┼────────────┤
//	│ 16K x 1       │ 16384 │     1 │      16384 │
//	│  8K x 2       │  8192 │     2 │      16384 │
//	│  4K x 4       │  4096 │     4 │      16384 │
//	│  2K x 8       │  2048 │     8 │      16384 │
//	│  1K x 16      │  1024 │    16 │      16384 │
//	│ 512 x 32      │   512 │    32 │      16384 │
//	└───────────────┴───────┴───────┴────────────┘
//
// The total storage is fixed; you trade depth for width by changing how
// the address decoder and column MUX are configured. The underlying SRAM
// cells do not change — only the access pattern changes.
//
// This file wraps DualPortRAM with reconfiguration support.

import "fmt"

// ConfigurableBRAM is a Block RAM with configurable aspect ratio.
//
// Total storage is fixed at initialization. Width and depth can be
// reconfigured as long as width * depth == total_bits.
//
// Supports dual-port access (port A and port B).
type ConfigurableBRAM struct {
	totalBits int
	width     int
	depth     int
	ram       *DualPortRAM
	prevClock int
	lastReadA []int
	lastReadB []int
}

// NewConfigurableBRAM creates a Block RAM with the given total capacity
// and initial word width.
//
// Parameters:
//   - totalBits: total storage in bits (default would be 18432 = 18 Kbit)
//   - width: initial bits per word
//
// Panics if totalBits < 1, width < 1, or width does not evenly divide totalBits.
func NewConfigurableBRAM(totalBits, width int) *ConfigurableBRAM {
	if totalBits < 1 {
		panic(fmt.Sprintf("blockram: ConfigurableBRAM totalBits must be >= 1, got %d", totalBits))
	}
	if width < 1 {
		panic(fmt.Sprintf("blockram: ConfigurableBRAM width must be >= 1, got %d", width))
	}
	if totalBits%width != 0 {
		panic(fmt.Sprintf("blockram: width %d does not evenly divide totalBits %d", width, totalBits))
	}

	depth := totalBits / width
	return &ConfigurableBRAM{
		totalBits: totalBits,
		width:     width,
		depth:     depth,
		ram:       NewDualPortRAM(depth, width, ReadFirst, ReadFirst),
		prevClock: 0,
		lastReadA: make([]int, width),
		lastReadB: make([]int, width),
	}
}

// Reconfigure changes the aspect ratio. Clears all stored data.
//
// Panics if width < 1 or does not evenly divide totalBits.
func (b *ConfigurableBRAM) Reconfigure(width int) {
	if width < 1 {
		panic(fmt.Sprintf("blockram: ConfigurableBRAM width must be >= 1, got %d", width))
	}
	if b.totalBits%width != 0 {
		panic(fmt.Sprintf("blockram: width %d does not evenly divide totalBits %d", width, b.totalBits))
	}

	b.width = width
	b.depth = b.totalBits / width
	b.ram = NewDualPortRAM(b.depth, b.width, ReadFirst, ReadFirst)
	b.prevClock = 0
	b.lastReadA = make([]int, width)
	b.lastReadB = make([]int, width)
}

// TickA performs a port A operation.
//
// Parameters:
//   - clock: clock signal (0 or 1)
//   - address: word address (0 to depth-1)
//   - dataIn: write data (slice of width bits)
//   - writeEnable: 0 = read, 1 = write
//
// Returns data_out: slice of width bits.
func (b *ConfigurableBRAM) TickA(clock, address int, dataIn []int, writeEnable int) []int {
	validateBit(clock, "clock")

	// Use the dual-port RAM with port B idle (read address 0)
	zeros := make([]int, b.width)
	outA, _, _ := b.ram.Tick(
		clock,
		address, dataIn, writeEnable,
		0, zeros, 0,
	)
	return outA
}

// TickB performs a port B operation.
//
// Parameters:
//   - clock: clock signal (0 or 1)
//   - address: word address (0 to depth-1)
//   - dataIn: write data (slice of width bits)
//   - writeEnable: 0 = read, 1 = write
//
// Returns data_out: slice of width bits.
func (b *ConfigurableBRAM) TickB(clock, address int, dataIn []int, writeEnable int) []int {
	validateBit(clock, "clock")

	// Use the dual-port RAM with port A idle
	zeros := make([]int, b.width)
	_, outB, _ := b.ram.Tick(
		clock,
		0, zeros, 0,
		address, dataIn, writeEnable,
	)
	return outB
}

// Depth returns the number of addressable words at current configuration.
func (b *ConfigurableBRAM) Depth() int { return b.depth }

// Width returns the bits per word at current configuration.
func (b *ConfigurableBRAM) Width() int { return b.width }

// TotalBits returns the total storage capacity in bits (fixed).
func (b *ConfigurableBRAM) TotalBits() int { return b.totalBits }
