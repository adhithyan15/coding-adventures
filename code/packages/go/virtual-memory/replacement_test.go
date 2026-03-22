package virtualmemory

import "testing"

func TestFIFOPolicy(t *testing.T) {
	t.Run("empty", func(t *testing.T) {
		fifo := NewFIFOPolicy()
		if fifo.SelectVictim() != -1 {
			t.Error("empty FIFO should return -1")
		}
	})

	t.Run("evicts oldest", func(t *testing.T) {
		fifo := NewFIFOPolicy()
		fifo.AddFrame(10)
		fifo.AddFrame(20)
		fifo.AddFrame(30)

		if v := fifo.SelectVictim(); v != 10 {
			t.Errorf("victim = %d, want 10", v)
		}
		if v := fifo.SelectVictim(); v != 20 {
			t.Errorf("victim = %d, want 20", v)
		}
	})

	t.Run("access does not change order", func(t *testing.T) {
		fifo := NewFIFOPolicy()
		fifo.AddFrame(10)
		fifo.AddFrame(20)
		fifo.RecordAccess(10) // no effect

		if v := fifo.SelectVictim(); v != 10 {
			t.Error("FIFO should ignore access events")
		}
	})

	t.Run("remove frame", func(t *testing.T) {
		fifo := NewFIFOPolicy()
		fifo.AddFrame(10)
		fifo.AddFrame(20)
		fifo.AddFrame(30)
		fifo.RemoveFrame(20)

		if v := fifo.SelectVictim(); v != 10 {
			t.Error("should skip removed frame")
		}
		if v := fifo.SelectVictim(); v != 30 {
			t.Error("should get 30 after 10")
		}
	})

	t.Run("remove nonexistent", func(t *testing.T) {
		fifo := NewFIFOPolicy()
		fifo.RemoveFrame(999) // should not panic
	})
}

func TestLRUPolicy(t *testing.T) {
	t.Run("empty", func(t *testing.T) {
		lru := NewLRUPolicy()
		if lru.SelectVictim() != -1 {
			t.Error("empty LRU should return -1")
		}
	})

	t.Run("evicts least recently used", func(t *testing.T) {
		lru := NewLRUPolicy()
		lru.AddFrame(10) // time 0
		lru.AddFrame(20) // time 1
		lru.AddFrame(30) // time 2

		if v := lru.SelectVictim(); v != 10 {
			t.Errorf("victim = %d, want 10", v)
		}
	})

	t.Run("access changes order", func(t *testing.T) {
		lru := NewLRUPolicy()
		lru.AddFrame(10) // time 0
		lru.AddFrame(20) // time 1
		lru.AddFrame(30) // time 2

		lru.RecordAccess(10) // time 3 — now 10 is most recent

		if v := lru.SelectVictim(); v != 20 {
			t.Errorf("victim = %d, want 20", v)
		}
	})

	t.Run("remove frame", func(t *testing.T) {
		lru := NewLRUPolicy()
		lru.AddFrame(10)
		lru.AddFrame(20)
		lru.RemoveFrame(10)

		if v := lru.SelectVictim(); v != 20 {
			t.Errorf("victim = %d, want 20", v)
		}
	})

	t.Run("remove nonexistent", func(t *testing.T) {
		lru := NewLRUPolicy()
		lru.RemoveFrame(999) // should not panic
	})
}

func TestClockPolicy(t *testing.T) {
	t.Run("empty", func(t *testing.T) {
		clock := NewClockPolicy()
		if clock.SelectVictim() != -1 {
			t.Error("empty Clock should return -1")
		}
	})

	t.Run("evicts cleared frame", func(t *testing.T) {
		clock := NewClockPolicy()
		clock.AddFrame(10)
		clock.AddFrame(20)
		clock.AddFrame(30)

		clock.useBits[10] = false

		if v := clock.SelectVictim(); v != 10 {
			t.Errorf("victim = %d, want 10", v)
		}
	})

	t.Run("second chance clears bit", func(t *testing.T) {
		clock := NewClockPolicy()
		clock.AddFrame(10) // use=true
		clock.AddFrame(20) // use=true

		clock.useBits[20] = false

		// Hand at 0: frame 10 use=true -> clear, move.
		// Hand at 1: frame 20 use=false -> evict.
		if v := clock.SelectVictim(); v != 20 {
			t.Errorf("victim = %d, want 20", v)
		}
		if clock.useBits[10] {
			t.Error("frame 10 use bit should be cleared (second chance)")
		}
	})

	t.Run("all set wraps around", func(t *testing.T) {
		clock := NewClockPolicy()
		clock.AddFrame(10)
		clock.AddFrame(20)
		clock.AddFrame(30)

		victim := clock.SelectVictim()
		if victim != 10 {
			t.Errorf("victim = %d, want 10", victim)
		}
	})

	t.Run("access sets use bit", func(t *testing.T) {
		clock := NewClockPolicy()
		clock.AddFrame(10)
		clock.AddFrame(20)

		clock.useBits[10] = false
		clock.useBits[20] = false

		clock.RecordAccess(10) // set use bit

		if v := clock.SelectVictim(); v != 20 {
			t.Errorf("victim = %d, want 20", v)
		}
	})

	t.Run("remove frame", func(t *testing.T) {
		clock := NewClockPolicy()
		clock.AddFrame(10)
		clock.AddFrame(20)
		clock.AddFrame(30)

		clock.RemoveFrame(20)

		clock.useBits[10] = false
		clock.useBits[30] = false

		if v := clock.SelectVictim(); v != 10 {
			t.Errorf("first victim = %d, want 10", v)
		}
		if v := clock.SelectVictim(); v != 30 {
			t.Errorf("second victim = %d, want 30", v)
		}
	})

	t.Run("remove nonexistent", func(t *testing.T) {
		clock := NewClockPolicy()
		clock.RemoveFrame(999) // should not panic
	})

	t.Run("sequential evictions", func(t *testing.T) {
		clock := NewClockPolicy()
		for i := 0; i < 5; i++ {
			clock.AddFrame(i)
		}
		for i := 0; i < 5; i++ {
			clock.useBits[i] = false
		}
		for i := 0; i < 5; i++ {
			if v := clock.SelectVictim(); v != i {
				t.Errorf("victim = %d, want %d", v, i)
			}
		}
	})
}
