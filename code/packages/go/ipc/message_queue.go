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
	return &MessageQueue{
		messages:       make([]message, 0),
		MaxMessages:    maxMessages,
		MaxMessageSize: maxMessageSize,
	}
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
	// Validate message type — must be positive
	if msgType <= 0 {
		return false
	}

	// Validate message size
	if len(data) > mq.MaxMessageSize {
		return false
	}

	// Check capacity
	if len(mq.messages) >= mq.MaxMessages {
		return false
	}

	// Enqueue — append to the back of the FIFO
	mq.messages = append(mq.messages, message{MsgType: msgType, Data: data})
	return true
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
	if len(mq.messages) == 0 {
		return 0, nil, false
	}

	if msgType == 0 {
		// Any type: dequeue the oldest message
		msg := mq.messages[0]
		mq.messages = mq.messages[1:]
		return msg.MsgType, msg.Data, true
	}

	// Filtered receive: find first matching type.
	// Like sorting through a mailbox looking for a specific label.
	for i, msg := range mq.messages {
		if msg.MsgType == msgType {
			mq.messages = append(mq.messages[:i], mq.messages[i+1:]...)
			return msg.MsgType, msg.Data, true
		}
	}

	return 0, nil, false
}

// MessageCount returns the number of messages currently in the queue.
func (mq *MessageQueue) MessageCount() int {
	return len(mq.messages)
}

// IsEmpty returns true if the queue has no messages.
func (mq *MessageQueue) IsEmpty() bool {
	return len(mq.messages) == 0
}

// IsFull returns true if the queue is at MaxMessages capacity.
func (mq *MessageQueue) IsFull() bool {
	return len(mq.messages) >= mq.MaxMessages
}
