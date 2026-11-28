package hddwritebackcache

import (
	"bytes"
	"errors"
	"fmt"
	"math"
	"math/rand"
	"testing"

	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
)

type model struct {
	durable [SectorCount][BytesPerSector]byte
	pending [SectorCount][BytesPerSector]byte
	dirty   [SectorCount]bool
}

type command struct {
	kind byte
	lba  int64
	data [BytesPerSector]byte
	cut  hddwritecrashcuts.Cut
}

func TestEveryCutPreservesExactPrefixAndReceipt(t *testing.T) {
	old := [BytesPerSector]byte{0xF0, 0x0F, 0xAA, 0x55, 0xCC, 0x33, 0x96, 0x69}
	newData := [BytesPerSector]byte{0x0F, 0xF0, 0x55, 0xAA, 0x33, 0xCC, 0x69, 0x96}
	cuts := []hddwritecrashcuts.Cut{{Kind: hddwritecrashcuts.BeforeMediaWrite}}
	for prefix := 1; prefix < BytesPerSector; prefix++ {
		value := prefix
		cuts = append(cuts, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &value})
	}
	cuts = append(cuts, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged})
	for index, cut := range cuts {
		t.Run(fmt.Sprintf("cut_%02d", index), func(t *testing.T) {
			state := State{durable: [SectorCount][BytesPerSector]byte{old}}
			state.pending[2] = [BytesPerSector]byte{1, 3, 5, 7, 9, 11, 13, 15}
			state.dirty[2] = true
			pendingInput := append([]byte(nil), newData[:]...)
			state, err := state.WriteSector(0, pendingInput)
			if err != nil {
				t.Fatal(err)
			}
			before := state
			got, receipt, err := state.FlushSector(0, cut)
			if err != nil {
				t.Fatal(err)
			}
			if receipt.Acknowledged != (index == len(cuts)-1) {
				t.Fatalf("acknowledgement=%v for cut %#v", receipt.Acknowledged, cut)
			}
			wantState, wantReceipt := modelFlush(modelOf(before), 0, newData, cut)
			if got != stateOf(wantState) || receipt != wantReceipt {
				t.Fatalf("got state=%+v result=%+v; want state=%+v result=%+v", got, receipt, stateOf(wantState), wantReceipt)
			}
			if state != before {
				t.Fatal("flush modified its predecessor")
			}
		})
	}
}

func TestWriteVisibilityOverwriteAndPowerLoss(t *testing.T) {
	state := State{}
	first := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	second := []byte{8, 7, 6, 5, 4, 3, 2, 1}
	initial := state
	next, err := state.WriteSector(4, first)
	if err != nil {
		t.Fatal(err)
	}
	first[0] = 99
	state = next
	if initial != (State{}) {
		t.Fatal("write modified zero predecessor")
	}
	visible, err := state.ReadSector(4)
	if err != nil || visible != [BytesPerSector]byte{1, 2, 3, 4, 5, 6, 7, 8} {
		t.Fatalf("pending read = %v, %v", visible, err)
	}
	visible[0] = 77
	visibleAgain, err := state.ReadSector(4)
	if err != nil || visibleAgain[0] != 1 {
		t.Fatal("read result aliases state")
	}
	state, err = state.WriteSector(4, second)
	if err != nil {
		t.Fatal(err)
	}
	snapshot := state.Snapshot()
	if snapshot.Durable[4] != [BytesPerSector]byte{} || snapshot.Visible[4] != [BytesPerSector]byte{8, 7, 6, 5, 4, 3, 2, 1} || !snapshot.Dirty[4] {
		t.Fatalf("wrong overlay snapshot: %+v", snapshot)
	}
	snapshot.Visible[4][0] = 0
	read, _ := state.ReadSector(4)
	if read[0] != 8 {
		t.Fatal("snapshot bytes alias state")
	}
	crashed := state.PowerLoss()
	if crashed.Snapshot().Durable != [SectorCount][BytesPerSector]byte{} || crashed.Snapshot().Dirty != [SectorCount]bool{} {
		t.Fatalf("pending bytes survived power loss: %+v", crashed.Snapshot())
	}
	if state.Snapshot().Visible[4][0] != 8 {
		t.Fatal("power loss modified predecessor")
	}
	if crashed.PowerLoss() != crashed {
		t.Fatal("successive power loss changed an already-clean state")
	}
}

func TestFlushAcknowledgesOnlySelectedEntryAndCrashDropsAll(t *testing.T) {
	state := State{}
	state, _ = state.WriteSector(5, []byte{0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88})
	state, _, _ = state.FlushSector(5, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged})
	state, _ = state.WriteSector(0, []byte{1, 3, 5, 7, 9, 11, 13, 15})
	state, _ = state.WriteSector(1, []byte{2, 4, 6, 8, 10, 12, 14, 16})
	ack, result, err := state.FlushSector(0, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged})
	if err != nil || !result.Acknowledged || result.PersistedPrefixBytes != BytesPerSector {
		t.Fatalf("ack flush: result=%+v err=%v", result, err)
	}
	if ack.durable[0] != state.pending[0] || ack.dirty[0] || !ack.dirty[1] || ack.pending[1] != state.pending[1] {
		t.Fatalf("ack did not commit only selected entry: %+v", ack.Snapshot())
	}
	if ack.durable[5] != [BytesPerSector]byte{0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88} {
		t.Fatal("ack flush changed an already durable neighbor")
	}
	if got, err := ack.ReadSector(0); err != nil || got != state.pending[0] {
		t.Fatalf("acknowledged sector read = %v, %v", got, err)
	}
	prefix := 3
	crash, result, err := state.FlushSector(0, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &prefix})
	if err != nil || result.Acknowledged || result.PersistedPrefixBytes != prefix {
		t.Fatalf("partial flush: result=%+v err=%v", result, err)
	}
	wantPrefix := [BytesPerSector]byte{1, 3, 5}
	if crash.dirty != [SectorCount]bool{} || !bytes.Equal(crash.durable[0][:prefix], wantPrefix[:prefix]) || crash.durable[1] != [BytesPerSector]byte{} {
		t.Fatalf("unacknowledged flush did not drop all volatile writes: %+v", crash.Snapshot())
	}
}

func TestRefusalPrecedenceAndZeroResults(t *testing.T) {
	state, err := (State{}).WriteSector(3, []byte{1, 2, 3, 4, 5, 6, 7, 8})
	if err != nil {
		t.Fatal(err)
	}
	for _, lba := range []int64{-1, math.MinInt64, 32, math.MaxInt64} {
		got, gotErr := state.WriteSector(lba, nil)
		if gotErr != hddwritecrashcuts.ErrInvalidLBA || got != state {
			t.Fatalf("WriteSector(%d,nil) = state change %v, error %v", lba, got != state, gotErr)
		}
		data, readErr := state.ReadSector(lba)
		if readErr != hddwritecrashcuts.ErrInvalidLBA || data != [BytesPerSector]byte{} {
			t.Fatalf("ReadSector(%d) = %v, %v", lba, data, readErr)
		}
		got, result, flushErr := state.FlushSector(lba, hddwritecrashcuts.Cut{Kind: "unknown"})
		if flushErr != hddwritecrashcuts.ErrInvalidLBA || got != state || result != (FlushResult{}) {
			t.Fatalf("FlushSector(%d) refusal not atomic: %v %+v", lba, flushErr, result)
		}
	}
	got, gotErr := state.WriteSector(32, nil)
	if gotErr != hddwritecrashcuts.ErrInvalidLBA || got != state {
		t.Fatalf("LBA must precede length: state changed=%v error=%v", got != state, gotErr)
	}
	got, gotErr = state.WriteSector(3, nil)
	if gotErr != hddwritecrashcuts.ErrInvalidDataLength || got != state {
		t.Fatalf("bad payload refusal: state changed=%v error=%v", got != state, gotErr)
	}
	clean := State{}
	got, result, gotErr := clean.FlushSector(3, hddwritecrashcuts.Cut{Kind: "unknown"})
	if gotErr != ErrSectorClean || got != clean || result != (FlushResult{}) {
		t.Fatalf("clean sector must precede cut validation: state changed=%v result=%+v err=%v", got != clean, result, gotErr)
	}
	badCut := hddwritecrashcuts.Cut{Kind: "unknown"}
	got, result, gotErr = state.FlushSector(3, badCut)
	if gotErr != hddwritecrashcuts.ErrInvalidCut || got != state || result != (FlushResult{}) {
		t.Fatalf("dirty invalid cut must preserve state and zero result: %v %+v", gotErr, result)
	}
	if !errors.Is(gotErr, hddwritecrashcuts.ErrInvalidCut) {
		t.Fatal("lower typed error identity was not preserved")
	}
	prefixZero, prefixEight := 0, BytesPerSector
	invalidCuts := []hddwritecrashcuts.Cut{
		{Kind: hddwritecrashcuts.BeforeMediaWrite, WrittenPrefix: &prefixZero},
		{Kind: hddwritecrashcuts.DuringMediaWrite},
		{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &prefixZero},
		{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &prefixEight},
		{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse, WrittenPrefix: &prefixZero},
		{Kind: hddwritecrashcuts.Acknowledged, WrittenPrefix: &prefixZero},
	}
	for _, cut := range invalidCuts {
		got, result, err := state.FlushSector(3, cut)
		if err != hddwritecrashcuts.ErrInvalidPrefix || got != state || result != (FlushResult{}) {
			t.Fatalf("invalid prefix %#v changed state or returned wrong error: %+v %v", cut, result, err)
		}
	}
}

func TestBoundedHistoriesMatchIndependentArrayModel(t *testing.T) {
	commands := []command{
		{kind: 'w', lba: 0, data: [BytesPerSector]byte{1, 2, 3, 4, 5, 6, 7, 8}},
		{kind: 'w', lba: 1, data: [BytesPerSector]byte{0xF0, 0x0F, 0xAA, 0x55, 0xCC, 0x33, 0x96, 0x69}},
		{kind: 'o', lba: 0, data: [BytesPerSector]byte{8, 7, 6, 5, 4, 3, 2, 1}},
		{kind: 'p', lba: 0},
		{kind: 'f', lba: 0, cut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.BeforeMediaWrite}},
		{kind: 'f', lba: 0, cut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}},
		{kind: 'f', lba: 1, cut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}},
	}
	decisions := 0
	for depth := 0; depth <= 4; depth++ {
		var walk func(int, State, model)
		walk = func(left int, actual State, expected model) {
			if left == 0 {
				return
			}
			for _, cmd := range commands {
				decisions++
				got, gotReceipt, gotErr := runActual(actual, cmd)
				want, wantReceipt, wantErr := runModel(expected, cmd)
				if errorName(gotErr) != wantErr || got != stateOf(want) || gotReceipt != wantReceipt {
					t.Fatalf("history depth=%d command=%+v: actual err=%s result=%+v, want err=%s result=%+v", depth, cmd, errorName(gotErr), gotReceipt, wantErr, wantReceipt)
				}
				walk(left-1, got, want)
			}
		}
		walk(depth, State{}, model{})
	}
	if decisions != 3_262 {
		t.Fatalf("bounded model decisions = %d", decisions)
	}
}

func TestAllOnesPayloadRemainsDirtyUntilAcknowledgement(t *testing.T) {
	data := bytes.Repeat([]byte{0xFF}, BytesPerSector)
	state, err := (State{}).WriteSector(0, data)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := state.ReadSector(0); err != nil || got != [BytesPerSector]byte{0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF} {
		t.Fatalf("pending all-ones read = %v, %v", got, err)
	}
	cuts := []hddwritecrashcuts.Cut{
		{Kind: hddwritecrashcuts.BeforeMediaWrite},
		{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: intPointer(1)},
		{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: intPointer(7)},
		{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse},
		{Kind: hddwritecrashcuts.Acknowledged},
	}
	for _, cut := range cuts {
		next, result, err := state.FlushSector(0, cut)
		if err != nil {
			t.Fatal(err)
		}
		wantAck := cut.Kind == hddwritecrashcuts.Acknowledged
		if result.Acknowledged != wantAck {
			t.Fatalf("cut %#v acknowledged=%v", cut, result.Acknowledged)
		}
		persisted := 0
		switch cut.Kind {
		case hddwritecrashcuts.DuringMediaWrite:
			persisted = *cut.WrittenPrefix
		case hddwritecrashcuts.AfterMediaWriteBeforeResponse, hddwritecrashcuts.Acknowledged:
			persisted = BytesPerSector
		}
		want := [BytesPerSector]byte{}
		for index := 0; index < persisted; index++ {
			want[index] = 0xFF
		}
		if next.Snapshot().Durable[0] != want {
			t.Fatalf("cut %#v durable=%v, want %v", cut, next.Snapshot().Durable[0], want)
		}
	}
}

func intPointer(value int) *int { return &value }

func TestSeededHistoriesMatchIndependentArrayModel(t *testing.T) {
	const seed int64 = 0xDB06A
	random := rand.New(rand.NewSource(seed))
	steps := 0
	for history := 0; history < 512; history++ {
		actual, expected := State{}, model{}
		for step := 0; step < 200; step++ {
			cmd := seededCommand(random)
			got, gotReceipt, gotErr := runActual(actual, cmd)
			want, wantReceipt, wantErr := runModel(expected, cmd)
			if errorName(gotErr) != wantErr || got != stateOf(want) || gotReceipt != wantReceipt {
				t.Fatalf("seed=%x history=%d step=%d command=%+v got error=%s result=%+v want error=%s result=%+v", seed, history, step, cmd, errorName(gotErr), gotReceipt, wantErr, wantReceipt)
			}
			actual, expected = got, want
			if gotErr == nil {
				assertModelRead(t, actual, expected, int64(random.Intn(SectorCount)))
			}
			steps++
		}
	}
	if steps != 102_400 {
		t.Fatalf("seeded steps = %d", steps)
	}
}

func seededCommand(random *rand.Rand) command {
	cmd := command{lba: int64(random.Intn(SectorCount))}
	for i := range cmd.data {
		cmd.data[i] = byte(random.Intn(256))
	}
	switch random.Intn(6) {
	case 0, 1:
		cmd.kind = 'w'
	case 2:
		cmd.kind = 'f'
		cmd.cut = hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}
	case 3:
		cmd.kind = 'f'
		prefix := random.Intn(BytesPerSector-1) + 1
		cmd.cut = hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &prefix}
	case 4:
		cmd.kind = 'p'
	default:
		cmd.kind = 'f'
		cmd.cut = hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}
	}
	return cmd
}

func assertModelRead(t *testing.T, state State, expected model, lba int64) {
	t.Helper()
	got, err := state.ReadSector(lba)
	if err != nil {
		t.Fatalf("ReadSector(%d): %v", lba, err)
	}
	want := expected.durable[int(lba)]
	if expected.dirty[int(lba)] {
		want = expected.pending[int(lba)]
	}
	if got != want {
		t.Fatalf("ReadSector(%d) = %v, want %v", lba, got, want)
	}
}

func FuzzCacheHistories(f *testing.F) {
	f.Add([]byte{0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15})
	f.Add([]byte{3, 1, 0, 2, 7, 4, 1, 0, 5, 2, 8, 6, 3, 9, 1, 4})
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > 128 {
			input = input[:128]
		}
		actual, expected := State{}, model{}
		for cursor := 0; cursor+2 < len(input); cursor += 3 {
			cmd := commandFromBytes(input[cursor : cursor+3])
			got, gotReceipt, gotErr := runActual(actual, cmd)
			want, wantReceipt, wantErr := runModel(expected, cmd)
			if errorName(gotErr) != wantErr || got != stateOf(want) || gotReceipt != wantReceipt {
				t.Fatalf("command %d %+v: actual err=%s result=%+v, want err=%s result=%+v", cursor/3, cmd, errorName(gotErr), gotReceipt, wantErr, wantReceipt)
			}
			actual, expected = got, want
		}
	})
}

func commandFromBytes(raw []byte) command {
	cmd := command{lba: int64(raw[1] % SectorCount)}
	for i := range cmd.data {
		cmd.data[i] = raw[2] ^ byte(i*29) ^ raw[0]
	}
	switch raw[0] % 5 {
	case 0:
		cmd.kind = 'w'
	case 1:
		cmd.kind = 'x'
	case 2:
		cmd.kind = 'f'
		cmd.cut = hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}
	case 3:
		cmd.kind = 'f'
		prefix := int(raw[2]%7) + 1
		cmd.cut = hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &prefix}
	default:
		cmd.kind = 'p'
	}
	return cmd
}

func runActual(state State, cmd command) (State, FlushResult, error) {
	switch cmd.kind {
	case 'w':
		data := cmd.data[:]
		next, err := state.WriteSector(cmd.lba, data)
		return next, FlushResult{}, err
	case 'o':
		next, err := state.WriteSector(cmd.lba, cmd.data[:])
		return next, FlushResult{}, err
	case 'f':
		return state.FlushSector(cmd.lba, cmd.cut)
	case 'p':
		return state.PowerLoss(), FlushResult{}, nil
	default:
		return state, FlushResult{}, nil
	}
}

func runModel(state model, cmd command) (model, FlushResult, string) {
	switch cmd.kind {
	case 'w', 'o':
		if cmd.lba < 0 || cmd.lba >= SectorCount {
			return state, FlushResult{}, "invalid_lba"
		}
		state.pending[int(cmd.lba)] = cmd.data
		state.dirty[int(cmd.lba)] = true
		return state, FlushResult{}, ""
	case 'f':
		if cmd.lba < 0 || cmd.lba >= SectorCount {
			return state, FlushResult{}, "invalid_lba"
		}
		index := int(cmd.lba)
		if !state.dirty[index] {
			return state, FlushResult{}, "sector_clean"
		}
		prefix, ack, kindErr := modelCut(cmd.cut)
		if kindErr != "" {
			return state, FlushResult{}, kindErr
		}
		copy(state.durable[index][:prefix], state.pending[index][:prefix])
		result := FlushResult{Acknowledged: ack, PersistedPrefixBytes: prefix}
		if ack {
			state.pending[index] = [BytesPerSector]byte{}
			state.dirty[index] = false
		} else {
			state.pending = [SectorCount][BytesPerSector]byte{}
			state.dirty = [SectorCount]bool{}
		}
		return state, result, ""
	case 'p':
		state.pending = [SectorCount][BytesPerSector]byte{}
		state.dirty = [SectorCount]bool{}
		return state, FlushResult{}, ""
	default:
		return state, FlushResult{}, ""
	}
}

func modelCut(cut hddwritecrashcuts.Cut) (int, bool, string) {
	switch cut.Kind {
	case hddwritecrashcuts.BeforeMediaWrite:
		if cut.WrittenPrefix != nil {
			return 0, false, "invalid_prefix"
		}
		return 0, false, ""
	case hddwritecrashcuts.DuringMediaWrite:
		if cut.WrittenPrefix == nil || *cut.WrittenPrefix < 1 || *cut.WrittenPrefix >= BytesPerSector {
			return 0, false, "invalid_prefix"
		}
		return *cut.WrittenPrefix, false, ""
	case hddwritecrashcuts.AfterMediaWriteBeforeResponse:
		if cut.WrittenPrefix != nil {
			return 0, false, "invalid_prefix"
		}
		return BytesPerSector, false, ""
	case hddwritecrashcuts.Acknowledged:
		if cut.WrittenPrefix != nil {
			return 0, false, "invalid_prefix"
		}
		return BytesPerSector, true, ""
	default:
		return 0, false, "invalid_cut"
	}
}

func modelOf(state State) model {
	return model{durable: state.durable, pending: state.pending, dirty: state.dirty}
}
func stateOf(value model) State {
	return State{durable: value.durable, pending: value.pending, dirty: value.dirty}
}

func modelFlush(state model, lba int64, data [BytesPerSector]byte, cut hddwritecrashcuts.Cut) (model, FlushResult) {
	state.pending[int(lba)] = data
	state.dirty[int(lba)] = true
	next, result, _ := runModel(state, command{kind: 'f', lba: lba, cut: cut})
	return next, result
}

func errorName(err error) string {
	switch {
	case err == nil:
		return ""
	case errors.Is(err, hddwritecrashcuts.ErrInvalidLBA):
		return "invalid_lba"
	case errors.Is(err, hddwritecrashcuts.ErrInvalidDataLength):
		return "invalid_data_length"
	case errors.Is(err, hddwritecrashcuts.ErrInvalidCut):
		return "invalid_cut"
	case errors.Is(err, hddwritecrashcuts.ErrInvalidPrefix):
		return "invalid_prefix"
	case errors.Is(err, ErrSectorClean):
		return "sector_clean"
	case errors.Is(err, ErrInvalidBarrierCut):
		return "invalid_barrier_cut"
	default:
		return "unexpected_error"
	}
}
