//go:build linux && amd64

package exechead

import "testing"

// The pending-lifetime bound must not kill (or taint) an exec whose thread has
// just made a NEW execve (D48 step 4's review, closed by step 5): that
// notification is about to settle the old exec at receipt.  inbox.holds is the
// fact the bound reads: a notification from the tid is waiting OR being
// processed, as one fact under one lock.
func TestInboxHoldsAWaitingAndAProcessedNotification(t *testing.T) {
	var b inbox
	b.init()
	mk := func(tid uint32) event { return event{kind: evNotif, n: seccompNotif{Pid: tid}} }

	if b.holds(7) {
		t.Fatal("an empty inbox holds nothing")
	}
	b.push(mk(7), 8)
	b.push(mk(9), 8)
	if !b.holds(7) || !b.holds(9) || b.holds(8) {
		t.Fatal("a waiting notification is held, and only its own tid's")
	}

	// Taken by the processor: no longer waiting, still "being processed".
	e, ok := b.pop()
	if !ok || e.n.Pid != 7 {
		t.Fatalf("popped %+v", e)
	}
	if !b.holds(7) {
		t.Fatal("a notification being processed must still be held: there is no instant in which it is in neither place")
	}
	if !b.holds(9) {
		t.Fatal("the second notification is still waiting")
	}
	b.done()
	if b.holds(7) {
		t.Fatal("a finished notification is not held")
	}

	// A taint event carries no notification, whatever its zero tid.
	b.pushFront([]event{{kind: evTaint}})
	if b.holds(0) {
		t.Fatal("a taint event is not a notification of tid 0")
	}
	// A released (held-then-handed-back) notification counts like a received one.
	b.pushFront([]event{{kind: evReleased, n: seccompNotif{Pid: 11}}})
	if !b.holds(11) {
		t.Fatal("a released notification is a notification")
	}
}
