package blockram

// =========================================================================
// Configurable Block RAM — FPGA-style Memory with Reconfigurable Aspect Ratio
// =========================================================================
//
// # What is Block RAM?
//
// In an FPGA, Block RAM (BRAM) tiles are dedicated memory blocks separate
// from the configurable logic. Each tile has a fixed total storage but can
// be configured with different width/depth ratios.
//
// This file wraps DualPortRAM with reconfiguration support.

import "fmt"

// ConfigurableBRAM is a Block RAM with configurable aspect ratio.
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
func NewConfigurableBRAM(totalBits, width int) *ConfigurableBRAM {
	result, _ := StartNew[*ConfigurableBRAM]("block-ram.NewConfigurableBRAM", nil,
		func(op *Operation[*ConfigurableBRAM], rf *ResultFactory[*ConfigurableBRAM]) *OperationResult[*ConfigurableBRAM] {
			op.AddProperty("totalBits", totalBits)
			op.AddProperty("width", width)
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
			bram := &ConfigurableBRAM{
				totalBits: totalBits,
				width:     width,
				depth:     depth,
				ram:       NewDualPortRAM(depth, width, ReadFirst, ReadFirst),
				prevClock: 0,
				lastReadA: make([]int, width),
				lastReadB: make([]int, width),
			}
			return rf.Generate(true, false, bram)
		}).GetResult()
	return result
}

// Reconfigure changes the aspect ratio. Clears all stored data.
// Panics if width < 1 or does not evenly divide totalBits.
func (b *ConfigurableBRAM) Reconfigure(width int) {
	_, _ = StartNew[struct{}]("block-ram.ConfigurableBRAM.Reconfigure", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("width", width)
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
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// TickA performs a port A operation.
func (b *ConfigurableBRAM) TickA(clock, address int, dataIn []int, writeEnable int) []int {
	result, _ := StartNew[[]int]("block-ram.ConfigurableBRAM.TickA", nil,
		func(op *Operation[[]int], rf *ResultFactory[[]int]) *OperationResult[[]int] {
			op.AddProperty("clock", clock)
			op.AddProperty("address", address)
			op.AddProperty("writeEnable", writeEnable)
			validateBit(clock, "clock")
			zeros := make([]int, b.width)
			outA, _, _ := b.ram.Tick(
				clock,
				address, dataIn, writeEnable,
				0, zeros, 0,
			)
			return rf.Generate(true, false, outA)
		}).GetResult()
	return result
}

// TickB performs a port B operation.
func (b *ConfigurableBRAM) TickB(clock, address int, dataIn []int, writeEnable int) []int {
	result, _ := StartNew[[]int]("block-ram.ConfigurableBRAM.TickB", nil,
		func(op *Operation[[]int], rf *ResultFactory[[]int]) *OperationResult[[]int] {
			op.AddProperty("clock", clock)
			op.AddProperty("address", address)
			op.AddProperty("writeEnable", writeEnable)
			validateBit(clock, "clock")
			zeros := make([]int, b.width)
			_, outB, _ := b.ram.Tick(
				clock,
				0, zeros, 0,
				address, dataIn, writeEnable,
			)
			return rf.Generate(true, false, outB)
		}).GetResult()
	return result
}

// Depth returns the number of addressable words at current configuration.
func (b *ConfigurableBRAM) Depth() int {
	result, _ := StartNew[int]("block-ram.ConfigurableBRAM.Depth", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, b.depth)
		}).GetResult()
	return result
}

// Width returns the bits per word at current configuration.
func (b *ConfigurableBRAM) Width() int {
	result, _ := StartNew[int]("block-ram.ConfigurableBRAM.Width", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, b.width)
		}).GetResult()
	return result
}

// TotalBits returns the total storage capacity in bits (fixed).
func (b *ConfigurableBRAM) TotalBits() int {
	result, _ := StartNew[int]("block-ram.ConfigurableBRAM.TotalBits", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, b.totalBits)
		}).GetResult()
	return result
}
