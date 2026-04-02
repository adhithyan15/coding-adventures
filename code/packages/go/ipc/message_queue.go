package ipc

// MessageQueue is a FIFO queue of typed messages.
//
// While pipes transmit raw bytes (the reader must know how to parse them),
// message queues transmit discrete, typed messages. Each message has a type
// tag and a body, and the receiver can filter by type.
//
// # Analogy
//
// Think of a message queue as a shared mailbox in a hallway. Anyone can
// drop off an envelope with a label ("type 1: request", "type 2: status"),
// and anyone can pick up envelopes. You can ask for "any envelope" or
// "only type-2 envelopes."
//
// # Message Structure
//
// Each message is a (type, data) pair:
//
//	+----------+---------------------------------------------------+
//	| msg_type | Positive integer identifying the message kind.     |
//	| data     | The payload — up to MaxMessageSize bytes.          |
//	+----------+---------------------------------------------------+
//
// # Queue Limits
//
// Two limits prevent unbounded resource consumption:
//   - MaxMessages (default 256): queue is full when this many messages
//     are pending. Send returns false.
//   - MaxMessageSize (default 4096): messages larger than this are
//     rejected immediately.
//
// # Send/Receive Semantics
//
//	Send(type, data):
//	  1. Validate len(data) <= MaxMessageSize
//	  2. If queue full: return false
//	  3. Append (type, data) to the back of the FIFO
//
//	Receive(msgType):
//	  msgType == 0: dequeue the oldest message of ANY type
//	  msgType > 0:  find and remove the oldest message matching that type
//	  No match:     return (0, nil, false)
type MessageQueue struct {
	messages       []message
	MaxMessages    int
	MaxMessageSize int
}

// message is an internal struct representing a single queued message.
type message struct {
	MsgType int
	Data    []byte
}

// NewMessageQueue creates a new message queue with the given limits.
func NewMessageQueue(maxMessages, maxMessageSize int) *MessageQueue {
	result, _ := StartNew[*MessageQueue]("ipc.NewMessageQueue", nil,
		func(op *Operation[*MessageQueue], rf *ResultFactory[*MessageQueue]) *OperationResult[*MessageQueue] {
			op.AddProperty("maxMessages", maxMessages)
			op.AddProperty("maxMessageSize", maxMessageSize)
			return rf.Generate(true, false, &MessageQueue{
				messages:       make([]message, 0),
				MaxMessages:    maxMessages,
				MaxMessageSize: maxMessageSize,
			})
		}).GetResult()
	return result
}

// Send adds a message to the queue.
//
// Returns true if the message was enqueued successfully. Returns false if:
//   - The queue is full (at MaxMessages capacity)
//   - The message body exceeds MaxMessageSize
//   - msgType is not positive
//
// In a real OS, the "queue full" case would block the sender. In our
// simulation we return false.
func (mq *MessageQueue) Send(msgType int, data []byte) bool {
	result, _ := StartNew[bool]("ipc.MessageQueue.Send", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			op.AddProperty("msgType", msgType)
			// Validate message type — must be positive
			if msgType <= 0 {
				return rf.Generate(true, false, false)
			}

			// Validate message size
			if len(data) > mq.MaxMessageSize {
				return rf.Generate(true, false, false)
			}

			// Check capacity
			if len(mq.messages) >= mq.MaxMessages {
				return rf.Generate(true, false, false)
			}

			// Enqueue — append to the back of the FIFO
			mq.messages = append(mq.messages, message{MsgType: msgType, Data: data})
			return rf.Generate(true, false, true)
		}).GetResult()
	return result
}

// Receive removes and returns a message from the queue.
//
// If msgType == 0, the oldest message of any type is returned.
// If msgType > 0, the oldest message with that exact type is returned,
// skipping (and preserving) non-matching messages.
//
// Returns (msgType, data, true) on success, or (0, nil, false) if no
// matching message exists.
//
// Example of type filtering:
//
//	Queue: (1, "req1"), (2, "status"), (1, "req2")
//	Receive(2) → (2, "status")
//	Queue is now: (1, "req1"), (1, "req2")
func (mq *MessageQueue) Receive(msgType int) (int, []byte, bool) {
	type receiveResult struct {
		msgType int
		data    []byte
		ok      bool
	}
	result, _ := StartNew[receiveResult]("ipc.MessageQueue.Receive", receiveResult{},
		func(op *Operation[receiveResult], rf *ResultFactory[receiveResult]) *OperationResult[receiveResult] {
			op.AddProperty("msgType", msgType)
			if len(mq.messages) == 0 {
				return rf.Generate(true, false, receiveResult{0, nil, false})
			}

			if msgType == 0 {
				// Any type: dequeue the oldest message
				msg := mq.messages[0]
				mq.messages = mq.messages[1:]
				return rf.Generate(true, false, receiveResult{msg.MsgType, msg.Data, true})
			}

			// Filtered receive: find first matching type.
			// Like sorting through a mailbox looking for a specific label.
			for i, msg := range mq.messages {
				if msg.MsgType == msgType {
					mq.messages = append(mq.messages[:i], mq.messages[i+1:]...)
					return rf.Generate(true, false, receiveResult{msg.MsgType, msg.Data, true})
				}
			}

			return rf.Generate(true, false, receiveResult{0, nil, false})
		}).GetResult()
	return result.msgType, result.data, result.ok
}

// MessageCount returns the number of messages currently in the queue.
func (mq *MessageQueue) MessageCount() int {
	result, _ := StartNew[int]("ipc.MessageQueue.MessageCount", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			return rf.Generate(true, false, len(mq.messages))
		}).GetResult()
	return result
}

// IsEmpty returns true if the queue has no messages.
func (mq *MessageQueue) IsEmpty() bool {
	result, _ := StartNew[bool]("ipc.MessageQueue.IsEmpty", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, len(mq.messages) == 0)
		}).GetResult()
	return result
}

// IsFull returns true if the queue is at MaxMessages capacity.
func (mq *MessageQueue) IsFull() bool {
	result, _ := StartNew[bool]("ipc.MessageQueue.IsFull", false,
		func(op *Operation[bool], rf *ResultFactory[bool]) *OperationResult[bool] {
			return rf.Generate(true, false, len(mq.messages) >= mq.MaxMessages)
		}).GetResult()
	return result
}
