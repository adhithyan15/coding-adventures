package hddwritebackcache

import (
	"errors"
	"fmt"
	"math"
	"math/rand"
	"testing"

	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
)

func TestFlushDirtyMatchesIndependentArrayModelAcrossSubsetsAndCuts(t *testing.T) {
	lowerCuts := []hddwritecrashcuts.Cut{
		{Kind: hddwritecrashcuts.BeforeMediaWrite},
		{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: intPointer(1)},
		{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: intPointer(7)},
		{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse},
	}
	decisions := 0
	for mask := 0; mask < 1<<5; mask++ {
		var initial [SectorCount][BytesPerSector]byte
		state := State{}
		expected := model{}
		for lba := 0; lba < 5; lba++ {
			initial[lba] = [BytesPerSector]byte{byte(lba + 10), byte(lba + 20), byte(lba + 30)}
		}
		state.durable = initial
		expected.durable = initial
		for lba := 4; lba >= 0; lba-- {
			if mask&(1<<lba) == 0 {
				continue
			}
			data := [BytesPerSector]byte{byte(200 + lba), byte(150 + lba), byte(100 + lba), byte(50 + lba)}
			var err error
			state, err = state.WriteSector(int64(lba), data[:])
			if err != nil {
				t.Fatal(err)
			}
			expected.pending[lba] = data
			expected.dirty[lba] = true
		}
		cuts := []BarrierCut{{Kind: BeforeFirstSector}, {Kind: AfterAllMedia}, {Kind: BarrierComplete}}
		ordinal := int64(0)
		for lba := 0; lba < 5; lba++ {
			if mask&(1<<lba) == 0 {
				continue
			}
			for _, mediaCut := range lowerCuts {
				cuts = append(cuts, BarrierCut{Kind: AtSector, SectorIndex: ordinal, MediaCut: mediaCut})
			}
			ordinal++
		}
		for index, cut := range cuts {
			t.Run(fmt.Sprintf("mask_%02x_cut_%02d", mask, index), func(t *testing.T) {
				assertBarrierMatchesModel(t, state, expected, cut)
			})
			decisions++
		}
	}
	if decisions != 32*3+80*4 {
		t.Fatalf("barrier model decisions=%d", decisions)
	}
}

func TestFlushDirtyFullSetUsesAscendingLBARegardlessOfArrivalOrder(t *testing.T) {
	state, expected := State{}, model{}
	for lba := SectorCount - 1; lba >= 0; lba-- {
		data := [BytesPerSector]byte{byte(lba), byte(255 - lba), 3, 4, 5, 6, 7, 8}
		var err error
		state, err = state.WriteSector(int64(lba), data[:])
		if err != nil {
			t.Fatal(err)
		}
		expected.pending[lba] = data
		expected.dirty[lba] = true
	}
	for _, cut := range []BarrierCut{
		{Kind: AtSector, SectorIndex: 0, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.BeforeMediaWrite}},
		{Kind: AtSector, SectorIndex: SectorCount - 1, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}},
		{Kind: BarrierComplete},
	} {
		assertBarrierMatchesModel(t, state, expected, cut)
	}
}

func TestFlushDirtyCutValidationPrecedenceAndAtomicity(t *testing.T) {
	state, err := (State{}).WriteSector(3, []byte{1, 2, 3, 4, 5, 6, 7, 8})
	if err != nil {
		t.Fatal(err)
	}
	invalidPrefix := 0
	invalidCuts := []struct {
		name string
		cut  BarrierCut
		want error
	}{
		{"unknown kind", BarrierCut{Kind: 255}, ErrInvalidBarrierCut},
		{"negative ordinal before lower validation", BarrierCut{Kind: AtSector, SectorIndex: -1, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.CutKind("unknown")}}, ErrInvalidBarrierCut},
		{"ordinal after set", BarrierCut{Kind: AtSector, SectorIndex: 1}, ErrInvalidBarrierCut},
		{"minimum signed ordinal", BarrierCut{Kind: AtSector, SectorIndex: math.MinInt64}, ErrInvalidBarrierCut},
		{"maximum signed ordinal", BarrierCut{Kind: AtSector, SectorIndex: math.MaxInt64}, ErrInvalidBarrierCut},
		{"lower invalid kind", BarrierCut{Kind: AtSector, SectorIndex: 0, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.CutKind("unknown")}}, hddwritecrashcuts.ErrInvalidCut},
		{"lower invalid prefix", BarrierCut{Kind: AtSector, SectorIndex: 0, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &invalidPrefix}}, hddwritecrashcuts.ErrInvalidPrefix},
		{"lower acknowledgement", BarrierCut{Kind: AtSector, SectorIndex: 0, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}}, ErrInvalidBarrierCut},
		{"acknowledgement dominates malformed prefix", BarrierCut{Kind: AtSector, SectorIndex: 0, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged, WrittenPrefix: &invalidPrefix}}, ErrInvalidBarrierCut},
		{"before unused ordinal", BarrierCut{Kind: BeforeFirstSector, SectorIndex: 1}, ErrInvalidBarrierCut},
		{"after unused lower cut", BarrierCut{Kind: AfterAllMedia, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}}, ErrInvalidBarrierCut},
		{"complete unused ordinal", BarrierCut{Kind: BarrierComplete, SectorIndex: 1}, ErrInvalidBarrierCut},
	}
	for _, tc := range invalidCuts {
		t.Run(tc.name, func(t *testing.T) {
			got, receipt, gotErr := state.FlushDirty(tc.cut)
			if !errors.Is(gotErr, tc.want) || got != state || receipt != (BarrierReceipt{}) {
				t.Fatalf("refusal stateChanged=%v receipt=%+v error=%v; want %v and zero result", got != state, receipt, gotErr, tc.want)
			}
		})
	}
	clean := State{}
	for _, cut := range []BarrierCut{{Kind: BeforeFirstSector}, {Kind: AfterAllMedia}, {Kind: BarrierComplete}} {
		got, receipt, gotErr := clean.FlushDirty(cut)
		want := BarrierReceipt{Acknowledged: cut.Kind == BarrierComplete}
		if gotErr != nil || got != clean || receipt != want {
			t.Fatalf("empty barrier %#v = stateChanged=%v receipt=%+v err=%v; want %+v", cut, got != clean, receipt, gotErr, want)
		}
	}
	got, receipt, gotErr := clean.FlushDirty(BarrierCut{Kind: AtSector})
	if gotErr != ErrInvalidBarrierCut || got != clean || receipt != (BarrierReceipt{}) {
		t.Fatalf("empty AtSector = %v %+v %v", got != clean, receipt, gotErr)
	}
}

func assertBarrierMatchesModel(t *testing.T, actual State, expected model, cut BarrierCut) {
	t.Helper()
	got, gotReceipt, gotErr := actual.FlushDirty(cut)
	want, wantReceipt, wantErr := modelFlushDirty(expected, cut)
	if errorName(gotErr) != wantErr || got != stateOf(want) || gotReceipt != wantReceipt {
		t.Fatalf("FlushDirty(%+v): actual error=%s receipt=%+v state=%+v; want error=%s receipt=%+v state=%+v", cut, errorName(gotErr), gotReceipt, got, wantErr, wantReceipt, stateOf(want))
	}
	if actual != stateOf(expected) {
		t.Fatal("barrier modified its predecessor")
	}
}

func modelFlushDirty(state model, cut BarrierCut) (model, BarrierReceipt, string) {
	var dirty []int
	for lba, present := range state.dirty {
		if present {
			dirty = append(dirty, lba)
		}
	}
	invalid := func() (model, BarrierReceipt, string) { return state, BarrierReceipt{}, "invalid_barrier_cut" }
	switch cut.Kind {
	case BeforeFirstSector, AfterAllMedia, BarrierComplete:
		if cut.SectorIndex != 0 || cut.MediaCut != (hddwritecrashcuts.Cut{}) {
			return invalid()
		}
	case AtSector:
		if cut.SectorIndex < 0 || cut.SectorIndex >= int64(len(dirty)) {
			return invalid()
		}
		if cut.MediaCut.Kind == hddwritecrashcuts.Acknowledged {
			return invalid()
		}
		_, ack, lowerError := modelCut(cut.MediaCut)
		if lowerError != "" {
			return state, BarrierReceipt{}, lowerError
		}
		if ack {
			return invalid()
		}
	default:
		return invalid()
	}
	if cut.Kind == BeforeFirstSector {
		state.pending = [SectorCount][BytesPerSector]byte{}
		state.dirty = [SectorCount]bool{}
		return state, BarrierReceipt{}, ""
	}
	receipt := BarrierReceipt{}
	for ordinal, lba := range dirty {
		prefix, ack := BytesPerSector, true
		if cut.Kind == AtSector && int64(ordinal) == cut.SectorIndex {
			prefix, ack, _ = modelCut(cut.MediaCut)
		}
		copy(state.durable[lba][:prefix], state.pending[lba][:prefix])
		if cut.Kind == AtSector && int64(ordinal) == cut.SectorIndex {
			receipt.TerminalPrefixBytes = prefix
			if !ack {
				state.pending = [SectorCount][BytesPerSector]byte{}
				state.dirty = [SectorCount]bool{}
			}
			break
		}
		receipt.PriorCompletedSectors++
		state.pending[lba] = [BytesPerSector]byte{}
		state.dirty[lba] = false
	}
	receipt.Acknowledged = cut.Kind == BarrierComplete
	return state, receipt, ""
}

func TestSeededDirtyBarrierHistoriesMatchArrayModel(t *testing.T) {
	const seed int64 = 0xDB06B1
	random := rand.New(rand.NewSource(seed))
	decisions := 0
	for history := 0; history < 128; history++ {
		actual, expected := State{}, model{}
		for step := 0; step < 80; step++ {
			lba := int64(random.Intn(SectorCount))
			switch random.Intn(4) {
			case 0:
				var data [BytesPerSector]byte
				for i := range data {
					data[i] = byte(random.Intn(256))
				}
				var err error
				actual, err = actual.WriteSector(lba, data[:])
				if err != nil {
					t.Fatal(err)
				}
				expected.pending[lba] = data
				expected.dirty[lba] = true
			case 1:
				cut := BarrierCut{Kind: BarrierComplete}
				got, gotReceipt, gotErr := actual.FlushDirty(cut)
				want, wantReceipt, wantErr := modelFlushDirty(expected, cut)
				if errorName(gotErr) != wantErr || got != stateOf(want) || gotReceipt != wantReceipt {
					t.Fatalf("seed=%x history=%d step=%d barrier: got %s %+v want %s %+v", seed, history, step, errorName(gotErr), gotReceipt, wantErr, wantReceipt)
				}
				actual, expected = got, want
			case 2:
				cut := BarrierCut{Kind: BeforeFirstSector}
				got, gotReceipt, gotErr := actual.FlushDirty(cut)
				want, wantReceipt, wantErr := modelFlushDirty(expected, cut)
				if errorName(gotErr) != wantErr || got != stateOf(want) || gotReceipt != wantReceipt {
					t.Fatalf("seed=%x history=%d step=%d power cut: got %s %+v want %s %+v", seed, history, step, errorName(gotErr), gotReceipt, wantErr, wantReceipt)
				}
				actual, expected = got, want
			default:
				ordinal := int64(random.Intn(SectorCount + 1))
				mediaCut := hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.BeforeMediaWrite}
				if random.Intn(2) == 0 {
					prefix := random.Intn(BytesPerSector-1) + 1
					mediaCut = hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &prefix}
				}
				cut := BarrierCut{Kind: AtSector, SectorIndex: ordinal, MediaCut: mediaCut}
				got, gotReceipt, gotErr := actual.FlushDirty(cut)
				want, wantReceipt, wantErr := modelFlushDirty(expected, cut)
				if errorName(gotErr) != wantErr || got != stateOf(want) || gotReceipt != wantReceipt {
					t.Fatalf("seed=%x history=%d step=%d cut=%+v: got %s %+v want %s %+v", seed, history, step, cut, errorName(gotErr), gotReceipt, wantErr, wantReceipt)
				}
				actual, expected = got, want
			}
			if actual != stateOf(expected) {
				t.Fatalf("seed=%x history=%d step=%d: state differs from model", seed, history, step)
			}
			decisions++
		}
	}
	if decisions != 10_240 {
		t.Fatalf("seeded history decisions=%d", decisions)
	}
}

func FuzzDirtyBarrierHistories(f *testing.F) {
	f.Add([]byte{1, 0, 2, 1, 1, 2, 3, 4, 3, 0, 0, 0, 3, 1, 3, 0})
	f.Add([]byte{0, 3, 9, 7, 0, 1, 8, 6, 3, 4, 1, 2, 3, 5, 4, 1})
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > 256 {
			input = input[:256]
		}
		actual, expected := State{}, model{}
		for cursor := 0; cursor+3 < len(input); cursor += 4 {
			raw := input[cursor : cursor+4]
			lba := int64(raw[1] % SectorCount)
			switch raw[0] % 4 {
			case 0:
				var data [BytesPerSector]byte
				for i := range data {
					data[i] = raw[2] ^ raw[3] ^ byte(i*31)
				}
				var err error
				actual, err = actual.WriteSector(lba, data[:])
				if err != nil {
					t.Fatal(err)
				}
				expected.pending[lba] = data
				expected.dirty[lba] = true
			case 1:
				cut := BarrierCut{Kind: BeforeFirstSector}
				actual, _, _ = actual.FlushDirty(cut)
				expected, _, _ = modelFlushDirty(expected, cut)
			case 2:
				cut := BarrierCut{Kind: BarrierComplete}
				got, receipt, err := actual.FlushDirty(cut)
				want, wantReceipt, wantErr := modelFlushDirty(expected, cut)
				if errorName(err) != wantErr || got != stateOf(want) || receipt != wantReceipt {
					t.Fatalf("complete mismatch: %s %+v want %s %+v", errorName(err), receipt, wantErr, wantReceipt)
				}
				actual, expected = got, want
			default:
				kind := BarrierCutKind(raw[2] % 5)
				ordinal := int64(int8(raw[3]))
				lowerKinds := []hddwritecrashcuts.CutKind{
					hddwritecrashcuts.BeforeMediaWrite,
					hddwritecrashcuts.DuringMediaWrite,
					hddwritecrashcuts.AfterMediaWriteBeforeResponse,
					hddwritecrashcuts.Acknowledged,
					"unknown",
				}
				lower := hddwritecrashcuts.Cut{Kind: lowerKinds[int(raw[0]/4)%len(lowerKinds)]}
				if lower.Kind == hddwritecrashcuts.DuringMediaWrite {
					prefix := int(raw[1] % 9)
					lower.WrittenPrefix = &prefix
				}
				cut := BarrierCut{Kind: kind, SectorIndex: ordinal, MediaCut: lower}
				got, receipt, err := actual.FlushDirty(cut)
				want, wantReceipt, wantErr := modelFlushDirty(expected, cut)
				if errorName(err) != wantErr || got != stateOf(want) || receipt != wantReceipt {
					t.Fatalf("barrier mismatch: %+v got %s %+v want %s %+v", cut, errorName(err), receipt, wantErr, wantReceipt)
				}
				actual, expected = got, want
			}
			if actual != stateOf(expected) {
				t.Fatal("fuzz state differs from independent model")
			}
		}
	})
}
