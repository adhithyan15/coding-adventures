package actor

import (
	"fmt"
	"testing"
)

// ═══════════════════════════════════════════════════════════════════════════
// ActorSystem integration tests
//
// These tests exercise the ActorSystem as a whole — creating actors,
// routing messages, processing round-robin, and managing channels.
// They complement the unit tests in actor_test.go (which test individual
// actor behaviors) and message_test.go / channel_test.go (which test
// primitives in isolation).
// ═══════════════════════════════════════════════════════════════════════════

// TestActorSystemCreateAndQuery verifies the basic lifecycle operations.
func TestActorSystemCreateAndQuery(t *testing.T) {
	t.Run("create actor returns its ID", func(t *testing.T) {
		sys := NewActorSystem()
		id, err := sys.CreateActor("my-actor", nil, echoBehavior)
		if err != nil {
			t.Fatalf("CreateActor failed: %v", err)
		}
		if id != "my-actor" {
			t.Errorf("ID: got %q, want %q", id, "my-actor")
		}
	})

	t.Run("actor IDs are listed", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("a", nil, echoBehavior)
		sys.CreateActor("b", nil, echoBehavior)
		ids := sys.ActorIDs()
		if len(ids) != 2 {
			t.Errorf("Expected 2 actor IDs, got %d", len(ids))
		}
	})
}

// TestActorSystemMessageRouting verifies message delivery semantics.
func TestActorSystemMessageRouting(t *testing.T) {
	t.Run("message delivered to correct actor", func(t *testing.T) {
		sys := NewActorSystem()
		var receivedBy string
		behavior := func(state interface{}, msg *Message) (*ActorResult, error) {
			receivedBy = "actor-a"
			return &ActorResult{NewState: state}, nil
		}
		sys.CreateActor("actor-a", nil, behavior)
		sys.CreateActor("actor-b", nil, func(state interface{}, msg *Message) (*ActorResult, error) {
			receivedBy = "actor-b"
			return &ActorResult{NewState: state}, nil
		})

		sys.Send("actor-a", NewTextMessage("user", "targeted", nil))
		sys.ProcessNext("actor-a")

		if receivedBy != "actor-a" {
			t.Errorf("Message delivered to %q, expected actor-a", receivedBy)
		}
	})
}

// TestActorSystemRoundRobin verifies that RunUntilIdle processes actors
// fairly when multiple actors have pending messages.
func TestActorSystemRoundRobin(t *testing.T) {
	t.Run("all actors get processing time", func(t *testing.T) {
		sys := NewActorSystem()
		counts := make(map[string]int)

		for i := 0; i < 3; i++ {
			id := fmt.Sprintf("actor-%d", i)
			localID := id // capture for closure
			sys.CreateActor(id, nil, func(state interface{}, msg *Message) (*ActorResult, error) {
				counts[localID]++
				return &ActorResult{NewState: state}, nil
			})
			// Give each actor 5 messages.
			for j := 0; j < 5; j++ {
				sys.Send(id, NewTextMessage("sender", "work", nil))
			}
		}

		sys.RunUntilIdle()

		for i := 0; i < 3; i++ {
			id := fmt.Sprintf("actor-%d", i)
			if counts[id] != 5 {
				t.Errorf("Actor %s processed %d messages, want 5", id, counts[id])
			}
		}
	})
}

// TestActorSystemDeadLetterAccumulation verifies dead letters accumulate
// from multiple failure scenarios.
func TestActorSystemDeadLetterAccumulation(t *testing.T) {
	t.Run("dead letters from multiple sources", func(t *testing.T) {
		sys := NewActorSystem()

		// Send to nonexistent actor (1 dead letter).
		sys.Send("ghost", NewTextMessage("sender", "msg1", nil))

		// Create and stop an actor with pending messages (2 dead letters).
		sys.CreateActor("temp", nil, echoBehavior)
		sys.Send("temp", NewTextMessage("sender", "a", nil))
		sys.Send("temp", NewTextMessage("sender", "b", nil))
		sys.StopActor("temp")

		// Send to stopped actor (1 more dead letter).
		sys.Send("temp", NewTextMessage("sender", "c", nil))

		dl := sys.DeadLetters()
		if len(dl) != 4 {
			t.Errorf("Dead letters: got %d, want 4", len(dl))
		}
	})
}

// TestActorSystemChannelOperations verifies channel management.
func TestActorSystemChannelOperations(t *testing.T) {
	t.Run("create and retrieve channel", func(t *testing.T) {
		sys := NewActorSystem()
		ch := sys.CreateChannel("ch_1", "events")

		retrieved := sys.GetChannel("ch_1")
		if retrieved == nil {
			t.Fatal("Channel not found after creation")
		}
		if retrieved.Name() != "events" {
			t.Errorf("Channel name: got %q, want %q", retrieved.Name(), "events")
		}
		if ch.ID() != retrieved.ID() {
			t.Error("Channel IDs should match")
		}
	})

	t.Run("multiple channels are independent", func(t *testing.T) {
		sys := NewActorSystem()
		ch1 := sys.CreateChannel("ch_1", "alpha")
		ch2 := sys.CreateChannel("ch_2", "beta")

		ch1.Append(NewTextMessage("s", "msg1", nil))
		ch2.Append(NewTextMessage("s", "msg2", nil))
		ch2.Append(NewTextMessage("s", "msg3", nil))

		if ch1.Length() != 1 {
			t.Errorf("ch1 length: got %d, want 1", ch1.Length())
		}
		if ch2.Length() != 2 {
			t.Errorf("ch2 length: got %d, want 2", ch2.Length())
		}
	})
}

// TestActorSystemBehaviorError verifies error handling in the processing loop.
func TestActorSystemBehaviorError(t *testing.T) {
	t.Run("behavior error does not propagate to ProcessNext", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("failer", 0, failingBehavior)
		sys.Send("failer", NewTextMessage("sender", "fail", nil))

		// ProcessNext should succeed (return true) even though the behavior
		// returned an error. The error is handled internally.
		processed, err := sys.ProcessNext("failer")
		if err != nil {
			t.Errorf("ProcessNext should not return error: %v", err)
		}
		if !processed {
			t.Error("ProcessNext should return true (message was dequeued)")
		}
	})
}

// TestActorSystemStopDuringProcessing verifies that an actor can stop itself.
func TestActorSystemStopDuringProcessing(t *testing.T) {
	t.Run("actor stops itself and drains remaining messages", func(t *testing.T) {
		sys := NewActorSystem()
		sys.CreateActor("stopper", nil, stopBehavior)

		sys.Send("stopper", NewTextMessage("sender", "keep going", nil))
		sys.Send("stopper", NewTextMessage("sender", "stop", nil))
		sys.Send("stopper", NewTextMessage("sender", "never processed", nil))

		sys.RunUntilDone()

		status, _ := sys.GetActorStatus("stopper")
		if status != StatusStopped {
			t.Errorf("Status: got %q, want stopped", status)
		}

		// The third message should be in dead letters (drained after stop).
		dl := sys.DeadLetters()
		found := false
		for _, d := range dl {
			if d.PayloadText() == "never processed" {
				found = true
			}
		}
		if !found {
			t.Error("Third message should be in dead letters after stop")
		}
	})
}
