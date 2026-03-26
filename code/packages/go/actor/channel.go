package actor

// Channel is a one-way, append-only, ordered log of messages.
//
// Channels connect message producers to message consumers. Messages flow in
// one direction only. Once appended, a message cannot be removed, modified,
// or reordered.
//
// # Analogy
//
// A Channel is a one-way pneumatic tube in an office building. Documents go
// in one end and come out the other. You cannot send documents backwards.
// The tube keeps a copy of every document that has ever passed through it
// (the log), and each office at the receiving end has a bookmark showing
// which documents they have already read (the offset).
//
// # Why One-Way?
//
// Bidirectional channels create ambiguity: "who sent this message?" and
// "can a receiver inject messages that look like they came from the sender?"
// One-way channels eliminate both questions. If you need bidirectional
// communication, use two channels — one in each direction.
//
// # Why Append-Only?
//
// If messages could be deleted or modified, crash recovery becomes impossible.
// After a crash, an append-only log gives a definitive answer: "here is
// exactly what happened, in order, immutably recorded."
//
// # Persistence
//
// Channels persist to disk as a binary append log using the Message wire
// format. Each message is written as header + envelope + payload, concatenated
// end-to-end. This format is:
//   - Binary-native — no Base64 bloat for images/videos
//   - Appendable — just write bytes at the end of the file
//   - Replayable — read from the beginning, parse header, skip payload, repeat
//   - Scannable — index all messages without loading payloads
//
// # Offset Tracking
//
// Each consumer independently tracks how far it has read. This is NOT managed
// by the channel — it is the consumer's responsibility. The channel is a dumb
// log; consumers are smart readers.
//
//	Channel log:   [m0] [m1] [m2] [m3] [m4] [m5]
//	                                     ^
//	Consumer A:    offset = 4 ───────────┘
//	                     ^
//	Consumer B:    offset = 1 (behind — maybe processing slowly)

import (
	"bytes"
	"fmt"
	"io"
	"path/filepath"
)

// Channel is a one-way, append-only, ordered message log.
//
// Fields:
//
//	+────────────+───────────────────────────────────────────────────+
//	| id         | Unique identifier (string).                      |
//	| name       | Human-readable name (e.g., "email-summaries").   |
//	| log        | Ordered list of Messages. Append-only.           |
//	| createdAt  | Timestamp when the channel was created.          |
//	+────────────+───────────────────────────────────────────────────+
type Channel struct {
	id        string
	name      string
	log       []*Message
	createdAt int64
}

// NewChannel creates a new empty channel with the given ID and name.
//
//	ch := NewChannel("ch_001", "email-summaries")
func NewChannel(id, name string) *Channel {
	ch, _ := StartNew[*Channel]("actor.NewChannel", nil,
		func(op *Operation[*Channel], rf *ResultFactory[*Channel]) *OperationResult[*Channel] {
			return rf.Generate(true, false, &Channel{
				id:        id,
				name:      name,
				log:       make([]*Message, 0),
				createdAt: op.Time.Now().UnixNano(),
			})
		}).GetResult()
	return ch
}

// ─────────────────────────────────────────────────────────────────────────────
// Getter Methods
// ─────────────────────────────────────────────────────────────────────────────

// ID returns the channel's unique identifier.
func (c *Channel) ID() string { return c.id }

// Name returns the channel's human-readable name.
func (c *Channel) Name() string { return c.name }

// CreatedAt returns the nanosecond timestamp when the channel was created.
func (c *Channel) CreatedAt() int64 { return c.createdAt }

// ─────────────────────────────────────────────────────────────────────────────
// Core Operations
// ─────────────────────────────────────────────────────────────────────────────

// Append adds a message to the end of the channel log and returns its
// sequence number (0-indexed, monotonically increasing).
//
// This is the ONLY write operation. There is no delete, no update, no
// insert-at-position. The append-only property is what makes crash recovery
// possible — the log is always consistent up to the last complete write.
//
//	seq := ch.Append(msg)  // seq = 0 for first message, 1 for second, etc.
func (c *Channel) Append(msg *Message) int {
	seq := len(c.log)
	c.log = append(c.log, msg)
	return seq
}

// Read returns up to `limit` messages starting from `offset`.
//
// Behavior:
//   - If offset >= log length: returns empty slice (caller is caught up).
//   - If offset + limit > log length: returns remaining messages.
//   - Does NOT consume the messages — they remain in the log. Another
//     reader can read the same messages independently.
//
// The returned slice is a copy — modifying it does not affect the channel.
//
//	msgs := ch.Read(0, 10)  // First 10 messages
//	msgs = ch.Read(5, 3)    // Messages 5, 6, 7
func (c *Channel) Read(offset, limit int) []*Message {
	if offset >= len(c.log) {
		return []*Message{}
	}
	end := offset + limit
	if end > len(c.log) {
		end = len(c.log)
	}
	// Return a copy of the slice so callers can't modify our log.
	result := make([]*Message, end-offset)
	copy(result, c.log[offset:end])
	return result
}

// Length returns the number of messages in the log.
func (c *Channel) Length() int {
	return len(c.log)
}

// Slice returns messages from index `start` to `end` (exclusive).
// This is equivalent to Read(start, end-start).
//
//	msgs := ch.Slice(1, 4)  // Messages at indices 1, 2, 3
func (c *Channel) Slice(start, end int) []*Message {
	if start < 0 {
		start = 0
	}
	if end > len(c.log) {
		end = len(c.log)
	}
	if start >= end {
		return []*Message{}
	}
	result := make([]*Message, end-start)
	copy(result, c.log[start:end])
	return result
}

// ─────────────────────────────────────────────────────────────────────────────
// Persistence
// ─────────────────────────────────────────────────────────────────────────────
//
// Channels persist to disk as a binary append log. Each message is serialized
// using the Message wire format (header + envelope + payload), concatenated
// end-to-end in a single file.
//
// File layout:
//
//	┌──────────────────────────────────────────────────┐
//	│[ACTM][v1][env_len=82][pay_len=45] msg 0 header   │
//	│{"id":"msg_001","sender_id":"...",...} envelope    │
//	│Meeting tomorrow at 3pm...           payload      │
//	├──────────────────────────────────────────────────┤
//	│[ACTM][v1][env_len=78][pay_len=1048576] msg 1     │
//	│{"id":"msg_002","content_type":"image/png",...}    │
//	│<1MB of raw PNG bytes>                            │
//	└──────────────────────────────────────────────────┘

// Persist writes the entire channel log to a binary file in the given
// directory. The filename is derived from the channel name: "{name}.log".
//
// Each message is written using the Message wire format, concatenated
// end-to-end. This allows sequential reading during recovery.
func (c *Channel) Persist(directory string) error {
	_, err := StartNew[struct{}]("actor.Persist", struct{}{},
		func(op *Operation[struct{}], rf *ResultFactory[struct{}]) *OperationResult[struct{}] {
			path := filepath.Join(directory, c.name+".log")

			// Assemble all message bytes in memory so we can write
			// the complete log in a single WriteFile call.
			var buf bytes.Buffer
			for _, msg := range c.log {
				buf.Write(msg.ToBytes())
			}

			if err := op.File.WriteFile(path, buf.Bytes(), 0o644); err != nil {
				return rf.Fail(struct{}{}, fmt.Errorf("actor: failed to write channel file %s: %w", path, err))
			}
			return rf.Generate(true, false, struct{}{})
		}).GetResult()
	return err
}

// Recover reconstructs a channel from a binary log file on disk.
//
// The recovery process:
//  1. Open the file "{directory}/{name}.log".
//  2. Read messages sequentially using FromReader.
//  3. If a truncated message is found (crash mid-write), discard it.
//  4. Return the reconstructed channel with all complete messages.
//
// If the file does not exist, an empty channel is returned. This is not
// an error — it means the channel has never been persisted.
//
// This function is the Go equivalent of a Python classmethod — it creates
// and returns a new Channel instance.
func Recover(directory, name string) (*Channel, error) {
	return StartNew[*Channel]("actor.Recover", nil,
		func(op *Operation[*Channel], rf *ResultFactory[*Channel]) *OperationResult[*Channel] {
			ch := NewChannel("recovered_"+name, name)

			path := filepath.Join(directory, name+".log")

			// Read all the file data into memory, then parse messages from it.
			// This approach handles truncated writes gracefully: if we encounter
			// an incomplete message, we simply stop reading.
			data, err := op.File.ReadFile(path)
			if err != nil {
				// File doesn't exist — return empty channel. This is fine.
				return rf.Generate(true, false, ch)
			}

			reader := bytes.NewReader(data)

			for {
				msg, err := FromReader(reader)
				if err != nil {
					if err == io.EOF {
						// Clean end of file — all messages read.
						break
					}
					if err == io.ErrUnexpectedEOF {
						// Truncated message — crash happened mid-write.
						// Discard the incomplete message and stop.
						break
					}
					// Other errors (bad magic, version too new) — stop but
					// keep what we've recovered so far.
					break
				}
				ch.log = append(ch.log, msg)
			}

			return rf.Generate(true, false, ch)
		}).GetResult()
}
