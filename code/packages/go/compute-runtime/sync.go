package computeruntime

// Synchronization primitives -- Fence, Semaphore, Event.
//
// # The Synchronization Problem
//
// CPUs and GPUs run asynchronously. When you submit a command buffer, the
// CPU does not wait -- it immediately returns and can do other work (or submit
// more GPU work). But at some point you need to know:
//
//	"Has the GPU finished yet?"
//
// That is what synchronization primitives solve.
//
// # Three Levels of Synchronization
//
//	+-------------------------------------------------------------+
//	|  FENCE (CPU <-> GPU)                                         |
//	|                                                              |
//	|  CPU submits work with a fence attached, then calls          |
//	|  fence.Wait() to block until the GPU signals it.             |
//	|                                                              |
//	|  CPU:  [submit(fence=F)]----------[F.Wait()]--[read results] |
//	|  GPU:  ----------[execute]--[signal F]                       |
//	|                                                              |
//	|  Use case: "wait for my kernel to finish so I can read"      |
//	+-------------------------------------------------------------+
//
//	+-------------------------------------------------------------+
//	|  SEMAPHORE (GPU Queue <-> GPU Queue)                         |
//	|                                                              |
//	|  Queue A signals a semaphore when its command buffer done.   |
//	|  Queue B waits on that semaphore before starting.            |
//	|                                                              |
//	|  Transfer Queue: [upload data]--[signal S]                   |
//	|  Compute Queue:  ---------------[wait S]--[run kernel]       |
//	|                                                              |
//	|  Use case: "compute waits for transfer to finish"            |
//	+-------------------------------------------------------------+
//
//	+-------------------------------------------------------------+
//	|  EVENT (GPU <-> GPU, fine-grained)                           |
//	|                                                              |
//	|  Set and waited on WITHIN command buffers.                   |
//	|                                                              |
//	|  CB: [dispatch A]--[set E]--[wait E]--[dispatch B]           |
//	|                                                              |
//	|  Use case: "barrier between two dispatches in the same CB"   |
//	+-------------------------------------------------------------+

// =========================================================================
// idGenerator -- shared auto-increment ID generator
// =========================================================================

// nextFenceID is a package-level counter for unique fence IDs.
var nextFenceID int

// nextSemaphoreID is a package-level counter for unique semaphore IDs.
var nextSemaphoreID int

// nextEventID is a package-level counter for unique event IDs.
var nextEventID int

// =========================================================================
// Fence -- CPU waits for GPU
// =========================================================================

// Fence is a CPU-to-GPU synchronization primitive.
//
// # Fence Lifecycle
//
//	create_fence(signaled=false)
//	    |
//	    v
//	[unsignaled] --submit(fence=F)--> [GPU working]
//	    ^                                    |
//	    |                              GPU finishes
//	    |                                    |
//	    +---- Reset() <-- [signaled] <-------+
//	                          |
//	                      Wait() returns
//
// You attach a fence to a queue submission. When the GPU finishes all
// the command buffers in that submission, it signals the fence. The CPU
// can then call Wait() to block until the signal arrives.
//
// Fences are reusable -- call Reset() to clear the signal, then attach
// to another submission.
type Fence struct {
	id         int
	signaled   bool
	waitCycles int
}

// NewFence creates a new fence.
//
// If signaled is true, the fence starts already signaled. This is useful
// when you need a fence for the first frame of a render loop -- the CPU
// needs to "wait" on a fence that has not been submitted yet, so it must
// start signaled.
func NewFence(signaled bool) *Fence {
	result, _ := StartNew[*Fence]("compute-runtime.NewFence", nil,
		func(op *Operation[*Fence], rf *ResultFactory[*Fence]) *OperationResult[*Fence] {
			op.AddProperty("signaled", signaled)
			id := nextFenceID
			nextFenceID++
			return rf.Generate(true, false, &Fence{
				id:       id,
				signaled: signaled,
			})
		}).GetResult()
	return result
}

// FenceID returns the unique identifier for this fence.
func (f *Fence) FenceID() int {
	result, _ := StartNew[int]("compute-runtime.Fence.FenceID", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, f.id)
		}).GetResult()
	return result
}

// Signaled returns whether the GPU has signaled this fence.
func (f *Fence) Signaled() bool {
	result, _ := StartNew[bool]("compute-runtime.Fence.Signaled", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, f.signaled)
		}).GetResult()
	return result
}

// WaitCycles returns the total cycles the CPU spent waiting on this fence.
func (f *Fence) WaitCycles() int {
	result, _ := StartNew[int]("compute-runtime.Fence.WaitCycles", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, f.waitCycles)
		}).GetResult()
	return result
}

// Signal signals the fence (called by the runtime when GPU finishes).
func (f *Fence) Signal() {
	_, _ = StartNew[struct{}]("compute-runtime.Fence.Signal", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			f.signaled = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Wait waits for the fence to be signaled.
//
// In a real system, this blocks the CPU thread. In our simulator,
// the fence is either already signaled (because we run synchronously)
// or it is not (which would be a programming error).
//
// Returns true if the fence was signaled, false if timeout expired.
func (f *Fence) Wait(timeoutCycles *int) bool {
	result, _ := StartNew[bool]("compute-runtime.Fence.Wait", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, f.signaled)
		}).GetResult()
	return result
}

// Reset resets the fence to unsignaled state for reuse.
func (f *Fence) Reset() {
	_, _ = StartNew[struct{}]("compute-runtime.Fence.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			f.signaled = false
			f.waitCycles = 0
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// =========================================================================
// Semaphore -- GPU-to-GPU synchronization
// =========================================================================

// Semaphore is a GPU queue-to-queue synchronization primitive.
//
// # How Semaphores Differ from Fences
//
// Fences are for CPU <-> GPU synchronization (CPU blocks until GPU done).
// Semaphores are for GPU <-> GPU synchronization between different queues.
//
// The CPU never waits on a semaphore -- they are entirely GPU-side.
//
// # Usage Pattern
//
//	// Transfer queue signals when upload is done
//	transferQueue.Submit([uploadCB], signalSemaphores=[sem])
//
//	// Compute queue waits for upload before starting kernel
//	computeQueue.Submit([computeCB], waitSemaphores=[sem])
type Semaphore struct {
	id       int
	signaled bool
}

// NewSemaphore creates a new semaphore in unsignaled state.
func NewSemaphore() *Semaphore {
	result, _ := StartNew[*Semaphore]("compute-runtime.NewSemaphore", nil,
		func(op *Operation[*Semaphore], rf *ResultFactory[*Semaphore]) *OperationResult[*Semaphore] {
			id := nextSemaphoreID
			nextSemaphoreID++
			return rf.Generate(true, false, &Semaphore{id: id})
		}).GetResult()
	return result
}

// SemaphoreID returns the unique identifier for this semaphore.
func (s *Semaphore) SemaphoreID() int {
	result, _ := StartNew[int]("compute-runtime.Semaphore.SemaphoreID", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, s.id)
		}).GetResult()
	return result
}

// Signaled returns whether this semaphore has been signaled.
func (s *Semaphore) Signaled() bool {
	result, _ := StartNew[bool]("compute-runtime.Semaphore.Signaled", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, s.signaled)
		}).GetResult()
	return result
}

// Signal signals the semaphore (called by runtime after queue completes).
func (s *Semaphore) Signal() {
	_, _ = StartNew[struct{}]("compute-runtime.Semaphore.Signal", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			s.signaled = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Reset resets to unsignaled (called by runtime when consumed by a wait).
func (s *Semaphore) Reset() {
	_, _ = StartNew[struct{}]("compute-runtime.Semaphore.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			s.signaled = false
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// =========================================================================
// Event -- fine-grained GPU-side synchronization
// =========================================================================

// Event is a fine-grained GPU-side synchronization primitive.
//
// # Events vs Barriers
//
// Pipeline barriers are implicit -- they are executed inline in a command
// buffer. Events are explicit -- you set them at one point and wait for
// them at another, potentially in a different command buffer or even
// from the CPU.
//
// # Usage Patterns
//
// GPU-side (in command buffer):
//
//	cb.CmdSetEvent(event, PipelineStageCompute)
//	cb.CmdWaitEvent(event, PipelineStageCompute, PipelineStageCompute)
//
// CPU-side:
//
//	event.Set()       // CPU signals
//	event.Status()    // CPU checks without blocking
//	event.Reset()     // CPU clears
type Event struct {
	id       int
	signaled bool
}

// NewEvent creates a new event in unsignaled state.
func NewEvent() *Event {
	result, _ := StartNew[*Event]("compute-runtime.NewEvent", nil,
		func(op *Operation[*Event], rf *ResultFactory[*Event]) *OperationResult[*Event] {
			id := nextEventID
			nextEventID++
			return rf.Generate(true, false, &Event{id: id})
		}).GetResult()
	return result
}

// EventID returns the unique identifier for this event.
func (e *Event) EventID() int {
	result, _ := StartNew[int]("compute-runtime.Event.EventID", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, e.id)
		}).GetResult()
	return result
}

// Signaled returns whether this event has been signaled.
func (e *Event) Signaled() bool {
	result, _ := StartNew[bool]("compute-runtime.Event.Signaled", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, e.signaled)
		}).GetResult()
	return result
}

// Set signals the event.
func (e *Event) Set() {
	_, _ = StartNew[struct{}]("compute-runtime.Event.Set", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			e.signaled = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Reset clears the event.
func (e *Event) Reset() {
	_, _ = StartNew[struct{}]("compute-runtime.Event.Reset", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			e.signaled = false
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// Status checks if signaled without blocking.
func (e *Event) Status() bool {
	result, _ := StartNew[bool]("compute-runtime.Event.Status", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, e.signaled)
		}).GetResult()
	return result
}
