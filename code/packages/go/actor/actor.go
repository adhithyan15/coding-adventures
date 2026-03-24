package actor

// This file implements the Actor primitive — an isolated unit of computation
// with a mailbox and internal state.
//
// # The Actor Model
//
// An actor is defined by what it CAN do:
//   1. Receive a message
//   2. Send messages to other actors it knows about
//   3. Create new actors
//   4. Change its own internal state in response to a message
//
// An actor CANNOT:
//   1. Access another actor's internal state
//   2. Share memory with another actor
//   3. Communicate except through messages
//
// # Analogy
//
// An actor is a person sitting alone in a soundproofed room with a mail slot
// in the door. Letters (messages) come in through the slot and pile up in a
// tray (mailbox). The person reads one letter at a time, thinks about it,
// possibly writes reply letters and slides them out through their own mail
// slot, and possibly rearranges things on their desk (state). They never
// leave the room. They never look into anyone else's room.
//
// # Processing Guarantees
//
//   - Sequential: One message at a time. No concurrency within a single actor.
//   - At-most-once: A message is delivered exactly once. If the actor crashes
//     mid-processing, the message is lost.
//   - Ordered per-sender: If Actor A sends m1 then m2 to Actor B, B sees m1
//     before m2. No guarantee across different senders.

// ─────────────────────────────────────────────────────────────────────────────
// Status
// ─────────────────────────────────────────────────────────────────────────────

// ActorStatus represents the lifecycle state of an actor.
//
//	IDLE       — waiting for messages. Ready to process.
//	PROCESSING — currently handling a message. Mailbox accumulates.
//	STOPPED    — permanently halted. No more messages accepted.
type ActorStatus string

const (
	// StatusIdle means the actor is waiting for messages.
	StatusIdle ActorStatus = "idle"
	// StatusProcessing means the actor is currently handling a message.
	StatusProcessing ActorStatus = "processing"
	// StatusStopped means the actor is permanently halted.
	StatusStopped ActorStatus = "stopped"
)

// ─────────────────────────────────────────────────────────────────────────────
// BehaviorFunc
// ─────────────────────────────────────────────────────────────────────────────

// BehaviorFunc is the type signature for an actor's behavior function.
//
// It takes the actor's current state and one message, and returns an
// ActorResult describing what happened:
//   - What the new state should be
//   - What messages to send to other actors
//   - What new actors to create
//   - Whether to stop this actor
//
// The state parameter is interface{} — each actor chooses its own state type.
// A counter actor might use int, a cache actor might use map[string]string,
// a stateless echo actor might use nil.
//
// If the behavior function returns an error, the actor system will:
//   1. Leave the actor's state unchanged
//   2. Move the message to dead letters
//   3. Set the actor back to IDLE to process the next message
type BehaviorFunc func(state interface{}, msg *Message) (*ActorResult, error)

// ─────────────────────────────────────────────────────────────────────────────
// ActorResult
// ─────────────────────────────────────────────────────────────────────────────

// OutgoingMessage pairs a target actor ID with the message to send.
// This is how an actor's behavior function says "send this message to
// that actor."
type OutgoingMessage struct {
	TargetID string
	Msg      *Message
}

// ActorResult is the return value from a behavior function. It tells the
// actor system what to do after processing a message.
//
//	+──────────────────+──────────────────────────────────────────────+
//	| NewState         | The actor's state after processing.         |
//	| MessagesToSend   | List of (target, message) pairs to deliver. |
//	| ActorsToCreate   | List of actor specs to spawn.               |
//	| Stop             | If true, halt this actor permanently.       |
//	+──────────────────+──────────────────────────────────────────────+
type ActorResult struct {
	NewState       interface{}
	MessagesToSend []OutgoingMessage
	ActorsToCreate []ActorSpec
	Stop           bool
}

// ─────────────────────────────────────────────────────────────────────────────
// ActorSpec
// ─────────────────────────────────────────────────────────────────────────────

// ActorSpec is a specification for creating a new actor. When a behavior
// function wants to spawn a child actor, it includes an ActorSpec in the
// ActorResult.ActorsToCreate list.
//
//	spec := ActorSpec{
//	    ID:           "worker-1",
//	    InitialState: 0,
//	    Behavior:     counterBehavior,
//	}
type ActorSpec struct {
	ID           string
	InitialState interface{}
	Behavior     BehaviorFunc
}

// ─────────────────────────────────────────────────────────────────────────────
// Actor
// ─────────────────────────────────────────────────────────────────────────────

// Actor is an isolated unit of computation with a mailbox, internal state,
// and a behavior function.
//
// Fields:
//
//	+──────────+────────────────────────────────────────────────────────+
//	| id       | Unique identifier — this actor's "address".           |
//	| mailbox  | FIFO queue of incoming Messages.                      |
//	| state    | Private data. Only the behavior function can access.  |
//	| behavior | Function that processes one message at a time.        |
//	| status   | IDLE | PROCESSING | STOPPED                          |
//	+──────────+────────────────────────────────────────────────────────+
type Actor struct {
	id       string
	mailbox  []*Message
	state    interface{}
	behavior BehaviorFunc
	status   ActorStatus
}

// newActor creates a new actor with the given ID, initial state, and behavior
// function. The mailbox starts empty and status starts as IDLE.
//
// This is not exported — actors are created through the ActorSystem, which
// manages their lifecycle and ensures ID uniqueness.
func newActor(id string, initialState interface{}, behavior BehaviorFunc) *Actor {
	return &Actor{
		id:       id,
		mailbox:  make([]*Message, 0),
		state:    initialState,
		behavior: behavior,
		status:   StatusIdle,
	}
}

// ID returns the actor's unique identifier.
func (a *Actor) ID() string { return a.id }

// Status returns the actor's current lifecycle status.
func (a *Actor) Status() ActorStatus { return a.status }

// MailboxSize returns the number of pending messages in the actor's mailbox.
func (a *Actor) MailboxSize() int { return len(a.mailbox) }

// enqueue adds a message to the back of the actor's mailbox (FIFO).
func (a *Actor) enqueue(msg *Message) {
	a.mailbox = append(a.mailbox, msg)
}

// dequeue removes and returns the front message from the mailbox, or nil
// if the mailbox is empty.
func (a *Actor) dequeue() *Message {
	if len(a.mailbox) == 0 {
		return nil
	}
	msg := a.mailbox[0]
	a.mailbox = a.mailbox[1:]
	return msg
}
