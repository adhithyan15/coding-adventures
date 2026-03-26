package actor

// ActorSystem is the runtime that manages actor lifecycles, message delivery,
// and channels. It is the "world" that actors live in.
//
// # Analogy
//
// The ActorSystem is the office building. It has a directory (which actors
// exist and their addresses), a mail room (message routing), and a building
// manager (supervision — restart actors that crash). Actors are tenants.
// They register with the building, get an address, and the building delivers
// their mail. But the building manager does not read the mail.
//
// # Processing Model
//
// In V1, the ActorSystem processes actors sequentially in round-robin order.
// True parallelism (multiple actors processing simultaneously on different
// goroutines) is a future enhancement. The sequential model is simpler to
// test, debug, and reason about.
//
// # Dead Letters
//
// When a message cannot be delivered — because the target actor doesn't exist
// or has been stopped — the message goes to the dead letter queue. This is
// useful for debugging: you can inspect dead letters to find routing errors,
// missing actors, or messages sent after shutdown.

import (
	"fmt"
)

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

// ErrActorExists is returned when creating an actor with an ID that is
// already registered. Actor IDs must be unique within a system.
var ErrActorExists = fmt.Errorf("actor: actor with this ID already exists")

// ErrActorNotFound is returned when sending to or processing an actor
// that is not registered in the system.
var ErrActorNotFound = fmt.Errorf("actor: actor not found")

// ─────────────────────────────────────────────────────────────────────────────
// ActorSystem
// ─────────────────────────────────────────────────────────────────────────────

// ActorSystem manages actor lifecycles, message routing, and channels.
//
// Fields:
//
//	+─────────────+──────────────────────────────────────────────────+
//	| actors      | Map of actor_id → Actor. The registry.          |
//	| channels    | Map of channel_id → Channel.                    |
//	| deadLetters | Messages that could not be delivered.            |
//	| clock       | Monotonic counter for message timestamps.        |
//	| actorOrder  | Insertion-ordered list of actor IDs for          |
//	|             | deterministic round-robin processing.            |
//	+─────────────+──────────────────────────────────────────────────+
type ActorSystem struct {
	actors      map[string]*Actor
	channels    map[string]*Channel
	deadLetters []*Message
	clock       int64
	actorOrder  []string
}

// NewActorSystem creates a new empty actor system.
//
//	system := NewActorSystem()
//	system.CreateActor("echo", nil, echoBehavior)
//	system.Send("echo", NewTextMessage("user", "hello", nil))
//	system.RunUntilIdle()
func NewActorSystem() *ActorSystem {
	return &ActorSystem{
		actors:      make(map[string]*Actor),
		channels:    make(map[string]*Channel),
		deadLetters: make([]*Message, 0),
		clock:       0,
		actorOrder:  make([]string, 0),
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Actor Lifecycle
// ─────────────────────────────────────────────────────────────────────────────

// CreateActor registers a new actor in the system with the given ID,
// initial state, and behavior function.
//
// Returns the actor ID on success. Returns ErrActorExists if an actor
// with the same ID already exists.
//
// The actor starts in IDLE status with an empty mailbox.
//
//	id, err := system.CreateActor("counter", 0, counterBehavior)
func (s *ActorSystem) CreateActor(id string, initialState interface{}, behavior BehaviorFunc) (string, error) {
	if _, exists := s.actors[id]; exists {
		return "", fmt.Errorf("%w: %s", ErrActorExists, id)
	}
	a := newActor(id, initialState, behavior)
	s.actors[id] = a
	s.actorOrder = append(s.actorOrder, id)
	return id, nil
}

// StopActor sets the actor's status to STOPPED and drains any remaining
// messages in its mailbox to dead letters.
//
// After stopping, the actor will not accept new messages — any messages
// sent to it will go to dead letters.
//
// Returns ErrActorNotFound if no actor with the given ID exists.
func (s *ActorSystem) StopActor(id string) error {
	a, exists := s.actors[id]
	if !exists {
		return fmt.Errorf("%w: %s", ErrActorNotFound, id)
	}

	a.status = StatusStopped

	// Drain remaining mailbox messages to dead letters.
	// These messages will never be processed — save them for debugging.
	for {
		msg := a.dequeue()
		if msg == nil {
			break
		}
		s.deadLetters = append(s.deadLetters, msg)
	}

	return nil
}

// GetActorStatus returns the lifecycle status of the actor with the given ID.
// Returns StatusIdle, StatusProcessing, or StatusStopped.
//
// Returns an error if the actor is not found.
func (s *ActorSystem) GetActorStatus(id string) (ActorStatus, error) {
	a, exists := s.actors[id]
	if !exists {
		return "", fmt.Errorf("%w: %s", ErrActorNotFound, id)
	}
	return a.status, nil
}

// ─────────────────────────────────────────────────────────────────────────────
// Messaging
// ─────────────────────────────────────────────────────────────────────────────

// Send delivers a message to the target actor's mailbox.
//
// If the target actor does not exist or is stopped, the message goes to
// dead letters and an error is returned.
//
// The message delivery algorithm:
//  1. Look up target_id in actors map.
//  2. If NOT FOUND → dead_letters.append(message). Return error.
//  3. If STOPPED → dead_letters.append(message). Return error.
//  4. Enqueue message in target's mailbox.
//
// Time complexity: O(1) — hash map lookup + queue append.
func (s *ActorSystem) Send(targetID string, msg *Message) error {
	a, exists := s.actors[targetID]
	if !exists {
		s.deadLetters = append(s.deadLetters, msg)
		return fmt.Errorf("%w: %s", ErrActorNotFound, targetID)
	}
	if a.status == StatusStopped {
		s.deadLetters = append(s.deadLetters, msg)
		return fmt.Errorf("%w: %s (stopped)", ErrActorNotFound, targetID)
	}
	a.enqueue(msg)
	return nil
}

// ─────────────────────────────────────────────────────────────────────────────
// Processing
// ─────────────────────────────────────────────────────────────────────────────

// ProcessNext processes one message from the specified actor's mailbox.
//
// Returns true if a message was processed, false if the mailbox was empty.
// Returns an error if the actor is not found or is stopped.
//
// The processing algorithm:
//  1. Look up actor. Return error if not found or stopped.
//  2. If mailbox empty, return false (no work).
//  3. Dequeue front message.
//  4. Set status = PROCESSING.
//  5. Call behavior(state, message) → ActorResult.
//  6. If behavior returns error: state unchanged, message → dead letters,
//     actor continues processing (status = IDLE).
//  7. Update state to ActorResult.NewState.
//  8. Deliver each outgoing message via Send().
//  9. Create each new actor from ActorsToCreate specs.
//  10. If ActorResult.Stop: set status = STOPPED, drain mailbox.
//  11. Else: set status = IDLE.
func (s *ActorSystem) ProcessNext(actorID string) (bool, error) {
	a, exists := s.actors[actorID]
	if !exists {
		return false, fmt.Errorf("%w: %s", ErrActorNotFound, actorID)
	}
	if a.status == StatusStopped {
		return false, fmt.Errorf("%w: %s (stopped)", ErrActorNotFound, actorID)
	}

	// If mailbox is empty, there's nothing to process.
	msg := a.dequeue()
	if msg == nil {
		return false, nil
	}

	// Set status to PROCESSING while the behavior function runs.
	a.status = StatusProcessing

	// Call the behavior function.
	result, err := a.behavior(a.state, msg)
	if err != nil {
		// Behavior threw an error:
		//   - State is UNCHANGED (we don't apply partial updates)
		//   - Message goes to dead letters for debugging
		//   - Actor continues processing (returns to IDLE)
		s.deadLetters = append(s.deadLetters, msg)
		a.status = StatusIdle
		return true, nil
	}

	// Apply the result.
	a.state = result.NewState

	// Deliver outgoing messages.
	for _, out := range result.MessagesToSend {
		// Ignore send errors — they'll land in dead letters automatically.
		_ = s.Send(out.TargetID, out.Msg)
	}

	// Create new actors.
	for _, spec := range result.ActorsToCreate {
		_, _ = s.CreateActor(spec.ID, spec.InitialState, spec.Behavior)
	}

	// Check if the actor wants to stop.
	if result.Stop {
		a.status = StatusStopped
		// Drain remaining mailbox to dead letters.
		for {
			remaining := a.dequeue()
			if remaining == nil {
				break
			}
			s.deadLetters = append(s.deadLetters, remaining)
		}
	} else {
		a.status = StatusIdle
	}

	return true, nil
}

// RunUntilIdle processes all actors in round-robin order until no actor has
// pending messages.
//
// The algorithm:
//  1. Scan all actors for one with status==IDLE and non-empty mailbox.
//  2. If none found, return (system is idle).
//  3. Process one message from that actor.
//  4. Repeat from step 1.
//
// Returns statistics about what happened during processing.
//
// NOTE: In V1, this processes actors one at a time. True parallelism
// is a future enhancement.
func (s *ActorSystem) RunUntilIdle() map[string]int {
	stats := map[string]int{
		"messages_processed": 0,
		"actors_created":     0,
	}

	for {
		found := false
		for _, id := range s.actorOrder {
			a, exists := s.actors[id]
			if !exists {
				continue
			}
			if a.status == StatusIdle && len(a.mailbox) > 0 {
				actorsBefore := len(s.actors)
				processed, _ := s.ProcessNext(id)
				if processed {
					stats["messages_processed"]++
					actorsAfter := len(s.actors)
					stats["actors_created"] += actorsAfter - actorsBefore
					found = true
				}
			}
		}
		// Also check any newly created actors (their IDs were appended
		// to actorOrder during this round).
		if !found {
			break
		}
	}

	return stats
}

// RunUntilDone processes all actors repeatedly until the system is completely
// quiet — no messages in any mailbox and no new messages being generated.
//
// This is like RunUntilIdle but it keeps going through multiple rounds.
// RunUntilIdle stops when one pass finds no work. RunUntilDone repeats
// until the system truly settles.
//
// Safety: This will loop forever if actors keep generating new messages
// indefinitely (e.g., ping-pong without a stop condition). Use with caution.
func (s *ActorSystem) RunUntilDone() map[string]int {
	totalStats := map[string]int{
		"messages_processed": 0,
		"actors_created":     0,
	}

	for {
		stats := s.RunUntilIdle()
		totalStats["messages_processed"] += stats["messages_processed"]
		totalStats["actors_created"] += stats["actors_created"]

		if stats["messages_processed"] == 0 {
			break
		}
	}

	return totalStats
}

// ─────────────────────────────────────────────────────────────────────────────
// Channels
// ─────────────────────────────────────────────────────────────────────────────

// CreateChannel creates and registers a new channel in the actor system.
//
//	ch := system.CreateChannel("ch_001", "email-summaries")
func (s *ActorSystem) CreateChannel(id, name string) *Channel {
	ch := NewChannel(id, name)
	s.channels[id] = ch
	return ch
}

// GetChannel retrieves a channel by ID. Returns nil if not found.
func (s *ActorSystem) GetChannel(id string) *Channel {
	return s.channels[id]
}

// ─────────────────────────────────────────────────────────────────────────────
// Inspection
// ─────────────────────────────────────────────────────────────────────────────

// DeadLetters returns a copy of all messages that could not be delivered.
// These include messages sent to non-existent actors, stopped actors,
// and messages that caused behavior function errors.
func (s *ActorSystem) DeadLetters() []*Message {
	result := make([]*Message, len(s.deadLetters))
	copy(result, s.deadLetters)
	return result
}

// ActorIDs returns a list of all registered actor IDs.
func (s *ActorSystem) ActorIDs() []string {
	ids := make([]string, 0, len(s.actors))
	for id := range s.actors {
		ids = append(ids, id)
	}
	return ids
}

// MailboxSize returns the number of pending messages in the specified
// actor's mailbox. Returns 0 if the actor is not found.
func (s *ActorSystem) MailboxSize(actorID string) int {
	a, exists := s.actors[actorID]
	if !exists {
		return 0
	}
	return len(a.mailbox)
}

// Shutdown stops all actors, drains all mailboxes to dead letters.
func (s *ActorSystem) Shutdown() {
	for _, id := range s.actorOrder {
		a, exists := s.actors[id]
		if !exists {
			continue
		}
		if a.status != StatusStopped {
			_ = s.StopActor(id)
		}
	}
}
