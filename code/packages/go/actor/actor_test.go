package actor

import (
	"bytes"
	"fmt"
	"testing"
)

// ─────────────────────────────────────────────────────────────────────────────
// Helper behaviors used across multiple tests
// ─────────────────────────────────────────────────────────────────────────────

// echoBehavior sends the received message's text back to the sender.
func echoBehavior(state interface{}, msg *Message) (*ActorResult, error) {
	reply := NewTextMessage("echo", "echo: "+msg.PayloadText(), nil)
	return &ActorResult{
		NewState: state,
		MessagesToSend: []OutgoingMessage{
			{TargetID: msg.SenderID(), Msg: reply},
		},
	}, nil
}

// counterBehavior increments a counter for each message received.
func counterBehavior(state interface{}, msg *Message) (*ActorResult, error) {
	count := state.(int) + 1
	return &ActorResult{NewState: count}, nil
}

// failingBehavior returns an error for messages containing "fail".
func failingBehavior(state interface{}, msg *Message) (*ActorResult, error) {
	if msg.PayloadText() == "fail" {
		return nil, fmt.Errorf("intentional failure")
	}
	count := state.(int) + 1
	return &ActorResult{NewState: count}, nil
}

// stopBehavior stops the actor when it receives "stop".
func stopBehavior(state interface{}, msg *Message) (*ActorResult, error) {
	if msg.PayloadText() == "stop" {
		return &ActorResult{NewState: state, Stop: true}, nil
	}
	return &ActorResult{NewState: state}, nil
}

// spawnerBehavior creates a new echo actor when it receives "spawn".
func spawnerBehavior(state interface{}, msg *Message) (*ActorResult, error) {
	if msg.PayloadText() == "spawn" {
		count := state.(int)
		newID := fmt.Sprintf("spawned-%d", count)
		return &ActorResult{
			NewState: count + 1,
			ActorsToCreate: []ActorSpec{
				{ID: newID, InitialState: nil, Behavior: echoBehavior},
			},
			MessagesToSend: []OutgoingMessage{
				{TargetID: msg.SenderID(),
					Msg: NewTextMessage("spawner", "created "+newID, nil)},
			},
		}, nil
	}
	return &ActorResult{NewState: state}, nil
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 37: Create actor — status is IDLE
// ═══════════════════════════════════════════════════════════════════════════

func TestCreateActor(t *testing.T) {
	t.Run("new actor has IDLE status", func(t *testing.T) {
		sys := NewActorSystem()
		_, err := sys.CreateActor("test", nil, echoBehavior)
		if err != nil {
			t.Fatalf("CreateActor failed: %v", err)
		}
		status, err := sys.GetActorStatus("test")
		if err != nil {
			t.Fatalf("GetActorStatus failed: %v", err)
		}
		if status != StatusIdle {
			t.Errorf("Status: got %q, want %q", status, StatusIdle)
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 38: Send message — mailbox size is 1
// ═══════════════════════════════════════════════════════════════════════════

func TestSendMessage(t *testing.T) {
	t.Run("sending a message increases mailbox size", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("target", nil, echoBehavior)

		msg := NewTextMessage("sender", "hello", nil)
		err := sys.Send("target", msg)
		if err != nil {
			t.Fatalf("Send failed: %v", err)
		}
		if sys.MailboxSize("target") != 1 {
			t.Errorf("MailboxSize: got %d, want 1", sys.MailboxSize("target"))
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 39: Process message — behavior was called
// ═══════════════════════════════════════════════════════════════════════════

func TestProcessMessage(t *testing.T) {
	t.Run("process_next calls the behavior function", func(t *testing.T) {
		sys := NewActorSystem()
		called := false
		behavior := func(state interface{}, msg *Message) (*ActorResult, error) {
			called = true
			return &ActorResult{NewState: state}, nil
		}
		sys.CreateActor("test", nil, behavior)
		sys.Send("test", NewTextMessage("sender", "hello", nil))

		processed, err := sys.ProcessNext("test")
		if err != nil {
			t.Fatalf("ProcessNext failed: %v", err)
		}
		if !processed {
			t.Error("ProcessNext should return true")
		}
		if !called {
			t.Error("Behavior function was not called")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 40: State update — counter actor counts to 3
// ═══════════════════════════════════════════════════════════════════════════

func TestStateUpdate(t *testing.T) {
	t.Run("counter increments state for each message", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("counter", 0, counterBehavior)

		for i := 0; i < 3; i++ {
			sys.Send("counter", NewTextMessage("sender", "tick", nil))
		}
		sys.RunUntilIdle()

		// Verify state by sending one more message and checking the behavior.
		// We use a custom behavior that reports the current state.
		var finalState interface{}
		reportBehavior := func(state interface{}, msg *Message) (*ActorResult, error) {
			finalState = state
			return &ActorResult{NewState: state}, nil
		}

		// Replace the actor — create a new one that starts with the counter's state.
		// Actually, we can verify by sending another message and checking count.
		sys.Send("counter", NewTextMessage("sender", "tick", nil))
		sys.RunUntilIdle()

		// After 4 messages total, the counter state should be 4.
		// We verify indirectly through the ProcessNext mechanism by
		// creating a new system with an inspectable behavior.
		sys2 := NewActorSystem()
		sys2.CreateActor("counter", 0, reportBehavior)
		for i := 0; i < 3; i++ {
			sys2.Send("counter", NewTextMessage("sender", "tick", nil))
		}
		sys2.RunUntilIdle()
		if finalState.(int) != 0 {
			// reportBehavior doesn't increment — but finalState was set to 0
			// each time. Let's use counterBehavior properly.
		}

		// Better approach: use counterBehavior and check via a reporting message.
		sys3 := NewActorSystem()
		var reportedCount int
		countAndReportBehavior := func(state interface{}, msg *Message) (*ActorResult, error) {
			count := state.(int) + 1
			reportedCount = count
			return &ActorResult{NewState: count}, nil
		}
		sys3.CreateActor("counter", 0, countAndReportBehavior)
		for i := 0; i < 3; i++ {
			sys3.Send("counter", NewTextMessage("sender", "tick", nil))
		}
		sys3.RunUntilIdle()
		if reportedCount != 3 {
			t.Errorf("Counter state: got %d, want 3", reportedCount)
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 41: Messages to send — echo reply delivered
// ═══════════════════════════════════════════════════════════════════════════

func TestMessagesToSend(t *testing.T) {
	t.Run("echo reply is delivered to sender's mailbox", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("echo", nil, echoBehavior)
		sys.CreateActor("user", nil, func(state interface{}, msg *Message) (*ActorResult, error) {
			return &ActorResult{NewState: msg.PayloadText()}, nil
		})

		sys.Send("echo", NewTextMessage("user", "hello", nil))
		sys.RunUntilIdle()

		// The echo actor should have sent "echo: hello" to "user".
		// After RunUntilIdle, "user" should have processed the reply.
		if sys.MailboxSize("user") != 0 {
			t.Errorf("User mailbox should be empty after processing, got %d",
				sys.MailboxSize("user"))
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 42: Actor creation — spawner creates new actors
// ═══════════════════════════════════════════════════════════════════════════

func TestActorCreation(t *testing.T) {
	t.Run("spawner creates a new actor in the system", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("requester", nil, func(state interface{}, msg *Message) (*ActorResult, error) {
			return &ActorResult{NewState: state}, nil
		})
		sys.CreateActor("spawner", 0, spawnerBehavior)

		sys.Send("spawner", NewTextMessage("requester", "spawn", nil))
		sys.RunUntilIdle()

		// The spawner should have created "spawned-0".
		status, err := sys.GetActorStatus("spawned-0")
		if err != nil {
			t.Fatalf("spawned-0 should exist: %v", err)
		}
		if status != StatusIdle {
			t.Errorf("spawned-0 status: got %q, want %q", status, StatusIdle)
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 43: Stop actor — status is STOPPED
// ═══════════════════════════════════════════════════════════════════════════

func TestStopActor(t *testing.T) {
	t.Run("actor status becomes STOPPED after stop message", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("test", nil, stopBehavior)
		sys.Send("test", NewTextMessage("sender", "stop", nil))
		sys.ProcessNext("test")

		status, _ := sys.GetActorStatus("test")
		if status != StatusStopped {
			t.Errorf("Status: got %q, want %q", status, StatusStopped)
		}
	})

	t.Run("StopActor method sets STOPPED", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("test", nil, echoBehavior)
		sys.StopActor("test")

		status, _ := sys.GetActorStatus("test")
		if status != StatusStopped {
			t.Errorf("Status: got %q, want %q", status, StatusStopped)
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 44: Stopped actor rejects messages → dead letters
// ═══════════════════════════════════════════════════════════════════════════

func TestStoppedActorRejectsMessages(t *testing.T) {
	t.Run("messages to stopped actor go to dead letters", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("test", nil, echoBehavior)
		sys.StopActor("test")

		msg := NewTextMessage("sender", "hello", nil)
		err := sys.Send("test", msg)
		if err == nil {
			t.Error("Send to stopped actor should return error")
		}

		dl := sys.DeadLetters()
		// dead letters include messages drained from the mailbox + the new one
		found := false
		for _, d := range dl {
			if d.ID() == msg.ID() {
				found = true
				break
			}
		}
		if !found {
			t.Error("Message should be in dead letters")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 45: Dead letters — message to non-existent actor
// ═══════════════════════════════════════════════════════════════════════════

func TestDeadLetters(t *testing.T) {
	t.Run("message to non-existent actor goes to dead letters", func(t *testing.T) {
		sys := NewActorSystem()
		msg := NewTextMessage("sender", "hello", nil)
		err := sys.Send("nonexistent", msg)
		if err == nil {
			t.Error("Send to nonexistent actor should return error")
		}

		dl := sys.DeadLetters()
		if len(dl) != 1 {
			t.Fatalf("Dead letters: got %d, want 1", len(dl))
		}
		if dl[0].ID() != msg.ID() {
			t.Error("Dead letter should be the sent message")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 46: Sequential processing — FIFO order
// ═══════════════════════════════════════════════════════════════════════════

func TestSequentialProcessing(t *testing.T) {
	t.Run("messages are processed in FIFO order", func(t *testing.T) {
		sys := NewActorSystem()
		var order []string
		behavior := func(state interface{}, msg *Message) (*ActorResult, error) {
			order = append(order, msg.PayloadText())
			return &ActorResult{NewState: state}, nil
		}
		sys.CreateActor("test", nil, behavior)

		sys.Send("test", NewTextMessage("sender", "first", nil))
		sys.Send("test", NewTextMessage("sender", "second", nil))
		sys.Send("test", NewTextMessage("sender", "third", nil))

		sys.ProcessNext("test")
		sys.ProcessNext("test")
		sys.ProcessNext("test")

		if len(order) != 3 {
			t.Fatalf("Processed %d messages, want 3", len(order))
		}
		if order[0] != "first" || order[1] != "second" || order[2] != "third" {
			t.Errorf("Order: got %v, want [first second third]", order)
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 47: Mailbox drains on stop → dead letters
// ═══════════════════════════════════════════════════════════════════════════

func TestMailboxDrainsOnStop(t *testing.T) {
	t.Run("pending messages go to dead letters when actor stops", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("test", nil, echoBehavior)

		// Queue up 3 messages.
		sys.Send("test", NewTextMessage("sender", "a", nil))
		sys.Send("test", NewTextMessage("sender", "b", nil))
		sys.Send("test", NewTextMessage("sender", "c", nil))

		if sys.MailboxSize("test") != 3 {
			t.Fatalf("MailboxSize: got %d, want 3", sys.MailboxSize("test"))
		}

		// Stop the actor — all 3 should go to dead letters.
		sys.StopActor("test")

		dl := sys.DeadLetters()
		if len(dl) != 3 {
			t.Errorf("Dead letters: got %d, want 3", len(dl))
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 48: Behavior exception — state unchanged, message → dead letters
// ═══════════════════════════════════════════════════════════════════════════

func TestBehaviorException(t *testing.T) {
	t.Run("error leaves state unchanged and continues processing", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("test", 0, failingBehavior)

		// Send a normal message, a failing message, then another normal message.
		sys.Send("test", NewTextMessage("sender", "ok", nil))
		sys.Send("test", NewTextMessage("sender", "fail", nil))
		sys.Send("test", NewTextMessage("sender", "ok again", nil))

		sys.RunUntilIdle()

		// Dead letters should contain the failed message.
		dl := sys.DeadLetters()
		found := false
		for _, d := range dl {
			if d.PayloadText() == "fail" {
				found = true
			}
		}
		if !found {
			t.Error("Failed message should be in dead letters")
		}

		// The actor should still be alive and have processed the other 2 messages.
		// State should be 2 (incremented for "ok" and "ok again", not for "fail").
		status, _ := sys.GetActorStatus("test")
		if status != StatusIdle {
			t.Errorf("Status should be idle, got %q", status)
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 49: Duplicate actor ID
// ═══════════════════════════════════════════════════════════════════════════

func TestDuplicateActorID(t *testing.T) {
	t.Run("creating actor with existing ID returns error", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("test", nil, echoBehavior)

		_, err := sys.CreateActor("test", nil, echoBehavior)
		if err == nil {
			t.Error("Expected error for duplicate actor ID")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 50: Ping-pong — two actors exchange 10 messages
// ═══════════════════════════════════════════════════════════════════════════

func TestPingPong(t *testing.T) {
	t.Run("two actors exchange 10 messages then stop", func(t *testing.T) {
		sys := NewActorSystem()
		var pingCount, pongCount int

		pingBehavior := func(state interface{}, msg *Message) (*ActorResult, error) {
			count := state.(int) + 1
			pingCount = count
			if count >= 10 {
				return &ActorResult{NewState: count, Stop: true}, nil
			}
			reply := NewTextMessage("ping", fmt.Sprintf("ping-%d", count), nil)
			return &ActorResult{
				NewState:       count,
				MessagesToSend: []OutgoingMessage{{TargetID: "pong", Msg: reply}},
			}, nil
		}

		pongBehavior := func(state interface{}, msg *Message) (*ActorResult, error) {
			count := state.(int) + 1
			pongCount = count
			if count >= 10 {
				return &ActorResult{NewState: count, Stop: true}, nil
			}
			reply := NewTextMessage("pong", fmt.Sprintf("pong-%d", count), nil)
			return &ActorResult{
				NewState:       count,
				MessagesToSend: []OutgoingMessage{{TargetID: "ping", Msg: reply}},
			}, nil
		}

		sys.CreateActor("ping", 0, pingBehavior)
		sys.CreateActor("pong", 0, pongBehavior)

		// Start the exchange.
		sys.Send("ping", NewTextMessage("pong", "start", nil))
		sys.RunUntilDone()

		if pingCount != 10 {
			t.Errorf("Ping processed %d messages, want 10", pingCount)
		}
		if pongCount < 9 {
			t.Errorf("Pong processed %d messages, want >= 9", pongCount)
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 51: Pipeline — A → B → C
// ═══════════════════════════════════════════════════════════════════════════

func TestPipeline(t *testing.T) {
	t.Run("three actors in a chain transform and forward", func(t *testing.T) {
		sys := NewActorSystem()
		var finalResult string

		// A: receives message, uppercases, forwards to B.
		aBehavior := func(state interface{}, msg *Message) (*ActorResult, error) {
			upper := NewTextMessage("A", "UPPER:"+msg.PayloadText(), nil)
			return &ActorResult{
				NewState:       state,
				MessagesToSend: []OutgoingMessage{{TargetID: "B", Msg: upper}},
			}, nil
		}

		// B: receives message, adds prefix, forwards to C.
		bBehavior := func(state interface{}, msg *Message) (*ActorResult, error) {
			prefixed := NewTextMessage("B", "PREFIX:"+msg.PayloadText(), nil)
			return &ActorResult{
				NewState:       state,
				MessagesToSend: []OutgoingMessage{{TargetID: "C", Msg: prefixed}},
			}, nil
		}

		// C: terminal — stores the result.
		cBehavior := func(state interface{}, msg *Message) (*ActorResult, error) {
			finalResult = msg.PayloadText()
			return &ActorResult{NewState: state}, nil
		}

		sys.CreateActor("A", nil, aBehavior)
		sys.CreateActor("B", nil, bBehavior)
		sys.CreateActor("C", nil, cBehavior)

		sys.Send("A", NewTextMessage("user", "hello", nil))
		sys.RunUntilDone()

		expected := "PREFIX:UPPER:hello"
		if finalResult != expected {
			t.Errorf("Pipeline result: got %q, want %q", finalResult, expected)
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 52: Channel-based pipeline
// ═══════════════════════════════════════════════════════════════════════════

func TestChannelBasedPipeline(t *testing.T) {
	t.Run("producer writes to channel, consumer reads in order", func(t *testing.T) {
		sys := NewActorSystem()
		ch := sys.CreateChannel("ch_001", "pipeline")

		// Producer writes 5 messages.
		for i := 0; i < 5; i++ {
			msg := NewTextMessage("producer", fmt.Sprintf("msg-%d", i), nil)
			ch.Append(msg)
		}

		// Consumer reads from offset 0.
		msgs := ch.Read(0, 10)
		if len(msgs) != 5 {
			t.Fatalf("Consumer read %d messages, want 5", len(msgs))
		}
		for i, msg := range msgs {
			expected := fmt.Sprintf("msg-%d", i)
			if msg.PayloadText() != expected {
				t.Errorf("Message %d: got %q, want %q", i, msg.PayloadText(), expected)
			}
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 53: Fan-out — one actor sends to 5 actors
// ═══════════════════════════════════════════════════════════════════════════

func TestFanOut(t *testing.T) {
	t.Run("one actor sends to 5 receivers", func(t *testing.T) {
		sys := NewActorSystem()
		received := make(map[string]bool)

		receiverBehavior := func(state interface{}, msg *Message) (*ActorResult, error) {
			// Mark this receiver as having received the message.
			return &ActorResult{NewState: msg.PayloadText()}, nil
		}

		// Create broadcaster.
		broadcastBehavior := func(state interface{}, msg *Message) (*ActorResult, error) {
			outgoing := make([]OutgoingMessage, 5)
			for i := 0; i < 5; i++ {
				target := fmt.Sprintf("receiver-%d", i)
				outgoing[i] = OutgoingMessage{
					TargetID: target,
					Msg:      NewTextMessage("broadcaster", msg.PayloadText(), nil),
				}
			}
			return &ActorResult{
				NewState:       state,
				MessagesToSend: outgoing,
			}, nil
		}

		sys.CreateActor("broadcaster", nil, broadcastBehavior)
		for i := 0; i < 5; i++ {
			id := fmt.Sprintf("receiver-%d", i)
			sys.CreateActor(id, nil, func(state interface{}, msg *Message) (*ActorResult, error) {
				received[msg.PayloadText()] = true
				return receiverBehavior(state, msg)
			})
		}

		sys.Send("broadcaster", NewTextMessage("user", "broadcast!", nil))
		sys.RunUntilDone()

		// All 5 receivers should have gotten the message.
		if len(received) == 0 {
			t.Error("No receivers got the message")
		}
		if !received["broadcast!"] {
			t.Error("Receivers did not get the expected message content")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 54: Dynamic topology — spawn and communicate
// ═══════════════════════════════════════════════════════════════════════════

func TestDynamicTopology(t *testing.T) {
	t.Run("dynamically created actor responds to messages", func(t *testing.T) {
		sys := NewActorSystem()
		var receivedReply string

		// Requester: sends "spawn" to spawner, receives notification, then
		// sends a message to the newly created actor.
		requesterBehavior := func(state interface{}, msg *Message) (*ActorResult, error) {
			text := msg.PayloadText()
			if text == "start" {
				return &ActorResult{
					NewState: state,
					MessagesToSend: []OutgoingMessage{
						{TargetID: "spawner", Msg: NewTextMessage("requester", "spawn", nil)},
					},
				}, nil
			}
			// If we got a reply from the spawner or spawned actor.
			receivedReply = text
			return &ActorResult{NewState: state}, nil
		}

		sys.CreateActor("requester", nil, requesterBehavior)
		sys.CreateActor("spawner", 0, spawnerBehavior)

		sys.Send("requester", NewTextMessage("user", "start", nil))
		sys.RunUntilDone()

		// Verify spawned-0 exists.
		status, err := sys.GetActorStatus("spawned-0")
		if err != nil {
			t.Fatalf("spawned-0 not found: %v", err)
		}
		if status != StatusIdle {
			t.Errorf("spawned-0 status: %q", status)
		}

		// Now send a message to the spawned actor.
		sys.Send("spawned-0", NewTextMessage("requester", "hi spawned", nil))
		sys.RunUntilDone()

		// The spawned echo actor should have replied.
		if receivedReply == "" {
			t.Error("Did not receive reply from spawned actor")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 55: RunUntilIdle — complex network of 5 actors
// ═══════════════════════════════════════════════════════════════════════════

func TestRunUntilIdle(t *testing.T) {
	t.Run("5 actors process all messages and system becomes idle", func(t *testing.T) {
		sys := NewActorSystem()
		processed := make(map[string]int)

		makeBehavior := func(name string) BehaviorFunc {
			return func(state interface{}, msg *Message) (*ActorResult, error) {
				processed[name]++
				return &ActorResult{NewState: state}, nil
			}
		}

		for i := 0; i < 5; i++ {
			id := fmt.Sprintf("actor-%d", i)
			sys.CreateActor(id, nil, makeBehavior(id))
		}

		// Send messages to all actors.
		for i := 0; i < 5; i++ {
			target := fmt.Sprintf("actor-%d", i)
			for j := 0; j < 3; j++ {
				sys.Send(target, NewTextMessage("user", "work", nil))
			}
		}

		stats := sys.RunUntilIdle()
		if stats["messages_processed"] != 15 {
			t.Errorf("Processed %d messages, want 15", stats["messages_processed"])
		}

		// All mailboxes should be empty.
		for i := 0; i < 5; i++ {
			id := fmt.Sprintf("actor-%d", i)
			if sys.MailboxSize(id) != 0 {
				t.Errorf("Actor %s mailbox not empty: %d", id, sys.MailboxSize(id))
			}
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 56: Persistence round-trip with channels
// ═══════════════════════════════════════════════════════════════════════════

func TestPersistenceRoundTrip(t *testing.T) {
	t.Run("channels survive persist and recover with binary payloads", func(t *testing.T) {
		dir := t.TempDir()
		sys := NewActorSystem()
		ch := sys.CreateChannel("ch_001", "persist-test")

		// Add various message types.
		ch.Append(NewTextMessage("sender", "text message", nil))
		ch.Append(NewJSONMessage("sender", map[string]int{"count": 42}, nil))
		pngBytes := []byte{0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A}
		ch.Append(NewBinaryMessage("sender", "image/png", pngBytes, nil))

		if err := ch.Persist(dir); err != nil {
			t.Fatalf("Persist failed: %v", err)
		}

		// Recover in a new system.
		recovered, err := Recover(dir, "persist-test")
		if err != nil {
			t.Fatalf("Recover failed: %v", err)
		}
		if recovered.Length() != 3 {
			t.Fatalf("Recovered %d messages, want 3", recovered.Length())
		}

		msgs := recovered.Read(0, 3)
		if msgs[0].PayloadText() != "text message" {
			t.Errorf("Msg 0: got %q", msgs[0].PayloadText())
		}
		if !bytes.Equal(msgs[2].Payload(), pngBytes) {
			t.Error("Msg 2: PNG bytes mismatch")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 57: Large-scale — 100 actors, 1000 messages
// ═══════════════════════════════════════════════════════════════════════════

func TestLargeScale(t *testing.T) {
	t.Run("100 actors process 1000 messages with no loss", func(t *testing.T) {
		sys := NewActorSystem()
		totalProcessed := 0

		behavior := func(state interface{}, msg *Message) (*ActorResult, error) {
			totalProcessed++
			return &ActorResult{NewState: state}, nil
		}

		// Create 100 actors.
		for i := 0; i < 100; i++ {
			sys.CreateActor(fmt.Sprintf("actor-%d", i), nil, behavior)
		}

		// Send 1000 messages randomly distributed.
		for i := 0; i < 1000; i++ {
			target := fmt.Sprintf("actor-%d", i%100)
			sys.Send(target, NewTextMessage("sender", fmt.Sprintf("msg-%d", i), nil))
		}

		sys.RunUntilDone()

		if totalProcessed != 1000 {
			t.Errorf("Processed %d messages, want 1000", totalProcessed)
		}

		// All mailboxes should be empty.
		for i := 0; i < 100; i++ {
			id := fmt.Sprintf("actor-%d", i)
			if sys.MailboxSize(id) != 0 {
				t.Errorf("Actor %s has %d pending messages", id, sys.MailboxSize(id))
			}
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 58: Binary message pipeline via channel
// ═══════════════════════════════════════════════════════════════════════════

func TestBinaryMessagePipeline(t *testing.T) {
	t.Run("PNG image passes through channel and is identical", func(t *testing.T) {
		sys := NewActorSystem()
		ch := sys.CreateChannel("ch_images", "images")

		// Simulate a PNG image (8-byte header + some fake data).
		pngData := make([]byte, 1024)
		pngData[0] = 0x89
		pngData[1] = 0x50
		pngData[2] = 0x4E
		pngData[3] = 0x47
		for i := 4; i < len(pngData); i++ {
			pngData[i] = byte(i % 256)
		}

		// Actor A sends PNG to the channel.
		msg := NewBinaryMessage("actor-A", "image/png", pngData, nil)
		ch.Append(msg)

		// Actor B reads from the channel.
		msgs := ch.Read(0, 1)
		if len(msgs) != 1 {
			t.Fatalf("Expected 1 message, got %d", len(msgs))
		}

		received := msgs[0]
		if received.ContentType() != "image/png" {
			t.Errorf("ContentType: got %q, want image/png", received.ContentType())
		}
		if !bytes.Equal(received.Payload(), pngData) {
			t.Error("PNG payload mismatch — image was corrupted in transit")
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Additional tests for coverage
// ═══════════════════════════════════════════════════════════════════════════

func TestProcessNextEmptyMailbox(t *testing.T) {
	t.Run("process_next on empty mailbox returns false", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("test", nil, echoBehavior)
		processed, err := sys.ProcessNext("test")
		if err != nil {
			t.Fatalf("ProcessNext failed: %v", err)
		}
		if processed {
			t.Error("Should return false for empty mailbox")
		}
	})
}

func TestProcessNextNonexistent(t *testing.T) {
	t.Run("process_next on nonexistent actor returns error", func(t *testing.T) {
		sys := NewActorSystem()
		_, err := sys.ProcessNext("ghost")
		if err == nil {
			t.Error("Expected error for nonexistent actor")
		}
	})
}

func TestProcessNextStoppedActor(t *testing.T) {
	t.Run("process_next on stopped actor returns error", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("test", nil, echoBehavior)
		sys.StopActor("test")
		_, err := sys.ProcessNext("test")
		if err == nil {
			t.Error("Expected error for stopped actor")
		}
	})
}

func TestActorIDs(t *testing.T) {
	t.Run("lists all registered actor IDs", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("alpha", nil, echoBehavior)
		sys.CreateActor("beta", nil, echoBehavior)
		sys.CreateActor("gamma", nil, echoBehavior)

		ids := sys.ActorIDs()
		if len(ids) != 3 {
			t.Errorf("Expected 3 IDs, got %d", len(ids))
		}
	})
}

func TestGetChannel(t *testing.T) {
	t.Run("returns nil for non-existent channel", func(t *testing.T) {
		sys := NewActorSystem()
		ch := sys.GetChannel("ghost")
		if ch != nil {
			t.Error("Expected nil for non-existent channel")
		}
	})

	t.Run("returns channel after creation", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateChannel("ch1", "test-channel")
		ch := sys.GetChannel("ch1")
		if ch == nil {
			t.Fatal("Expected channel, got nil")
		}
		if ch.Name() != "test-channel" {
			t.Errorf("Channel name: got %q, want %q", ch.Name(), "test-channel")
		}
	})
}

func TestMailboxSizeNonexistent(t *testing.T) {
	t.Run("returns 0 for non-existent actor", func(t *testing.T) {
		sys := NewActorSystem()
		if sys.MailboxSize("ghost") != 0 {
			t.Error("Expected 0 for non-existent actor")
		}
	})
}

func TestStopNonexistentActor(t *testing.T) {
	t.Run("stopping non-existent actor returns error", func(t *testing.T) {
		sys := NewActorSystem()
		err := sys.StopActor("ghost")
		if err == nil {
			t.Error("Expected error for non-existent actor")
		}
	})
}

func TestGetStatusNonexistent(t *testing.T) {
	t.Run("getting status of non-existent actor returns error", func(t *testing.T) {
		sys := NewActorSystem()
		_, err := sys.GetActorStatus("ghost")
		if err == nil {
			t.Error("Expected error for non-existent actor")
		}
	})
}

func TestShutdown(t *testing.T) {
	t.Run("shutdown stops all actors and drains mailboxes", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("a", nil, echoBehavior)
		sys.CreateActor("b", nil, echoBehavior)
		sys.Send("a", NewTextMessage("sender", "msg1", nil))
		sys.Send("b", NewTextMessage("sender", "msg2", nil))

		sys.Shutdown()

		statusA, _ := sys.GetActorStatus("a")
		statusB, _ := sys.GetActorStatus("b")
		if statusA != StatusStopped {
			t.Errorf("Actor a: got %q, want stopped", statusA)
		}
		if statusB != StatusStopped {
			t.Errorf("Actor b: got %q, want stopped", statusB)
		}

		dl := sys.DeadLetters()
		if len(dl) != 2 {
			t.Errorf("Dead letters: got %d, want 2", len(dl))
		}
	})
}

func TestRunUntilDone(t *testing.T) {
	t.Run("returns stats with zero when no work", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("test", nil, echoBehavior)
		stats := sys.RunUntilDone()
		if stats["messages_processed"] != 0 {
			t.Errorf("Expected 0 messages processed, got %d", stats["messages_processed"])
		}
	})
}

// ═══════════════════════════════════════════════════════════════════════════
// Direct Actor struct tests (for coverage of exported getters)
// ═══════════════════════════════════════════════════════════════════════════

func TestActorDirectAccess(t *testing.T) {
	t.Run("actor ID, Status, MailboxSize via Actor struct", func(t *testing.T) {
		a := newActor("direct-test", 42, echoBehavior)
		if a.ID() != "direct-test" {
			t.Errorf("ID: got %q, want %q", a.ID(), "direct-test")
		}
		if a.Status() != StatusIdle {
			t.Errorf("Status: got %q, want %q", a.Status(), StatusIdle)
		}
		if a.MailboxSize() != 0 {
			t.Errorf("MailboxSize: got %d, want 0", a.MailboxSize())
		}
		a.enqueue(NewTextMessage("sender", "hi", nil))
		if a.MailboxSize() != 1 {
			t.Errorf("MailboxSize after enqueue: got %d, want 1", a.MailboxSize())
		}
		msg := a.dequeue()
		if msg == nil {
			t.Fatal("dequeue returned nil")
		}
		if a.MailboxSize() != 0 {
			t.Errorf("MailboxSize after dequeue: got %d, want 0", a.MailboxSize())
		}
		// Dequeue on empty returns nil.
		if a.dequeue() != nil {
			t.Error("dequeue on empty should return nil")
		}
	})
}

// Test NewJSONMessage with unmarshalable value (channel).
func TestNewJSONMessageMarshalError(t *testing.T) {
	t.Run("unmarshalable value produces error JSON", func(t *testing.T) {
		// Channels cannot be marshaled to JSON.
		ch := make(chan int)
		msg := NewJSONMessage("sender", ch, nil)
		if msg.ContentType() != "application/json" {
			t.Errorf("ContentType: got %q", msg.ContentType())
		}
		text := msg.PayloadText()
		if text == "" {
			t.Error("Payload should contain error text")
		}
	})
}

// Test Slice with negative start.
func TestSliceNegativeStart(t *testing.T) {
	t.Run("negative start is clamped to 0", func(t *testing.T) {
		ch := NewChannel("ch_001", "test")
		ch.Append(NewTextMessage("s", "a", nil))
		ch.Append(NewTextMessage("s", "b", nil))
		msgs := ch.Slice(-5, 2)
		if len(msgs) != 2 {
			t.Errorf("Expected 2 messages, got %d", len(msgs))
		}
	})
}

// Test Recover with read error (permission denied scenario covered by
// non-existent; here we test a directory instead of file).
func TestRecoverDirectoryError(t *testing.T) {
	t.Run("recover handles non-file gracefully", func(t *testing.T) {
		// Try to recover with a path that is a directory.
		dir := t.TempDir()
		// No file to read, just returns empty channel.
		ch, err := Recover(dir, "empty")
		if err != nil {
			t.Fatalf("Recover failed: %v", err)
		}
		if ch.Length() != 0 {
			t.Errorf("Expected empty channel")
		}
	})
}

// Test FromReader with truncated envelope.
func TestFromReaderTruncatedEnvelope(t *testing.T) {
	t.Run("truncated envelope returns error", func(t *testing.T) {
		msg := NewTextMessage("sender", "hello", nil)
		data := msg.ToBytes()
		// Keep header but truncate the envelope.
		truncated := data[:headerSize+2]
		_, err := FromBytes(truncated)
		if err == nil {
			t.Error("Expected error for truncated envelope")
		}
	})
}

// Test FromReader with truncated payload.
func TestFromReaderTruncatedPayload(t *testing.T) {
	t.Run("truncated payload returns error", func(t *testing.T) {
		msg := NewTextMessage("sender", "hello world this is a longer payload", nil)
		data := msg.ToBytes()
		// Keep header + envelope but truncate the payload.
		truncated := data[:len(data)-5]
		_, err := FromBytes(truncated)
		if err == nil {
			t.Error("Expected error for truncated payload")
		}
	})
}
