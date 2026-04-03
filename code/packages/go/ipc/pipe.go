// Package ipc implements three classic Inter-Process Communication mechanisms:
// pipes, message queues, and shared memory.
//
// Processes are isolated by design — each has its own virtual address space,
// file descriptors, and registers. IPC is the set of mechanisms the OS
// provides for processes to exchange data despite their isolation.
package ipc

import "errors"

// ErrBrokenPipe is returned when writing to a pipe that has no readers.
//
// In Unix, this corresponds to the EPIPE error and the SIGPIPE signal.
// If nobody will ever read the data, there is no point in writing it —
// the pipe is "broken."
var ErrBrokenPipe = errors.New("broken pipe: no readers attached")

// ErrWriteEndClosed is returned when writing to a pipe whose write end
// has been explicitly closed.
var ErrWriteEndClosed = errors.New("write end is closed")

// Pipe is a unidirectional byte stream backed by a circular buffer.
//
// Think of a pipe as a garden hose: water goes in one end and comes out
// the other. You can't send water backwards, and the hose has a fixed
// capacity. If you push more water than it can hold, you wait until some
// drains out the other end.
//
// # Circular Buffer
//
// The pipe uses a circular (ring) buffer to store data in transit. Two
// pointers chase each other around the ring:
//
//	+---------+
//	| a b c . |   readPos=0, writePos=3, count=3
//	+---------+
//	  ^     ^
//	  R     W
//
// After reading 2 bytes ("ab"):
//
//	+---------+
//	| . . c . |   readPos=2, writePos=3, count=1
//	+---------+
//	      ^ ^
//	      R W
//
// When writePos reaches the end of the array, it wraps to index 0.
// This lets us reuse space without ever shifting elements.
//
// # Reference Counts
//
//   - When all writers close: the pipe is at EOF. A reader that finds an
//     empty buffer knows no more data will ever arrive.
//   - When all readers close: the pipe is "broken." A writer gets
//     ErrBrokenPipe — nobody is listening.
type Pipe struct {
	buffer     []byte
	capacity   int
	readPos    int
	writePos   int
	count      int  // bytes currently in buffer
	readers    int  // reference count of read ends
	writers    int  // reference count of write ends
	closedRead  bool
	closedWrite bool
}

// NewPipe creates a new pipe with the given capacity.
//
// The default convention in Unix is 4096 bytes (one memory page), but
// any positive integer works.
func NewPipe(capacity int) *Pipe {
	result, _ := StartNew[*Pipe]("ipc.NewPipe", nil,
		func(op *Operation[*Pipe], rf *ResultFactory[*Pipe]) *OperationResult[*Pipe] {
			op.AddProperty("capacity", capacity)
			return rf.Generate(true, false, &Pipe{
				buffer:   make([]byte, capacity),
				capacity: capacity,
				readers:  1,
				writers:  1,
			})
		}).GetResult()
	return result
}

// Write pushes data into the pipe, returning the number of bytes written.
//
// Behavior:
//
//	+-------------------+------------------+---------------------------+
//	| Readers alive?    | Buffer has space? | Action                    |
//	+===================+==================+===========================+
//	| No                | (any)            | Return ErrBrokenPipe      |
//	| Yes               | Yes              | Write as much as fits     |
//	| Yes               | No (full)        | Return 0 (would block)    |
//	+-------------------+------------------+---------------------------+
//
// In a real OS, the "would block" case suspends the process. In our
// simulation, we write what fits and return the count.
func (p *Pipe) Write(data []byte) (int, error) {
	type writeResult struct {
		n   int
		err error
	}
	result, _ := StartNew[writeResult]("ipc.Pipe.Write", writeResult{},
		func(op *Operation[writeResult], rf *ResultFactory[writeResult]) *OperationResult[writeResult] {
			// Guard: broken pipe — no readers
			if p.closedRead || p.readers <= 0 {
				return rf.Generate(true, false, writeResult{0, ErrBrokenPipe})
			}

			// Guard: write end closed
			if p.closedWrite {
				return rf.Generate(true, false, writeResult{0, ErrWriteEndClosed})
			}

			// Calculate how many bytes we can write
			toWrite := len(data)
			if space := p.Space(); toWrite > space {
				toWrite = space
			}
			if toWrite == 0 {
				return rf.Generate(true, false, writeResult{0, nil})
			}

			// Copy bytes into the circular buffer.
			// We may need two chunks if data wraps around the end of the array.
			for i := 0; i < toWrite; i++ {
				p.buffer[p.writePos] = data[i]
				p.writePos = (p.writePos + 1) % p.capacity
			}

			p.count += toWrite
			return rf.Generate(true, false, writeResult{toWrite, nil})
		}).GetResult()
	return result.n, result.err
}

// Read pulls up to count bytes from the pipe.
//
// Behavior:
//
//	+-------------------+------------------+---------------------------+
//	| Writers alive?    | Buffer has data? | Action                    |
//	+===================+==================+===========================+
//	| (any)             | Yes              | Read available bytes      |
//	| Yes               | No (empty)       | Return nil (would block)  |
//	| No                | No (empty)       | Return nil (EOF)          |
//	+-------------------+------------------+---------------------------+
//
// The caller checks IsEOF() to distinguish "no data yet" from "pipe done."
func (p *Pipe) Read(count int) []byte {
	result, _ := StartNew[[]byte]("ipc.Pipe.Read", nil,
		func(op *Operation[[]byte], rf *ResultFactory[[]byte]) *OperationResult[[]byte] {
			op.AddProperty("count", count)
			if p.closedRead {
				return rf.Generate(true, false, nil)
			}

			toRead := count
			if toRead > p.count {
				toRead = p.count
			}
			if toRead == 0 {
				return rf.Generate(true, false, nil)
			}

			result := make([]byte, toRead)
			for i := 0; i < toRead; i++ {
				result[i] = p.buffer[p.readPos]
				p.readPos = (p.readPos + 1) % p.capacity
			}

			p.count -= toRead
			return rf.Generate(true, false, result)
		}).GetResult()
	return result
}

// CloseRead closes the read end of the pipe. Any subsequent write
// will return ErrBrokenPipe.
func (p *Pipe) CloseRead() {
	_, _ = StartNew[struct{}]("ipc.Pipe.CloseRead", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			p.readers = 0
			p.closedRead = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// CloseWrite closes the write end of the pipe. Once the buffer drains,
// readers will see EOF. This is how shell pipelines terminate.
func (p *Pipe) CloseWrite() {
	_, _ = StartNew[struct{}]("ipc.Pipe.CloseWrite", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			p.writers = 0
			p.closedWrite = true
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
}

// IsEmpty returns true if no data is in the buffer.
func (p *Pipe) IsEmpty() bool {
	result, _ := StartNew[bool]("ipc.Pipe.IsEmpty", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, p.count == 0)
		}).GetResult()
	return result
}

// IsFull returns true if the buffer is at capacity.
func (p *Pipe) IsFull() bool {
	result, _ := StartNew[bool]("ipc.Pipe.IsFull", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, p.count == p.capacity)
		}).GetResult()
	return result
}

// Available returns the number of bytes available to read.
func (p *Pipe) Available() int {
	result, _ := StartNew[int]("ipc.Pipe.Available", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, p.count)
		}).GetResult()
	return result
}

// Space returns the number of bytes of free space for writing.
func (p *Pipe) Space() int {
	result, _ := StartNew[int]("ipc.Pipe.Space", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, p.capacity-p.count)
		}).GetResult()
	return result
}

// IsEOF returns true if no writers remain and the buffer is empty.
//
// This is the definitive "pipe is done" signal. In a shell pipeline like
// ls | grep foo, EOF occurs when ls exits (closing its write end) and
// grep has read all remaining buffered data.
func (p *Pipe) IsEOF() bool {
	result, _ := StartNew[bool]("ipc.Pipe.IsEOF", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, (p.writers <= 0 || p.closedWrite) && p.count == 0)
		}).GetResult()
	return result
}

// Capacity returns the fixed capacity of this pipe's buffer.
func (p *Pipe) Capacity() int {
	result, _ := StartNew[int]("ipc.Pipe.Capacity", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, p.capacity)
		}).GetResult()
	return result
}
