package blockram

// =========================================================================
// RAM Modules — Synchronous Memory with Read/Write Ports
// =========================================================================
//
// # From Array to Module
//
// An SRAM array (sram.go) provides raw row-level read/write. A RAM module
// adds the interface that digital circuits actually use:
//
//  1. Address decoding — binary address bits select a row
//  2. Synchronous operation — reads and writes happen on clock edges
//  3. Read modes — what the output shows during a write operation
//  4. Dual-port access — two independent ports for simultaneous operations
//
// # Read Modes
//
//  1. Read-first: Output shows the OLD value at the address being written.
//  2. Write-first (read-after-write): Output shows the NEW value being written.
//  3. No-change: Output retains its previous value during writes.
//
// # Dual-Port RAM
//
// Two completely independent ports (A and B), each with its own address,
// data, and write enable. Both can operate simultaneously.

import "fmt"

// ReadMode controls what data_out shows during a write operation.
type ReadMode int

const (
	// ReadFirst: data_out = old value (read before write).
	ReadFirst ReadMode = iota
	// WriteFirst: data_out = new value (write before read).
	WriteFirst
	// NoChange: data_out = previous read value (output unchanged).
	NoChange
)

// WriteCollisionError is returned when both ports of a dual-port RAM
// write to the same address in the same clock cycle.
type WriteCollisionError struct {
	Address int
}

func (e *WriteCollisionError) Error() string {
	return fmt.Sprintf("blockram: write collision: both ports writing to address %d", e.Address)
}

// =========================================================================
// SinglePortRAM
// =========================================================================

// SinglePortRAM is a single-port synchronous RAM.
type SinglePortRAM struct {
	depth     int
	width     int
	readMode  ReadMode
	array     *SRAMArray
	prevClock int
	lastRead  []int
}

// NewSinglePortRAM creates a single-port synchronous RAM.
func NewSinglePortRAM(depth, width int, readMode ReadMode) *SinglePortRAM {
	result, _ := StartNew[*SinglePortRAM]("block-ram.NewSinglePortRAM", nil,
		func(op *Operation[*SinglePortRAM], rf *ResultFactory[*SinglePortRAM]) *OperationResult[*SinglePortRAM] {
			op.AddProperty("depth", depth)
			op.AddProperty("width", width)
			op.AddProperty("readMode", readMode)
			if depth < 1 {
				panic(fmt.Sprintf("blockram: SinglePortRAM depth must be >= 1, got %d", depth))
			}
			if width < 1 {
				panic(fmt.Sprintf("blockram: SinglePortRAM width must be >= 1, got %d", width))
			}
			lastRead := make([]int, width)
			ram := &SinglePortRAM{
				depth:     depth,
				width:     width,
				readMode:  readMode,
				array:     NewSRAMArray(depth, width),
				prevClock: 0,
				lastRead:  lastRead,
			}
			return rf.Generate(true, false, ram)
		}).GetResult()
	return result
}

// Tick executes one half-cycle. Operations happen on the rising edge (0→1).
func (r *SinglePortRAM) Tick(clock, address int, dataIn []int, writeEnable int) []int {
	result, _ := StartNew[[]int]("block-ram.SinglePortRAM.Tick", nil,
		func(op *Operation[[]int], rf *ResultFactory[[]int]) *OperationResult[[]int] {
			op.AddProperty("clock", clock)
			op.AddProperty("address", address)
			op.AddProperty("writeEnable", writeEnable)
			validateBit(clock, "clock")
			validateBit(writeEnable, "writeEnable")
			r.validateAddress(address)
			r.validateData(dataIn)

			risingEdge := r.prevClock == 0 && clock == 1
			r.prevClock = clock

			if !risingEdge {
				out := make([]int, r.width)
				copy(out, r.lastRead)
				return rf.Generate(true, false, out)
			}

			if writeEnable == 0 {
				r.lastRead = r.array.Read(address)
				out := make([]int, r.width)
				copy(out, r.lastRead)
				return rf.Generate(true, false, out)
			}

			switch r.readMode {
			case ReadFirst:
				r.lastRead = r.array.Read(address)
				r.array.Write(address, dataIn)
				out := make([]int, r.width)
				copy(out, r.lastRead)
				return rf.Generate(true, false, out)
			case WriteFirst:
				r.array.Write(address, dataIn)
				r.lastRead = make([]int, r.width)
				copy(r.lastRead, dataIn)
				out := make([]int, r.width)
				copy(out, r.lastRead)
				return rf.Generate(true, false, out)
			default: // NoChange
				r.array.Write(address, dataIn)
				out := make([]int, r.width)
				copy(out, r.lastRead)
				return rf.Generate(true, false, out)
			}
		}).GetResult()
	return result
}

// Depth returns the number of addressable words.
func (r *SinglePortRAM) Depth() int {
	result, _ := StartNew[int]("block-ram.SinglePortRAM.Depth", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, r.depth)
		}).GetResult()
	return result
}

// Width returns the bits per word.
func (r *SinglePortRAM) Width() int {
	result, _ := StartNew[int]("block-ram.SinglePortRAM.Width", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, r.width)
		}).GetResult()
	return result
}

// Dump returns all contents for inspection.
func (r *SinglePortRAM) Dump() [][]int {
	result, _ := StartNew[[][]int]("block-ram.SinglePortRAM.Dump", nil,
		func(op *Operation[[][]int], rf *ResultFactory[[][]int]) *OperationResult[[][]int] {
			out := make([][]int, r.depth)
			for i := 0; i < r.depth; i++ {
				out[i] = r.array.Read(i)
			}
			return rf.Generate(true, false, out)
		}).GetResult()
	return result
}

func (r *SinglePortRAM) validateAddress(address int) {
	if address < 0 || address >= r.depth {
		panic(fmt.Sprintf("blockram: address %d out of range [0, %d]", address, r.depth-1))
	}
}

func (r *SinglePortRAM) validateData(dataIn []int) {
	if len(dataIn) != r.width {
		panic(fmt.Sprintf("blockram: data_in length %d does not match width %d", len(dataIn), r.width))
	}
	for i, bit := range dataIn {
		validateBit(bit, fmt.Sprintf("dataIn[%d]", i))
	}
}

// =========================================================================
// DualPortRAM
// =========================================================================

// DualPortRAM is a true dual-port synchronous RAM.
type DualPortRAM struct {
	depth      int
	width      int
	readModeA  ReadMode
	readModeB  ReadMode
	array      *SRAMArray
	prevClock  int
	lastReadA  []int
	lastReadB  []int
}

// dualPortTickResult holds the ([]int, []int, error) result for Tick.
type dualPortTickResult struct {
	outA []int
	outB []int
	err  error
}

// NewDualPortRAM creates a true dual-port synchronous RAM.
func NewDualPortRAM(depth, width int, readModeA, readModeB ReadMode) *DualPortRAM {
	result, _ := StartNew[*DualPortRAM]("block-ram.NewDualPortRAM", nil,
		func(op *Operation[*DualPortRAM], rf *ResultFactory[*DualPortRAM]) *OperationResult[*DualPortRAM] {
			op.AddProperty("depth", depth)
			op.AddProperty("width", width)
			if depth < 1 {
				panic(fmt.Sprintf("blockram: DualPortRAM depth must be >= 1, got %d", depth))
			}
			if width < 1 {
				panic(fmt.Sprintf("blockram: DualPortRAM width must be >= 1, got %d", width))
			}
			ram := &DualPortRAM{
				depth:     depth,
				width:     width,
				readModeA: readModeA,
				readModeB: readModeB,
				array:     NewSRAMArray(depth, width),
				prevClock: 0,
				lastReadA: make([]int, width),
				lastReadB: make([]int, width),
			}
			return rf.Generate(true, false, ram)
		}).GetResult()
	return result
}

// Tick executes one half-cycle on both ports.
// Returns (dataOutA, dataOutB, error).
func (r *DualPortRAM) Tick(
	clock int,
	addressA int, dataInA []int, writeEnableA int,
	addressB int, dataInB []int, writeEnableB int,
) ([]int, []int, error) {
	res, err := StartNew[dualPortTickResult]("block-ram.DualPortRAM.Tick", dualPortTickResult{},
		func(op *Operation[dualPortTickResult], rf *ResultFactory[dualPortTickResult]) *OperationResult[dualPortTickResult] {
			op.AddProperty("clock", clock)
			op.AddProperty("addressA", addressA)
			op.AddProperty("addressB", addressB)
			op.AddProperty("writeEnableA", writeEnableA)
			op.AddProperty("writeEnableB", writeEnableB)
			validateBit(clock, "clock")
			validateBit(writeEnableA, "writeEnableA")
			validateBit(writeEnableB, "writeEnableB")
			r.validateAddress(addressA, "addressA")
			r.validateAddress(addressB, "addressB")
			r.validateData(dataInA, "dataInA")
			r.validateData(dataInB, "dataInB")

			risingEdge := r.prevClock == 0 && clock == 1
			r.prevClock = clock

			if !risingEdge {
				outA := make([]int, r.width)
				outB := make([]int, r.width)
				copy(outA, r.lastReadA)
				copy(outB, r.lastReadB)
				return rf.Generate(true, false, dualPortTickResult{outA: outA, outB: outB, err: nil})
			}

			if writeEnableA == 1 && writeEnableB == 1 && addressA == addressB {
				collisionErr := &WriteCollisionError{Address: addressA}
				return rf.Fail(dualPortTickResult{}, collisionErr)
			}

			outA := r.processPort(addressA, dataInA, writeEnableA, r.readModeA, r.lastReadA)
			r.lastReadA = outA
			outB := r.processPort(addressB, dataInB, writeEnableB, r.readModeB, r.lastReadB)
			r.lastReadB = outB

			resultA := make([]int, r.width)
			resultB := make([]int, r.width)
			copy(resultA, outA)
			copy(resultB, outB)
			return rf.Generate(true, false, dualPortTickResult{outA: resultA, outB: resultB, err: nil})
		}).GetResult()
	if err != nil {
		return nil, nil, err
	}
	return res.outA, res.outB, res.err
}

// processPort handles a single port operation.
func (r *DualPortRAM) processPort(
	address int, dataIn []int, writeEnable int,
	readMode ReadMode, lastRead []int,
) []int {
	if writeEnable == 0 {
		return r.array.Read(address)
	}

	switch readMode {
	case ReadFirst:
		result := r.array.Read(address)
		r.array.Write(address, dataIn)
		return result
	case WriteFirst:
		r.array.Write(address, dataIn)
		out := make([]int, r.width)
		copy(out, dataIn)
		return out
	default: // NoChange
		r.array.Write(address, dataIn)
		out := make([]int, r.width)
		copy(out, lastRead)
		return out
	}
}

// Depth returns the number of addressable words.
func (r *DualPortRAM) Depth() int {
	result, _ := StartNew[int]("block-ram.DualPortRAM.Depth", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, r.depth)
		}).GetResult()
	return result
}

// Width returns the bits per word.
func (r *DualPortRAM) Width() int {
	result, _ := StartNew[int]("block-ram.DualPortRAM.Width", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, r.width)
		}).GetResult()
	return result
}

func (r *DualPortRAM) validateAddress(address int, name string) {
	if address < 0 || address >= r.depth {
		panic(fmt.Sprintf("blockram: %s %d out of range [0, %d]", name, address, r.depth-1))
	}
}

func (r *DualPortRAM) validateData(dataIn []int, name string) {
	if len(dataIn) != r.width {
		panic(fmt.Sprintf("blockram: %s length %d does not match width %d", name, len(dataIn), r.width))
	}
	for i, bit := range dataIn {
		validateBit(bit, fmt.Sprintf("%s[%d]", name, i))
	}
}
