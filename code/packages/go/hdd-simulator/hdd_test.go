package hddsimulator

import (
	"bytes"
	"errors"
	"math/rand"
	"sync"
	"testing"
)

type model struct {
	tick  uint64
	head  int
	media [sectorCount][bytesPerSector]byte
}

func modelTiming(m model, lba int) (IOTiming, error) {
	track, sector := lba/8, lba%4
	delta := track - m.head
	if delta < 0 {
		delta = -delta
	}
	if m.tick > maxTick {
		return IOTiming{}, ErrClockOverflow
	}
	clock := m.tick
	step := func() bool {
		if clock == maxTick {
			return false
		}
		clock++
		return true
	}
	seek := uint64(0)
	for trackStep := 0; trackStep < delta; trackStep++ {
		for tick := 0; tick < 3; tick++ {
			if !step() {
				return IOTiming{}, ErrClockOverflow
			}
			seek++
		}
	}
	rotation := uint64(0)
	for int(clock%4) != sector {
		if !step() {
			return IOTiming{}, ErrClockOverflow
		}
		rotation++
	}
	if !step() {
		return IOTiming{}, ErrClockOverflow
	}
	total := seek + rotation + 1
	return IOTiming{m.tick, seek, rotation, 1, total, clock}, nil
}

func modelStep(m *model, op byte, lba int, data []byte) (IOTiming, []byte, error) {
	if lba < 0 || lba >= 32 {
		return IOTiming{}, nil, ErrInvalidLBA
	}
	if op == 'w' && (data == nil || len(data) != 8) {
		return IOTiming{}, nil, ErrInvalidSectorData
	}
	timing, err := modelTiming(*m, lba)
	if err != nil {
		return IOTiming{}, nil, err
	}
	var read []byte
	if op == 'w' {
		copy(m.media[lba][:], data)
	} else {
		read = append([]byte(nil), m.media[lba][:]...)
	}
	m.tick, m.head = timing.EndTick, lba/8
	return timing, read, nil
}

func snapshotModel(m model) Snapshot {
	return Snapshot{Tick: m.tick, HeadTrack: m.head, Sectors: m.media}
}

func TestZeroValueNilAndStatuses(t *testing.T) {
	var d Device
	s, err := d.Snapshot()
	if err != nil || s != (Snapshot{}) {
		t.Fatalf("zero state: %+v, %v", s, err)
	}
	var nilDevice *Device
	if _, err := nilDevice.ReadSector(0); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("nil read: %v", err)
	}
	if _, err := nilDevice.WriteSector(0, make([]byte, 8)); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("nil write: %v", err)
	}
	if _, err := nilDevice.Snapshot(); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("nil snapshot: %v", err)
	}
	for e, want := range map[error]string{ErrInvalidArgument: "invalid API argument", ErrInvalidLBA: "LBA out of range", ErrInvalidSectorData: "sector data must contain exactly 8 bytes", ErrClockOverflow: "clock overflow"} {
		if e.Error() != want {
			t.Errorf("status text %q, want %q", e, want)
		}
	}
}

func TestAllTimingStatesAndLBAs(t *testing.T) {
	for head := 0; head < 4; head++ {
		for target := 0; target < 4; target++ {
			for sector := 0; sector < 4; sector++ {
				for surface := 0; surface < 2; surface++ {
					for phase := 0; phase < 4; phase++ {
						lba := target*8 + surface*4 + sector
						d := New()
						d.headTrack = head
						d.tick = uint64(phase)
						want, err := modelTiming(model{tick: uint64(phase), head: head}, lba)
						if err != nil {
							t.Fatal(err)
						}
						got, err := d.ReadSector(lba)
						if err != nil {
							t.Fatal(err)
						}
						if got.Timing != want || len(got.Data) != 8 || !bytes.Equal(got.Data, make([]byte, 8)) {
							t.Fatalf("h=%d t=%d s=%d surf=%d p=%d got=%+v want=%+v", head, target, sector, surface, phase, got, want)
						}
					}
				}
			}
		}
	}
}

func TestOwnershipAndDetachedSnapshots(t *testing.T) {
	d := New()
	source := []byte{0, 1, 2, 3, 4, 5, 6, 7}
	if _, err := d.WriteSector(11, source); err != nil {
		t.Fatal(err)
	}
	source[0] = 99
	first, err := d.ReadSector(11)
	if err != nil {
		t.Fatal(err)
	}
	if first.Data[0] != 0 {
		t.Fatal("write retained caller buffer")
	}
	first.Data[1] = 88
	snap, err := d.Snapshot()
	if err != nil {
		t.Fatal(err)
	}
	snap.Sectors[11][2] = 77
	snap.Tick = 0
	second, err := d.ReadSector(11)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(second.Data, []byte{0, 1, 2, 3, 4, 5, 6, 7}) {
		t.Fatalf("device bytes aliased: %x", second.Data)
	}
	if second.Timing.StartTick == 0 {
		t.Fatal("snapshot clock mutation affected device")
	}
}

func TestRefusalsAreOrderedZeroAndAtomic(t *testing.T) {
	d := New()
	for _, lba := range []int{-1, 32, -int(^uint(0)>>1) - 1, int(^uint(0) >> 1)} {
		before, _ := d.Snapshot()
		result, err := d.ReadSector(lba)
		if !errors.Is(err, ErrInvalidLBA) || result.Data != nil || result.Timing != (IOTiming{}) {
			t.Fatalf("read %d: %+v %v", lba, result, err)
		}
		for _, payload := range [][]byte{nil, {}, make([]byte, 7), make([]byte, 9)} {
			timing, err := d.WriteSector(lba, payload)
			if !errors.Is(err, ErrInvalidLBA) || timing != (IOTiming{}) {
				t.Fatalf("write priority lba=%d len=%d: %+v %v", lba, len(payload), timing, err)
			}
		}
		after, _ := d.Snapshot()
		if after != before {
			t.Fatalf("invalid LBA mutated state: %d", lba)
		}
	}
	for _, payload := range [][]byte{nil, {}, make([]byte, 7), make([]byte, 9)} {
		before, _ := d.Snapshot()
		timing, err := d.WriteSector(0, payload)
		if !errors.Is(err, ErrInvalidSectorData) || timing != (IOTiming{}) {
			t.Fatalf("payload len=%d: %+v %v", len(payload), timing, err)
		}
		after, _ := d.Snapshot()
		if after != before {
			t.Fatal("bad payload mutated state")
		}
	}
}

func TestClockBoundariesAllHeadsAndTargets(t *testing.T) {
	for head := 0; head < 4; head++ {
		for target := 0; target < 4; target++ {
			for sector := 0; sector < 4; sector++ {
				for surf := 0; surf < 2; surf++ {
					for offset := uint64(0); offset <= 13; offset++ {
						lba := target*8 + surf*4 + sector
						for _, write := range []bool{false, true} {
							d := New()
							d.headTrack = head
							d.tick = maxTick - offset
							m := model{tick: d.tick, head: head}
							before, _ := d.Snapshot()
							payload := []byte{1, 2, 3, 4, 5, 6, 7, 8}
							op := byte('r')
							if write {
								op = 'w'
							}
							wantTiming, _, wantErr := modelStep(&m, op, lba, payload)
							if write {
								got, err := d.WriteSector(lba, payload)
								if !errors.Is(err, wantErr) || got != wantTiming {
									t.Fatalf("write h/t/s/surf/off %d/%d/%d/%d/%d got=%+v %v want=%+v %v", head, target, sector, surf, offset, got, err, wantTiming, wantErr)
								}
							} else {
								got, err := d.ReadSector(lba)
								if !errors.Is(err, wantErr) || got.Timing != wantTiming {
									t.Fatalf("read boundary got=%+v %v want=%+v %v", got, err, wantTiming, wantErr)
								}
							}
							after, _ := d.Snapshot()
							if wantErr != nil {
								if after != before {
									t.Fatal("overflow mutated state")
								}
							} else if after != snapshotModel(m) {
								t.Fatal("valid boundary state mismatch")
							}
						}
					}
				}
			}
		}
	}
}

func TestCorruptedClockRefusesAtomically(t *testing.T) {
	d := New()
	d.tick = maxTick + 1
	before, err := d.Snapshot()
	if err != nil {
		t.Fatal(err)
	}
	read, err := d.ReadSector(0)
	if !errors.Is(err, ErrClockOverflow) || read.Data != nil || read.Timing != (IOTiming{}) {
		t.Fatalf("corrupt-clock read = %+v, %v", read, err)
	}
	timing, err := d.WriteSector(0, []byte("12345678"))
	if !errors.Is(err, ErrClockOverflow) || timing != (IOTiming{}) {
		t.Fatalf("corrupt-clock write = %+v, %v", timing, err)
	}
	after, err := d.Snapshot()
	if err != nil || after != before {
		t.Fatalf("corrupt-clock refusal mutated state: before=%+v after=%+v err=%v", before, after, err)
	}
}

func TestSeededHistoriesAgainstIndependentModel(t *testing.T) {
	rng := rand.New(rand.NewSource(0xDB02))
	d := New()
	m := model{}
	for i := 0; i < 5000; i++ {
		op := byte('r')
		if rng.Intn(2) == 0 {
			op = 'w'
		}
		lba := rng.Intn(40) - 4
		payload := make([]byte, 8)
		rng.Read(payload)
		before, _ := d.Snapshot()
		wantTiming, wantBytes, wantErr := modelStep(&m, op, lba, payload)
		if op == 'w' {
			got, err := d.WriteSector(lba, payload)
			if !errors.Is(err, wantErr) || got != wantTiming {
				t.Fatalf("step %d write mismatch: %+v %v / %+v %v", i, got, err, wantTiming, wantErr)
			}
		} else {
			got, err := d.ReadSector(lba)
			if !errors.Is(err, wantErr) || got.Timing != wantTiming || !bytes.Equal(got.Data, wantBytes) {
				t.Fatalf("step %d read mismatch", i)
			}
		}
		after, _ := d.Snapshot()
		if wantErr != nil {
			if before != after {
				t.Fatalf("step %d refusal mutated state", i)
			}
		} else if after != snapshotModel(m) {
			t.Fatalf("step %d model mismatch", i)
		}
	}
}

func TestBoundedShortHistories(t *testing.T) {
	type command struct {
		op      byte
		lba     int
		payload []byte
	}
	commands := []command{{'r', 0, nil}, {'r', 3, nil}, {'r', 31, nil}, {'w', 0, []byte("abcdefgh")}, {'w', 3, []byte("ABCDEFGH")}, {'w', 31, []byte("12345678")}, {'w', 0, []byte("short")}, {'r', 32, nil}}
	var run func([]command)
	run = func(history []command) {
		d := New()
		m := model{}
		for i, c := range history {
			timing, read, wantErr := modelStep(&m, c.op, c.lba, c.payload)
			before, _ := d.Snapshot()
			if c.op == 'r' {
				got, err := d.ReadSector(c.lba)
				if !errors.Is(err, wantErr) || got.Timing != timing || !bytes.Equal(got.Data, read) {
					t.Fatalf("history %v step %d", history, i)
				}
			} else {
				got, err := d.WriteSector(c.lba, c.payload)
				if !errors.Is(err, wantErr) || got != timing {
					t.Fatalf("history %v step %d", history, i)
				}
			}
			after, _ := d.Snapshot()
			if wantErr != nil {
				if after != before {
					t.Fatal("refusal changed history state")
				}
			} else if after != snapshotModel(m) {
				t.Fatalf("history %v model step %d", history, i)
			}
		}
	}
	var extend func([]command, int)
	extend = func(h []command, left int) {
		if left == 0 {
			run(h)
			return
		}
		for _, c := range commands {
			next := append(append([]command(nil), h...), c)
			extend(next, left-1)
		}
	}
	for n := 1; n <= 4; n++ {
		extend(nil, n)
	}
}

func FuzzMixedCommandStream(f *testing.F) {
	f.Add([]byte{0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 1, 2, 3, 4, 5, 6, 7, 8, 1, 31, 0, 0, 0, 0, 0, 0, 0, 0, 8, 9, 8, 7, 6, 5, 4, 3, 2})
	f.Add([]byte{1, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f, 1, 0})
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > 1024 {
			return
		}
		d := New()
		m := model{}
		for off := 0; off+18 <= len(input); off += 18 {
			op := byte('r')
			if input[off]&1 != 0 {
				op = 'w'
			}
			var encoded uint64
			for i := 0; i < 8; i++ {
				encoded |= uint64(input[off+1+i]) << (8 * i)
			}
			lba := int(int64(encoded))
			payloadLen := int(input[off+9] % 11)
			if input[off+10]&1 == 0 && payloadLen == 8 {
				payloadLen = 0
			}
			payload := make([]byte, payloadLen)
			copy(payload, input[off+10:off+18])
			before, _ := d.Snapshot()
			timing, wantData, wantErr := modelStep(&m, op, lba, payload)
			if op == 'w' {
				got, err := d.WriteSector(lba, payload)
				if !errors.Is(err, wantErr) || got != timing {
					t.Fatalf("write %d: %v/%+v want %v/%+v", lba, err, got, wantErr, timing)
				}
			} else {
				got, err := d.ReadSector(lba)
				if !errors.Is(err, wantErr) || got.Timing != timing || !bytes.Equal(got.Data, wantData) {
					t.Fatalf("read %d mismatch", lba)
				}
			}
			after, _ := d.Snapshot()
			if wantErr != nil {
				if after != before {
					t.Fatal("refusal mutated state")
				}
			} else if after != snapshotModel(m) {
				t.Fatal("model mismatch")
			}
		}
	})
}

func TestSeparateDevicesAreRaceSafe(t *testing.T) {
	var wg sync.WaitGroup
	for g := 0; g < 8; g++ {
		wg.Add(1)
		go func(seed int) {
			defer wg.Done()
			d := New()
			for i := 0; i < 100; i++ {
				data := bytes.Repeat([]byte{byte(seed + i)}, 8)
				if _, err := d.WriteSector((seed+i)%32, data); err != nil {
					t.Error(err)
				}
				got, err := d.ReadSector((seed + i) % 32)
				if err != nil {
					t.Error(err)
				}
				if len(got.Data) != 8 || got.Data[0] != byte(seed+i) {
					t.Errorf("read back wrong bytes: %x", got.Data)
				}
			}
		}(g)
	}
	wg.Wait()
}
