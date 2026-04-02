package core

// =========================================================================
// MemoryController -- serializes memory requests from multiple cores
// =========================================================================

// MemoryController manages access to shared main memory from multiple cores.
//
// # Why a Memory Controller?
//
// In a multi-core system, multiple cores may request memory access in the
// same clock cycle. Real memory (DRAM) can only handle a limited number of
// concurrent requests, so the memory controller queues and serializes them.
//
// The memory controller is like a librarian at a busy library: patrons
// (cores) line up with their requests, and the librarian processes them
// one at a time, delivering books (data) after a delay (latency).
//
// # Latency Simulation
//
// Each memory request takes `latency` cycles to complete. The controller
// counts down the remaining cycles on each Tick(). When a request reaches
// zero remaining cycles, its data is delivered to the requester.
//
// # Memory Model
//
// The underlying memory is a flat byte array. Word reads/writes use
// little-endian byte ordering, matching modern ARM and x86 architectures.
type MemoryController struct {
	// memory is the raw byte storage (shared across all cores).
	memory []byte

	// latency is the number of cycles for a memory access to complete.
	latency int

	// pendingReads tracks in-flight read requests.
	pendingReads []memoryRequest

	// pendingWrites tracks in-flight write requests.
	pendingWrites []memoryWriteRequest

	// completedReads accumulates completed read results each tick.
	completedReads []MemoryReadResult
}

// memoryRequest is an in-flight read request.
type memoryRequest struct {
	Address      int // Starting byte address
	NumBytes     int // Number of bytes to read
	RequesterID  int // Which core submitted the request
	CyclesLeft   int // Cycles remaining until data is ready
}

// memoryWriteRequest is an in-flight write request.
type memoryWriteRequest struct {
	Address     int    // Starting byte address
	Data        []byte // Bytes to write
	RequesterID int    // Which core submitted the request
	CyclesLeft  int    // Cycles remaining until write completes
}

// MemoryReadResult is a completed read -- data delivered to a requester.
type MemoryReadResult struct {
	RequesterID int    // Which core receives this data
	Address     int    // The address that was read
	Data        []byte // The bytes that were read
}

// NewMemoryController creates a memory controller with the given backing
// memory and access latency.
//
// The memory slice is shared (not copied) -- multiple cores access the same
// underlying bytes. This models shared physical memory in a multi-core system.
func NewMemoryController(memory []byte, latency int) *MemoryController {
	result, _ := StartNew[*MemoryController]("core.NewMemoryController", nil,
		func(op *Operation[*MemoryController], rf *ResultFactory[*MemoryController]) *OperationResult[*MemoryController] {
			op.AddProperty("latency", latency)
			op.AddProperty("memory_size", len(memory))
			return rf.Generate(true, false, &MemoryController{
				memory:  memory,
				latency: latency,
			})
		}).GetResult()
	return result
}

// RequestRead submits a read request.
//
// The read will complete after `latency` cycles. Call Tick() each cycle
// and check the returned results for completed reads.
func (mc *MemoryController) RequestRead(address, numBytes, requesterID int) {
	_, _ = StartNew[struct{}]("core.MemoryController.RequestRead", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			op.AddProperty("num_bytes", numBytes)
			op.AddProperty("requester_id", requesterID)
			mc.pendingReads = append(mc.pendingReads, memoryRequest{
				Address:     address,
				NumBytes:    numBytes,
				RequesterID: requesterID,
				CyclesLeft:  mc.latency,
			})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// RequestWrite submits a write request.
//
// The write completes after `latency` cycles. The data is committed to
// memory when the request finishes (not immediately).
func (mc *MemoryController) RequestWrite(address int, data []byte, requesterID int) {
	_, _ = StartNew[struct{}]("core.MemoryController.RequestWrite", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			op.AddProperty("requester_id", requesterID)
			dataCopy := make([]byte, len(data))
			copy(dataCopy, data)
			mc.pendingWrites = append(mc.pendingWrites, memoryWriteRequest{
				Address:     address,
				Data:        dataCopy,
				RequesterID: requesterID,
				CyclesLeft:  mc.latency,
			})
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Tick advances the memory controller by one cycle.
//
// Decrements all pending request counters. When a request reaches zero
// remaining cycles, it is completed:
//   - Reads: data is copied from memory and returned in the result list
//   - Writes: data is committed to memory
//
// Returns a list of completed read results (requester ID + data).
func (mc *MemoryController) Tick() []MemoryReadResult {
	result, _ := StartNew[[]MemoryReadResult]("core.MemoryController.Tick", nil,
		func(op *Operation[[]MemoryReadResult], rf *ResultFactory[[]MemoryReadResult]) *OperationResult[[]MemoryReadResult] {
			mc.completedReads = mc.completedReads[:0]

			remaining := mc.pendingReads[:0]
			for i := range mc.pendingReads {
				mc.pendingReads[i].CyclesLeft--
				if mc.pendingReads[i].CyclesLeft <= 0 {
					req := mc.pendingReads[i]
					data := mc.readMemory(req.Address, req.NumBytes)
					mc.completedReads = append(mc.completedReads, MemoryReadResult{
						RequesterID: req.RequesterID,
						Address:     req.Address,
						Data:        data,
					})
				} else {
					remaining = append(remaining, mc.pendingReads[i])
				}
			}
			mc.pendingReads = remaining

			remainingWrites := mc.pendingWrites[:0]
			for i := range mc.pendingWrites {
				mc.pendingWrites[i].CyclesLeft--
				if mc.pendingWrites[i].CyclesLeft <= 0 {
					req := mc.pendingWrites[i]
					mc.writeMemory(req.Address, req.Data)
				} else {
					remainingWrites = append(remainingWrites, mc.pendingWrites[i])
				}
			}
			mc.pendingWrites = remainingWrites

			res := make([]MemoryReadResult, len(mc.completedReads))
			copy(res, mc.completedReads)
			return rf.Generate(true, false, res)
		}).GetResult()
	return result
}

// ReadWord reads a 32-bit word from memory at the given address.
// Little-endian byte order.
func (mc *MemoryController) ReadWord(address int) int {
	result, _ := StartNew[int]("core.MemoryController.ReadWord", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("address", address)
			if address < 0 || address+4 > len(mc.memory) {
				return rf.Generate(true, false, 0)
			}
			val := int(mc.memory[address]) |
				(int(mc.memory[address+1]) << 8) |
				(int(mc.memory[address+2]) << 16) |
				(int(mc.memory[address+3]) << 24)
			return rf.Generate(true, false, val)
		}).GetResult()
	return result
}

// WriteWord writes a 32-bit word to memory at the given address.
// Little-endian byte order.
func (mc *MemoryController) WriteWord(address int, value int) {
	_, _ = StartNew[struct{}]("core.MemoryController.WriteWord", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("address", address)
			if address < 0 || address+4 > len(mc.memory) {
				return rf.Generate(true, false, struct{}{})
			}
			mc.memory[address] = byte(value & 0xFF)
			mc.memory[address+1] = byte((value >> 8) & 0xFF)
			mc.memory[address+2] = byte((value >> 16) & 0xFF)
			mc.memory[address+3] = byte((value >> 24) & 0xFF)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// LoadProgram copies program bytes into memory starting at the given address.
func (mc *MemoryController) LoadProgram(program []byte, startAddress int) {
	_, _ = StartNew[struct{}]("core.MemoryController.LoadProgram", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			op.AddProperty("start_address", startAddress)
			op.AddProperty("program_size", len(program))
			if startAddress < 0 || startAddress+len(program) > len(mc.memory) {
				return rf.Generate(true, false, struct{}{})
			}
			copy(mc.memory[startAddress:], program)
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// MemorySize returns the total size of memory in bytes.
func (mc *MemoryController) MemorySize() int {
	result, _ := StartNew[int]("core.MemoryController.MemorySize", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(mc.memory))
		}).GetResult()
	return result
}

// PendingCount returns the number of in-flight requests.
func (mc *MemoryController) PendingCount() int {
	result, _ := StartNew[int]("core.MemoryController.PendingCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(mc.pendingReads)+len(mc.pendingWrites))
		}).GetResult()
	return result
}

// readMemory reads bytes from the backing memory array.
func (mc *MemoryController) readMemory(address, numBytes int) []byte {
	if address < 0 || address+numBytes > len(mc.memory) {
		return make([]byte, numBytes)
	}
	data := make([]byte, numBytes)
	copy(data, mc.memory[address:address+numBytes])
	return data
}

// writeMemory writes bytes to the backing memory array.
func (mc *MemoryController) writeMemory(address int, data []byte) {
	if address < 0 || address+len(data) > len(mc.memory) {
		return
	}
	copy(mc.memory[address:], data)
}
