package ftlnandrelocation

import (
	"bytes"
	"encoding/json"
	"errors"
	"math/rand"
	"reflect"
	"testing"

	ftlevacuationplan "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-evacuation-plan"
	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

func TestEvacuationClosedSmallGraphs(t *testing.T) {
	for _, geometry := range []struct{ logical, physical, ppb int64 }{{1, 4, 2}, {2, 4, 2}} {
		initial, err := NewWithEraseBlocks(geometry.logical, geometry.physical, 1, geometry.ppb, 1)
		if err != nil {
			t.Fatal(err)
		}
		queue := []struct {
			device Device
			model  referenceState
		}{{initial, referenceInitialWithEraseBlocks(geometry.logical, geometry.physical, 1, geometry.ppb, 1)}}
		seen := map[string]bool{}
		for len(queue) > 0 {
			node := queue[0]
			queue = queue[1:]
			key := evacuationStateKey(node.model)
			if seen[key] {
				continue
			}
			seen[key] = true
			assertEvacuationState(t, node.device, node.model)
			for _, next := range evacuationSuccessors(t, node.device, node.model) {
				nextKey := evacuationStateKey(next.model)
				if !seen[nextKey] {
					queue = append(queue, next)
				}
			}
		}
		t.Logf("closed evacuation graph %d/%d ppb=%d ceiling=1: %d states", geometry.logical, geometry.physical, geometry.ppb, len(seen))
	}
}

func evacuationSuccessors(t *testing.T, device Device, model referenceState) []struct {
	device Device
	model  referenceState
} {
	t.Helper()
	var out []struct {
		device Device
		model  referenceState
	}
	add := func(next Device, state referenceState) {
		assertEvacuationState(t, next, state)
		out = append(out, struct {
			device Device
			model  referenceState
		}{next, state})
	}
	for logical := int64(0); logical < model.logicalCount; logical++ {
		for _, value := range []byte{0x00, 0xff} {
			next, state := compareEvacuationWrite(t, device, model, logical, []byte{value}, Cut{Kind: nandpageprogramcuts.Complete})
			add(next, state)
		}
	}
	for block := int64(0); block < int64(len(model.blocks)); block++ {
		next, state := compareEvacuationBegin(t, device, model, block)
		add(next, state)
	}
	for _, cut := range []Cut{{Kind: nandpageprogramcuts.BeforeAccept}, {Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0}, {Kind: nandpageprogramcuts.AfterMedia}, {Kind: nandpageprogramcuts.Complete}} {
		next, state := compareEvacuationCopy(t, device, model, cut)
		add(next, state)
	}
	for _, cut := range []EraseCut{{Kind: nandblockerasecuts.BeforeAccept}, {Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0}, {Kind: nandblockerasecuts.Complete}, {Kind: nandblockerasecuts.AfterMedia}} {
		next, _, err := device.FinishEvacuation(cut)
		want, _, code := referenceFinishEvacuation(model, cut)
		assertError(t, err, code)
		assertDeviceEquals(t, next, want)
		assertEvacuationProgress(t, next, want)
		add(next, want)
	}
	next, state := compareEvacuationCancel(t, device, model)
	add(next, state)
	for block := int64(0); block < int64(len(model.blocks)); block++ {
		next, state := compareEraseBlock(t, device, model, block, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
		add(next, state)
	}
	return out
}

func evacuationStateKey(state referenceState) string {
	encoded, _ := json.Marshal(struct {
		Map        []int64
		Status     []modelSlot
		Pages      [][]byte
		Programmed []bool
		Blocks     []BlockSnapshot
		Active     bool
		Target     int64
	}{state.mapping, state.status, state.pages, state.programmed, state.blocks, state.evacuationActive, state.evacuationBlock})
	return string(encoded)
}

func FuzzEvacuationHistory(f *testing.F) {
	f.Add([]byte{0, 0, 1, 0, 2, 1, 3, 0, 4, 0, 5, 2, 6, 0, 7, 1, 8, 1, 9, 3})
	f.Add([]byte{1, 0xff, 2, 0, 3, 1, 5, 3, 4, 0, 7, 2, 8, 1, 9, 0})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 256 {
			return
		}
		device, err := NewWithEraseBlocks(2, 6, 1, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(2, 6, 1, 2, 2)
		for index := 0; index+1 < len(commands); index += 2 {
			op, value := commands[index]%10, commands[index+1]
			switch op {
			case 0, 1:
				cut := Cut{Kind: nandpageprogramcuts.Complete}
				if op == 1 {
					cut = Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: int(value % 2)}
				}
				device, model = compareEvacuationWrite(t, device, model, int64(value%2), []byte{value}, cut)
			case 2:
				device, model = compareEvacuationBegin(t, device, model, int64(value%3))
			case 3:
				cuts := []Cut{{Kind: nandpageprogramcuts.BeforeAccept}, {Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0}, {Kind: nandpageprogramcuts.AfterMedia}, {Kind: nandpageprogramcuts.Complete}}
				device, model = compareEvacuationCopy(t, device, model, cuts[int(value)%len(cuts)])
			case 4, 5:
				cuts := []EraseCut{{Kind: nandblockerasecuts.BeforeAccept}, {Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0}, {Kind: nandblockerasecuts.Complete}, {Kind: nandblockerasecuts.AfterMedia}}
				want, wReceipt, wErr := referenceFinishEvacuation(model, cuts[int(value)%len(cuts)])
				got, gReceipt, err := device.FinishEvacuation(cuts[int(value)%len(cuts)])
				assertError(t, err, wErr)
				if err == nil && gReceipt != wReceipt {
					t.Fatalf("finish receipt %+v want %+v", gReceipt, wReceipt)
				}
				assertEvacuationState(t, got, want)
				device, model = got, want
			case 6:
				device, model = compareEvacuationCancel(t, device, model)
			case 7:
				device, model = compareEraseBlock(t, device, model, int64(value%3), EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
			case 8:
				logical := int64(value % 2)
				got, present, err := device.Read(logical)
				want, wPresent, code := referenceRead(model, logical)
				if errorCode(err) != code || present != wPresent || !bytes.Equal(got, want) {
					t.Fatalf("read %d mismatch", logical)
				}
			case 9:
				_, receipt, err := device.CopyNextEvacuationPage(Cut{Kind: nandpageprogramcuts.CutKind(255)})
				if err == nil || receipt != (EvacuationCopyReceipt{}) {
					t.Fatalf("invalid command cut receipt=%+v err=%v", receipt, err)
				}
			}
		}
	})
}

func TestSeededEvacuationHistories(t *testing.T) {
	rng := rand.New(rand.NewSource(0xDB05c3b2))
	for _, geometry := range []struct{ logical, physical int64 }{{1, 4}, {2, 6}} {
		device, err := NewWithEraseBlocks(geometry.logical, geometry.physical, 1, 2, 2)
		if err != nil {
			t.Fatal(err)
		}
		model := referenceInitialWithEraseBlocks(geometry.logical, geometry.physical, 1, 2, 2)
		for i := 0; i < 5000; i++ {
			value := byte(rng.Intn(256))
			switch rng.Intn(9) {
			case 0, 1:
				device, model = compareEvacuationWrite(t, device, model, int64(rng.Intn(int(geometry.logical))), []byte{value}, Cut{Kind: nandpageprogramcuts.Complete})
			case 2:
				device, model = compareEvacuationBegin(t, device, model, int64(rng.Intn(int(geometry.physical/2))))
			case 3:
				cuts := []Cut{{Kind: nandpageprogramcuts.BeforeAccept}, {Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0}, {Kind: nandpageprogramcuts.Complete}}
				device, model = compareEvacuationCopy(t, device, model, cuts[rng.Intn(len(cuts))])
			case 4:
				cuts := []EraseCut{{Kind: nandblockerasecuts.BeforeAccept}, {Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0}, {Kind: nandblockerasecuts.Complete}, {Kind: nandblockerasecuts.AfterMedia}}
				cut := cuts[rng.Intn(len(cuts))]
				want, wantReceipt, wantErr := referenceFinishEvacuation(model, cut)
				got, receipt, gotErr := device.FinishEvacuation(cut)
				assertError(t, gotErr, wantErr)
				if gotErr == nil && receipt != wantReceipt {
					t.Fatalf("finish receipt=%+v want=%+v", receipt, wantReceipt)
				}
				assertEvacuationState(t, got, want)
				device, model = got, want
			case 5:
				device, model = compareEvacuationCancel(t, device, model)
			case 6:
				block := int64(rng.Intn(int(geometry.physical / 2)))
				device, model = compareEraseBlock(t, device, model, block, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
			case 7:
				logical := int64(rng.Intn(int(geometry.logical)))
				got, present, gotErr := device.Read(logical)
				want, wantPresent, wantErr := referenceRead(model, logical)
				if errorCode(gotErr) != wantErr || present != wantPresent || !bytes.Equal(got, want) {
					t.Fatal("seeded read differs from model")
				}
			case 8:
				_, receipt, gotErr := device.CopyNextEvacuationPage(Cut{Kind: nandpageprogramcuts.CutKind(255)})
				if gotErr == nil || receipt != (EvacuationCopyReceipt{}) {
					t.Fatalf("invalid cut refusal receipt=%+v err=%v", receipt, gotErr)
				}
			}
		}
	}
}

func TestEvacuationProgressAndCopyReplanning(t *testing.T) {
	device, model, _ := copyTestDevice(t)
	if got, active := device.EvacuationProgress(); got != (EvacuationProgress{}) || active {
		t.Fatalf("inactive progress=%+v/%t", got, active)
	}
	device, model = compareEvacuationBegin(t, device, model, 0)
	assertEvacuationState(t, device, model)
	if progress, active := device.EvacuationProgress(); !active || progress != (EvacuationProgress{Block: 0, RemainingLivePages: 2}) {
		t.Fatalf("begun progress=%+v/%t", progress, active)
	}
	before := device.Snapshot()
	if _, receipt, err := device.CopyLivePage(0, ftlevacuationplan.Copy{}, Cut{Kind: nandpageprogramcuts.Complete}); !errors.Is(err, ErrEvacuationActive) || receipt != (Receipt{}) {
		t.Fatalf("direct copy in active target: receipt=%+v err=%v", receipt, err)
	}
	if _, receipt, err := device.EraseBlock(0, EraseCut{Kind: nandblockerasecuts.Complete}); !errors.Is(err, ErrEvacuationActive) || receipt != (EraseReceipt{}) {
		t.Fatalf("direct erase in active target: receipt=%+v err=%v", receipt, err)
	}
	if _, receipt, err := device.FinishEvacuation(EraseCut{Kind: nandblockerasecuts.CutKind(255)}); !errors.Is(err, ErrEvacuationStillLive) || receipt != (EraseReceipt{}) {
		t.Fatalf("live target must precede erase-cut validation: %+v %v", receipt, err)
	}
	if !reflect.DeepEqual(device.Snapshot(), before) {
		t.Fatal("active-target refusals changed media")
	}

	device, model = compareEvacuationWrite(t, device, model, 0, []byte{0x71, 0x72}, Cut{Kind: nandpageprogramcuts.Complete})
	progress, _ := device.EvacuationProgress()
	if progress.RemainingLivePages != 1 {
		t.Fatalf("host write did not move a target owner: %+v", progress)
	}
	device, model = compareEvacuationCopy(t, device, model, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	first, _ := device.EvacuationProgress()
	if first.RemainingLivePages != 1 {
		t.Fatalf("zero-prefix copy changed ownership: %+v", first)
	}
	device, model = compareEvacuationCopy(t, device, model, Cut{Kind: nandpageprogramcuts.AfterMedia})
	progress, _ = device.EvacuationProgress()
	if progress.RemainingLivePages != 0 {
		t.Fatalf("complete copy did not drain target: %+v", progress)
	}
	if _, receipt, err := device.CopyNextEvacuationPage(Cut{Kind: nandpageprogramcuts.Complete}); !errors.Is(err, ErrEvacuationNoLivePages) || receipt != (EvacuationCopyReceipt{}) {
		t.Fatalf("empty target copy result: %+v %v", receipt, err)
	}
	device, receipt, err := device.FinishEvacuation(EraseCut{Kind: nandblockerasecuts.Complete})
	if err != nil || !receipt.Released || !receipt.Acknowledged {
		t.Fatalf("finished evacuation receipt=%+v err=%v", receipt, err)
	}
	model, _, code := referenceFinishEvacuation(model, nandblockerasecuts.Cut{Kind: nandblockerasecuts.Complete})
	if code != "" {
		t.Fatalf("model finish=%s", code)
	}
	assertEvacuationState(t, device, model)
	if _, active := device.EvacuationProgress(); active {
		t.Fatal("released target remained active")
	}
	if _, _, err := device.FinishEvacuation(EraseCut{Kind: nandblockerasecuts.Complete}); !errors.Is(err, ErrNoActiveEvacuation) {
		t.Fatalf("second finish error=%v", err)
	}
}

func TestHostWriteCannotAllocateInsideActiveEvacuation(t *testing.T) {
	device, err := NewWithEraseBlocks(1, 6, 1, 3, 2)
	if err != nil {
		t.Fatal(err)
	}
	device, _, _ = device.Write(0, []byte{0x10}, Cut{Kind: nandpageprogramcuts.Complete})
	device, _, _ = device.Write(0, []byte{0x20}, Cut{Kind: nandpageprogramcuts.Complete})
	active, err := device.BeginEvacuation(0)
	if err != nil {
		t.Fatal(err)
	}
	next, receipt, err := active.Write(0, []byte{0x30}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil || receipt.Physical != 3 {
		t.Fatalf("active extent was not fenced, receipt=%+v err=%v", receipt, err)
	}
	if next.Snapshot().Map.PhysicalStatus[2] != ftlpagemap.Free {
		t.Fatal("host write allocated inside active evacuation target")
	}
}

func TestCopyNextReplansInsteadOfReusingSpentDestination(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 8, 1, 4, 2)
	if err != nil {
		t.Fatal(err)
	}
	for _, logical := range []int64{0, 1, 0, 1} {
		device, _, err = device.Write(logical, []byte{byte(0x10 + logical)}, Cut{Kind: nandpageprogramcuts.Complete})
		if err != nil {
			t.Fatal(err)
		}
	}
	active, err := device.BeginEvacuation(0)
	if err != nil {
		t.Fatal(err)
	}
	spent, first, err := active.CopyNextEvacuationPage(Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	if err != nil || first.Move.DestinationPhysical != 4 {
		t.Fatalf("first selected move=%+v err=%v", first, err)
	}
	retried, second, err := spent.CopyNextEvacuationPage(Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	if err != nil || second.Move.DestinationPhysical == first.Move.DestinationPhysical {
		t.Fatalf("copy reused prior spent destination: first=%+v second=%+v err=%v", first, second, err)
	}
	if retried.Snapshot().Map.PhysicalStatus[int(first.Move.DestinationPhysical)] != ftlpagemap.Stale {
		t.Fatal("prefix destination was not spent")
	}
}

func TestDirectCopyIntoActiveTargetIsUnavailable(t *testing.T) {
	device, err := NewWithEraseBlocks(2, 12, 1, 4, 2)
	if err != nil {
		t.Fatal(err)
	}
	for _, command := range []struct {
		logical int64
		value   byte
	}{{0, 0x10}, {0, 0x11}, {1, 0x21}} {
		device, _, err = device.Write(command.logical, []byte{command.value}, Cut{Kind: nandpageprogramcuts.Complete})
		if err != nil {
			t.Fatal(err)
		}
	}
	active, err := device.BeginEvacuation(0)
	if err != nil {
		t.Fatal(err)
	}
	active, _, err = active.Write(1, []byte{0x22}, Cut{Kind: nandpageprogramcuts.Complete})
	if err != nil {
		t.Fatal(err)
	}
	snapshot := active.Snapshot()
	source := snapshot.Map.LogicalToPhysical[1]
	move := ftlevacuationplan.Copy{Logical: 1, SourcePhysical: source, DestinationPhysical: 3}
	before := active.Snapshot()
	got, receipt, err := active.CopyLivePage(1, move, Cut{Kind: nandpageprogramcuts.Complete})
	if !errors.Is(err, ErrCopyDestinationUnavailable) || receipt != (Receipt{}) || !reflect.DeepEqual(got.Snapshot(), before) {
		t.Fatalf("direct copy into active extent=%+v/%v", receipt, err)
	}
}

func TestEvacuationCancelDropsFenceButKeepsCompletedHostMove(t *testing.T) {
	device, err := NewWithEraseBlocks(1, 6, 1, 3, 2)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(1, 6, 1, 3, 2)
	device, model = compareEvacuationWrite(t, device, model, 0, []byte{0x11}, Cut{Kind: nandpageprogramcuts.Complete})
	device, model = compareEvacuationWrite(t, device, model, 0, []byte{0x22}, Cut{Kind: nandpageprogramcuts.Complete})
	device, model = compareEvacuationBegin(t, device, model, 0)
	device, model = compareEvacuationWrite(t, device, model, 0, []byte{0x33}, Cut{Kind: nandpageprogramcuts.Complete})
	progress, _ := device.EvacuationProgress()
	if progress.RemainingLivePages != 0 {
		t.Fatalf("host write should move the only target owner: %+v", progress)
	}
	device, model = compareEvacuationCancel(t, device, model)
	device, model = compareEvacuationWrite(t, device, model, 0, []byte{0x44}, Cut{Kind: nandpageprogramcuts.Complete})
	if mapping := device.Snapshot().Map.LogicalToPhysical[0]; mapping != 2 {
		t.Fatalf("cancel did not return the target slot to allocation: mapped=%d", mapping)
	}
}

func TestEvacuationPartialEraseLocksUntilMediaRelease(t *testing.T) {
	device, model, _ := copyTestDevice(t)
	device, model = compareEvacuationBegin(t, device, model, 0)
	for progress, active := device.EvacuationProgress(); active && progress.RemainingLivePages > 0; progress, active = device.EvacuationProgress() {
		device, model = compareEvacuationCopy(t, device, model, Cut{Kind: nandpageprogramcuts.Complete})
	}
	device, receipt, err := device.FinishEvacuation(EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1})
	if err != nil || receipt.Released || receipt.Acknowledged {
		t.Fatalf("partial finish receipt=%+v err=%v", receipt, err)
	}
	model, _, code := referenceFinishEvacuation(model, nandblockerasecuts.Cut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 1})
	if code != "" {
		t.Fatalf("model partial finish=%s", code)
	}
	assertEvacuationState(t, device, model)
	progress, active := device.EvacuationProgress()
	if !active || !progress.IncompleteErase {
		t.Fatalf("partial erase lost active lock: %+v/%t", progress, active)
	}
	if _, err := device.CancelEvacuation(); !errors.Is(err, ErrEvacuationBlockIncomplete) {
		t.Fatalf("partial erase cancellation error=%v", err)
	}
	before := device.Snapshot()
	device, mediaReceipt, err := device.FinishEvacuation(EraseCut{Kind: nandblockerasecuts.AfterMedia})
	if err != nil || !mediaReceipt.Released || mediaReceipt.Acknowledged {
		t.Fatalf("lost-ack media finish=%+v %v", mediaReceipt, err)
	}
	model, modelReceipt, modelError := referenceFinishEvacuation(model, nandblockerasecuts.Cut{Kind: nandblockerasecuts.AfterMedia})
	if modelError != "" || !modelReceipt.Released || modelReceipt.Acknowledged || mediaReceipt != modelReceipt {
		t.Fatalf("model media finish=%+v %s", modelReceipt, modelError)
	}
	assertDeviceEquals(t, device, model)
	if reflect.DeepEqual(device.Snapshot(), before) || device.ReclamationSnapshot().Blocks[0].CompletedErases != 1 {
		t.Fatal("completed erase did not change media and advance its count")
	}
	if _, active := device.EvacuationProgress(); active {
		t.Fatal("unacknowledged completed erase did not clear the active target")
	}
}

func TestEvacuationBeginOrderAndCapacityPreflight(t *testing.T) {
	var zero Device
	if _, err := zero.BeginEvacuation(-1); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) {
		t.Fatalf("zero device begin precedence: %v", err)
	}
	legacy, _ := New(1, 4, 1)
	if _, err := legacy.BeginEvacuation(-1); !errors.Is(err, ErrEraseDisabled) {
		t.Fatalf("legacy begin precedence: %v", err)
	}
	limited, _ := NewWithEraseBlocks(1, 4, 1, 2, 0)
	if _, err := limited.BeginEvacuation(-1); !errors.Is(err, ErrInvalidBlockIndex) {
		t.Fatalf("block index must precede ceiling: %v", err)
	}
	if _, err := limited.BeginEvacuation(0); !errors.Is(err, nandblockerasecuts.ErrEraseLimit) {
		t.Fatalf("zero ceiling begin error=%v", err)
	}
	insufficient, model := prepareEvacuationCapacityCase(t)
	before := insufficient.Snapshot()
	if _, err := insufficient.BeginEvacuation(2); !errors.Is(err, ftlevacuationplan.ErrInsufficientCapacity) {
		t.Fatalf("fenced spare preflight error=%v", err)
	}
	if !reflect.DeepEqual(insufficient.Snapshot(), before) || model.evacuationActive {
		t.Fatal("failed begin changed the device or activated its target")
	}
}

func TestEvacuationReplansAfterUnrelatedBlockRelease(t *testing.T) {
	device, model, _ := copyTestDevice(t)
	device, model = compareEvacuationWrite(t, device, model, 1, []byte{0x61, 0x62}, Cut{Kind: nandpageprogramcuts.Complete})
	device, model = compareEvacuationWrite(t, device, model, 1, []byte{0x71, 0x72}, Cut{Kind: nandpageprogramcuts.Complete})
	plan, err := ftlevacuationplan.PlanEvacuation(device.table, 3, 1, []bool{false, false, false})
	if err != nil || len(plan.Copies) != 1 {
		t.Fatalf("preparation plan=%+v err=%v", plan, err)
	}
	device, model = compareCopyLive(t, device, model, 1, plan.Copies[0], Cut{Kind: nandpageprogramcuts.Complete})
	device, model = compareEvacuationBegin(t, device, model, 0)
	device, model = compareEraseBlock(t, device, model, 1, EraseCut{Kind: nandblockerasecuts.Complete})
	device, model = compareEvacuationCopy(t, device, model, Cut{Kind: nandpageprogramcuts.AfterPrefix, PrefixBytes: 0})
	progress, _ := device.EvacuationProgress()
	if progress.RemainingLivePages != 2 {
		t.Fatalf("zero-prefix copy changed live progress: %+v", progress)
	}
	// The completed unrelated erase releases physical page 3, now the lowest
	// valid destination. Replanning must not use the old pre-erase plan.
	if device.Snapshot().Pages[3].Programmed != true {
		t.Fatal("the replan did not select the newly released lowest destination")
	}
}

func TestEvacuationZeroLegacyAndInvalidCutRefusals(t *testing.T) {
	var zero Device
	if got, receipt, err := zero.CopyNextEvacuationPage(Cut{}); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || receipt != (EvacuationCopyReceipt{}) || !reflect.DeepEqual(got.Snapshot(), zero.Snapshot()) {
		t.Fatalf("zero copy refusal=%+v/%v", receipt, err)
	}
	if got, receipt, err := zero.FinishEvacuation(EraseCut{}); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || receipt != (EraseReceipt{}) || !reflect.DeepEqual(got.Snapshot(), zero.Snapshot()) {
		t.Fatalf("zero finish refusal=%+v/%v", receipt, err)
	}
	if got, err := zero.CancelEvacuation(); !errors.Is(err, ftlpagemap.ErrInvalidGeometry) || !reflect.DeepEqual(got.Snapshot(), zero.Snapshot()) {
		t.Fatalf("zero cancel refusal=%v", err)
	}
	legacy, _ := New(1, 4, 1)
	if _, receipt, err := legacy.CopyNextEvacuationPage(Cut{}); !errors.Is(err, ErrEraseDisabled) || receipt != (EvacuationCopyReceipt{}) {
		t.Fatalf("legacy copy refusal=%+v/%v", receipt, err)
	}
	if _, receipt, err := legacy.FinishEvacuation(EraseCut{}); !errors.Is(err, ErrEraseDisabled) || receipt != (EraseReceipt{}) {
		t.Fatalf("legacy finish refusal=%+v/%v", receipt, err)
	}
	if _, err := legacy.CancelEvacuation(); !errors.Is(err, ErrEraseDisabled) {
		t.Fatalf("legacy cancel refusal=%v", err)
	}
	device, model, _ := copyTestDevice(t)
	device, model = compareEvacuationBegin(t, device, model, 0)
	before := device.Snapshot()
	device, receipt, err := device.CopyNextEvacuationPage(Cut{Kind: nandpageprogramcuts.CutKind(255)})
	if !errors.Is(err, nandpageprogramcuts.ErrInvalidCut) || receipt != (EvacuationCopyReceipt{}) || !reflect.DeepEqual(device.Snapshot(), before) {
		t.Fatalf("invalid accepted program cut=%+v/%v", receipt, err)
	}
	assertEvacuationState(t, device, model)
	for progress, active := device.EvacuationProgress(); active && progress.RemainingLivePages > 0; progress, active = device.EvacuationProgress() {
		device, model = compareEvacuationCopy(t, device, model, Cut{Kind: nandpageprogramcuts.Complete})
	}
	before = device.Snapshot()
	device, finishReceipt, finishErr := device.FinishEvacuation(EraseCut{Kind: nandblockerasecuts.CutKind(255)})
	if !errors.Is(finishErr, nandblockerasecuts.ErrInvalidCut) || finishReceipt != (EraseReceipt{}) || !reflect.DeepEqual(device.Snapshot(), before) {
		t.Fatalf("invalid erase cut=%+v/%v", finishReceipt, finishErr)
	}
	assertEvacuationState(t, device, model)
}

func compareEvacuationBegin(t *testing.T, device Device, model referenceState, block int64) (Device, referenceState) {
	t.Helper()
	want, wantErr := referenceBeginEvacuation(model, block)
	got, err := device.BeginEvacuation(block)
	assertError(t, err, wantErr)
	assertDeviceEquals(t, got, want)
	assertEvacuationProgress(t, got, want)
	return got, want
}

func compareEvacuationWrite(t *testing.T, device Device, model referenceState, logical int64, payload []byte, cut Cut) (Device, referenceState) {
	t.Helper()
	want, _, wantErr := referenceWrite(model, logical, payload, runtimeModelCut(cut))
	got, _, err := device.Write(logical, payload, cut)
	assertError(t, err, wantErr)
	assertDeviceEquals(t, got, want)
	assertEvacuationProgress(t, got, want)
	return got, want
}

func compareEvacuationCopy(t *testing.T, device Device, model referenceState, cut Cut) (Device, referenceState) {
	t.Helper()
	want, wantReceipt, wantErr := referenceCopyNext(model, runtimeModelCut(cut))
	got, receipt, err := device.CopyNextEvacuationPage(cut)
	assertError(t, err, wantErr)
	if err == nil {
		if receipt != wantReceipt {
			t.Fatalf("copy receipt=%+v want=%+v", receipt, wantReceipt)
		}
	} else if receipt != (EvacuationCopyReceipt{}) {
		t.Fatalf("refused copy returned receipt %+v", receipt)
	}
	assertDeviceEquals(t, got, want)
	assertEvacuationProgress(t, got, want)
	return got, want
}

func compareEvacuationCancel(t *testing.T, device Device, model referenceState) (Device, referenceState) {
	t.Helper()
	want, wantErr := referenceCancelEvacuation(model)
	got, err := device.CancelEvacuation()
	assertError(t, err, wantErr)
	assertDeviceEquals(t, got, want)
	assertEvacuationProgress(t, got, want)
	return got, want
}

func assertEvacuationState(t *testing.T, device Device, model referenceState) {
	t.Helper()
	assertDeviceEquals(t, device, model)
	assertEvacuationProgress(t, device, model)
}

func assertEvacuationProgress(t *testing.T, device Device, model referenceState) {
	t.Helper()
	want, wantActive := referenceEvacuationProgress(model)
	got, gotActive := device.EvacuationProgress()
	if got != want || gotActive != wantActive {
		t.Fatalf("evacuation progress=%+v/%t want %+v/%t", got, gotActive, want, wantActive)
	}
}

func referenceBeginEvacuation(before referenceState, block int64) (referenceState, string) {
	return referenceBeginEvacuationExcluding(before, block, nil)
}

func referenceCopyNext(before referenceState, cut modelCut) (referenceState, EvacuationCopyReceipt, string) {
	return referenceCopyNextExcluding(before, cut, nil)
}

func referenceFinishEvacuation(before referenceState, cut nandblockerasecuts.Cut) (referenceState, EraseReceipt, string) {
	if !referenceConfigured(before) {
		return before, EraseReceipt{}, "invalid_geometry"
	}
	if before.pagesPerBlock == 0 {
		return before, EraseReceipt{}, "erase_disabled"
	}
	if !before.evacuationActive {
		return before, EraseReceipt{}, "no_active_evacuation"
	}
	progress, _ := referenceEvacuationProgress(before)
	if progress.RemainingLivePages != 0 {
		return before, EraseReceipt{}, "evacuation_still_live"
	}
	next, receipt, code := referenceEraseBlockInternal(before, before.evacuationBlock, cut, true)
	if code != "" {
		return before, EraseReceipt{}, code
	}
	if receipt.Released {
		next.evacuationActive = false
		next.evacuationBlock = 0
	}
	return next, receipt, ""
}

func referenceCancelEvacuation(before referenceState) (referenceState, string) {
	if !referenceConfigured(before) {
		return before, "invalid_geometry"
	}
	if before.pagesPerBlock == 0 {
		return before, "erase_disabled"
	}
	if !before.evacuationActive {
		return before, "no_active_evacuation"
	}
	if before.blocks[int(before.evacuationBlock)].IncompleteErase {
		return before, "evacuation_block_incomplete"
	}
	next := cloneReference(before)
	next.evacuationActive = false
	next.evacuationBlock = 0
	return next, ""
}

func referenceEvacuationProgress(state referenceState) (EvacuationProgress, bool) {
	if !state.evacuationActive {
		return EvacuationProgress{}, false
	}
	first := state.evacuationBlock * state.pagesPerBlock
	remaining := int64(0)
	for physical := first; physical < first+state.pagesPerBlock; physical++ {
		if state.status[int(physical)] == modelLive {
			remaining++
		}
	}
	return EvacuationProgress{Block: state.evacuationBlock, RemainingLivePages: remaining, IncompleteErase: state.blocks[int(state.evacuationBlock)].IncompleteErase}, true
}

func referenceIncompleteMask(state referenceState) []bool {
	mask := make([]bool, len(state.blocks))
	for index, block := range state.blocks {
		mask[index] = block.IncompleteErase
	}
	return mask
}

func referenceConfigured(state referenceState) bool {
	return state.logicalCount > 0 && state.logicalCount <= 64 && state.physicalCount >= 2 && state.physicalCount <= 128 && state.physicalCount > state.logicalCount && state.pageSize > 0 && state.pageSize <= maxPageSize
}

func prepareEvacuationCapacityCase(t *testing.T) (Device, referenceState) {
	t.Helper()
	device, err := NewWithEraseBlocks(1, 6, 1, 2, 2)
	if err != nil {
		t.Fatal(err)
	}
	model := referenceInitialWithEraseBlocks(1, 6, 1, 2, 2)
	for _, value := range []byte{1, 2, 3, 4, 5} {
		device, model = compareEvacuationWrite(t, device, model, 0, []byte{value}, Cut{Kind: nandpageprogramcuts.Complete})
	}
	device, model = compareEraseBlock(t, device, model, 1, EraseCut{Kind: nandblockerasecuts.AfterPrefix, PrefixPages: 0})
	device, model = compareEvacuationWrite(t, device, model, 0, []byte{6}, Cut{Kind: nandpageprogramcuts.Complete})
	return device, model
}
