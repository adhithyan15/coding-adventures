package hddblockdevice

import (
	"bytes"
	"errors"
	"math"
	"math/rand"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
	hddwritebackcache "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache"
	memoryblockdevice "github.com/adhithyan15/coding-adventures/code/packages/go/memory-block-device"
)

const (
	modelBlockCount    = 32
	modelBlockBytes    = 8
	modelMaximumPrefix = modelBlockBytes - 1
)

var deviceGeometry = blockdevicecontract.Geometry{BlockCount: modelBlockCount, BytesPerBlock: modelBlockBytes}

func TestImplementsSharedBlockDeviceInterface(t *testing.T) {
	var device blockdevicecontract.Device = New()
	if got := device.Geometry(); got != deviceGeometry {
		t.Fatalf("Geometry()=%+v want %+v", got, deviceGeometry)
	}
}

func TestNilAndZeroReceiverContract(t *testing.T) {
	var zero HDD
	if got := zero.Geometry(); got != deviceGeometry {
		t.Fatalf("zero HDD Geometry()=%+v want %+v", got, deviceGeometry)
	}
	assertBlock(t, &zero, 0, make([]byte, modelBlockBytes))
	if err := zero.WriteBlock(0, blockBytes(7)); err != nil {
		t.Fatal(err)
	}
	if err := zero.Flush(); err != nil {
		t.Fatal(err)
	}
	if restarted, err := zero.Restart(); err != nil || restarted == nil {
		t.Fatalf("zero HDD Restart()=(%v,%v)", restarted, err)
	}

	var nilDevice *HDD
	if got := nilDevice.Geometry(); got != (blockdevicecontract.Geometry{}) {
		t.Fatalf("nil HDD Geometry()=%+v want zero", got)
	}
	if _, err := nilDevice.ReadBlock(0); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
		t.Fatalf("nil ReadBlock error=%v", err)
	}
	if err := nilDevice.WriteBlock(0, nil); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
		t.Fatalf("nil WriteBlock error=%v", err)
	}
	if err := nilDevice.Flush(); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
		t.Fatalf("nil Flush error=%v", err)
	}
	receipt, err := nilDevice.FlushWithCut(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete})
	if receipt != (hddwritebackcache.BarrierReceipt{}) || !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
		t.Fatalf("nil FlushWithCut()=(%+v,%v)", receipt, err)
	}
	if restarted, err := nilDevice.Restart(); restarted != nil || !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
		t.Fatalf("nil Restart()=(%v,%v)", restarted, err)
	}
}

func TestAddressThenPayloadValidationAndOwnership(t *testing.T) {
	device := New()
	for _, test := range []struct {
		name  string
		block int64
		data  []byte
		want  error
	}{
		{"negative before wrong width", -1, nil, blockdevicecontract.ErrInvalidBlock},
		{"upper bound before wrong width", modelBlockCount, []byte{1}, blockdevicecontract.ErrInvalidBlock},
		{"minimum signed address", math.MinInt64, nil, blockdevicecontract.ErrInvalidBlock},
		{"maximum signed address", math.MaxInt64, nil, blockdevicecontract.ErrInvalidBlock},
		{"wrong width after address", 0, []byte{1}, blockdevicecontract.ErrInvalidBlockData},
	} {
		t.Run(test.name, func(t *testing.T) {
			if err := device.WriteBlock(test.block, test.data); !errors.Is(err, test.want) {
				t.Fatalf("WriteBlock error=%v want %v", err, test.want)
			}
			if test.want == blockdevicecontract.ErrInvalidBlockData {
				if _, err := device.ReadBlock(test.block); err != nil {
					t.Fatalf("valid block read failed: %v", err)
				}
			}
		})
	}
	input := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	if err := device.WriteBlock(4, input); err != nil {
		t.Fatal(err)
	}
	input[0] = 99
	got, err := device.ReadBlock(4)
	if err != nil {
		t.Fatal(err)
	}
	want := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	if !bytes.Equal(got, want) {
		t.Fatalf("write did not copy caller bytes: %v", got)
	}
	got[1] = 99
	again, err := device.ReadBlock(4)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(again, want) {
		t.Fatalf("read result was not detached: %v", again)
	}
}

func TestFlushPersistsEveryDirtyBlock(t *testing.T) {
	device := New()
	for index, block := range []int64{2, 5, 8} {
		if err := device.WriteBlock(block, blockBytes(byte(31+index*17))); err != nil {
			t.Fatal(err)
		}
	}
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	restarted, err := device.Restart()
	if err != nil {
		t.Fatal(err)
	}
	for index, block := range []int64{2, 5, 8} {
		assertBlock(t, restarted, block, blockBytes(byte(31+index*17)))
	}
}

func TestSharedMemoryAndHDDClientTrace(t *testing.T) {
	memory, err := memoryblockdevice.New(deviceGeometry)
	if err != nil {
		t.Fatal(err)
	}
	var clients = []blockdevicecontract.Device{memory, New()}
	ops := []clientOperation{
		{kind: clientRead, block: 0},
		{kind: clientWrite, block: 0, data: []byte{1, 2, 3, 4, 5, 6, 7, 8}},
		{kind: clientRead, block: 0},
		{kind: clientWrite, block: 0, data: []byte{8, 7, 6, 5, 4, 3, 2, 1}},
		{kind: clientWrite, block: 31, data: []byte{0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88}},
		{kind: clientFlush},
		{kind: clientRestart},
		{kind: clientWrite, block: 1, data: []byte{5, 4, 3, 2, 1, 0, 9, 8}},
		{kind: clientRestart},
		{kind: clientFlush},
		{kind: clientRestart},
	}
	for step, op := range ops {
		var firstRead, secondRead []byte
		var firstError, secondError error
		for index, client := range clients {
			switch op.kind {
			case clientRead:
				data, readErr := client.ReadBlock(op.block)
				if index == 0 {
					firstRead, firstError = data, readErr
				} else {
					secondRead, secondError = data, readErr
				}
			case clientWrite:
				input := append([]byte(nil), op.data...)
				writeErr := client.WriteBlock(op.block, input)
				for byteIndex := range input {
					input[byteIndex] ^= 0xff
				}
				if index == 0 {
					firstError = writeErr
				} else {
					secondError = writeErr
				}
			case clientFlush:
				flushErr := client.Flush()
				if index == 0 {
					firstError = flushErr
				} else {
					secondError = flushErr
				}
			case clientRestart:
				restarted, restartErr := restartClient(t, client)
				clients[index] = restarted
				if index == 0 {
					firstError = restartErr
				} else {
					secondError = restartErr
				}
			}
		}
		if !sameError(firstError, secondError) {
			t.Fatalf("step %d %v errors differ: %v / %v", step, op, firstError, secondError)
		}
		if op.kind == clientRead {
			if !bytes.Equal(firstRead, secondRead) {
				t.Fatalf("step %d reads differ: %v / %v", step, firstRead, secondRead)
			}
			if len(firstRead) > 0 {
				firstRead[0], secondRead[0] = 0x55, 0xaa
			}
		}
		assertClientsEqual(t, clients[0], clients[1])
	}
}

func TestRestartSeparatesDurableAndPendingBranches(t *testing.T) {
	device := New()
	old := blockBytes(3)
	pending := blockBytes(81)
	if err := device.WriteBlock(3, old); err != nil {
		t.Fatal(err)
	}
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	if err := device.WriteBlock(3, pending); err != nil {
		t.Fatal(err)
	}
	restarted, err := device.Restart()
	if err != nil {
		t.Fatal(err)
	}
	assertBlock(t, restarted, 3, old)
	assertBlock(t, device, 3, pending)

	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	assertBlock(t, restarted, 3, old)
	newBranch, err := restarted.Restart()
	if err != nil {
		t.Fatal(err)
	}
	if err := newBranch.WriteBlock(3, blockBytes(44)); err != nil {
		t.Fatal(err)
	}
	if err := newBranch.Flush(); err != nil {
		t.Fatal(err)
	}
	assertBlock(t, device, 3, pending)
	assertBlock(t, restarted, 3, old)
	assertBlock(t, newBranch, 3, blockBytes(44))
}

func TestAllBarrierCutsMatchIndependentByteModel(t *testing.T) {
	cuts := []hddwritebackcache.BarrierCut{
		{Kind: hddwritebackcache.BeforeFirstSector},
		{Kind: hddwritebackcache.AfterAllMedia},
		{Kind: hddwritebackcache.BarrierComplete},
	}
	for ordinal := int64(0); ordinal < 3; ordinal++ {
		cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: ordinal, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.BeforeMediaWrite}})
		for prefix := 1; prefix <= modelMaximumPrefix; prefix++ {
			value := prefix
			cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: ordinal, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &value}})
		}
		cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: ordinal, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}})
	}
	for _, cut := range cuts {
		device, model := dirtyThreeBlocks(t)
		wantModel, wantReceipt, wantErr := model.flushWithCut(cut)
		gotReceipt, gotErr := device.FlushWithCut(cut)
		if !sameError(gotErr, wantErr) {
			t.Fatalf("cut=%+v error=%v want=%v", cut, gotErr, wantErr)
		}
		if gotReceipt != wantReceipt {
			t.Fatalf("cut=%+v receipt=%+v want=%+v", cut, gotReceipt, wantReceipt)
		}
		if wantErr == nil {
			model = wantModel
		}
		assertAgainstModel(t, device, model)
		if wantErr == nil {
			restarted, err := device.Restart()
			if err != nil {
				t.Fatal(err)
			}
			for block := int64(0); block < modelBlockCount; block++ {
				assertBlock(t, restarted, block, model.durable[block][:])
			}
		}
	}
}

func TestMalformedBarrierCutsReturnZeroReceiptAndPreservePending(t *testing.T) {
	badPrefix := 0
	cuts := []struct {
		name string
		cut  hddwritebackcache.BarrierCut
		want error
	}{
		{"unknown barrier", hddwritebackcache.BarrierCut{Kind: 99}, hddwritebackcache.ErrInvalidBarrierCut},
		{"unused ordinal", hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BeforeFirstSector, SectorIndex: 1}, hddwritebackcache.ErrInvalidBarrierCut},
		{"negative ordinal", hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: -1}, hddwritebackcache.ErrInvalidBarrierCut},
		{"ordinal outside dirty set", hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: 3}, hddwritebackcache.ErrInvalidBarrierCut},
		{"lower ack is not a barrier cut", hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: 0, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.Acknowledged}}, hddwritebackcache.ErrInvalidBarrierCut},
		{"bad lower prefix after prior work", hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: 1, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite}}, hddwritecrashcuts.ErrInvalidPrefix},
		{"zero lower prefix", hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: 0, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &badPrefix}}, hddwritecrashcuts.ErrInvalidPrefix},
		{"unknown lower cut", hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: 0, MediaCut: hddwritecrashcuts.Cut{Kind: "unknown"}}, hddwritecrashcuts.ErrInvalidCut},
		{"unexpected lower prefix", hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: 0, MediaCut: hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse, WrittenPrefix: &badPrefix}}, hddwritecrashcuts.ErrInvalidPrefix},
		{"complete has ordinal", hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete, SectorIndex: 1}, hddwritebackcache.ErrInvalidBarrierCut},
	}
	for _, test := range cuts {
		t.Run(test.name, func(t *testing.T) {
			device, before := dirtyThreeBlocks(t)
			receipt, err := device.FlushWithCut(test.cut)
			if !errors.Is(err, test.want) {
				t.Fatalf("error=%v want %v", err, test.want)
			}
			if receipt != (hddwritebackcache.BarrierReceipt{}) {
				t.Fatalf("refusal receipt=%+v, want zero", receipt)
			}
			assertAgainstModel(t, device, before)
		})
	}
}

func TestEmptyBarrierCutFamilies(t *testing.T) {
	for _, cut := range []hddwritebackcache.BarrierCut{
		{Kind: hddwritebackcache.BeforeFirstSector},
		{Kind: hddwritebackcache.AfterAllMedia},
		{Kind: hddwritebackcache.BarrierComplete},
	} {
		device := New()
		receipt, err := device.FlushWithCut(cut)
		want := hddwritebackcache.BarrierReceipt{Acknowledged: cut.Kind == hddwritebackcache.BarrierComplete}
		if err != nil || receipt != want {
			t.Fatalf("empty cut=%+v receipt=%+v err=%v want=%+v", cut, receipt, err, want)
		}
	}
	device := New()
	if receipt, err := device.FlushWithCut(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector}); receipt != (hddwritebackcache.BarrierReceipt{}) || !errors.Is(err, hddwritebackcache.ErrInvalidBarrierCut) {
		t.Fatalf("empty AtSector=(%+v,%v)", receipt, err)
	}
}

func TestSeededSharedHistories(t *testing.T) {
	const seed = int64(0xDB06B2B2)
	const commands = 65_536
	rng := rand.New(rand.NewSource(seed))
	device := New()
	model := zeroModel()
	for command := 0; command < commands; command++ {
		switch rng.Intn(4) {
		case 0:
			block := int64(rng.Intn(modelBlockCount))
			data := make([]byte, modelBlockBytes)
			_, _ = rng.Read(data)
			model.write(block, data)
			if err := device.WriteBlock(block, data); err != nil {
				t.Fatal(err)
			}
		case 1:
			model, _, _ = model.flushWithCut(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete})
			if err := device.Flush(); err != nil {
				t.Fatal(err)
			}
		case 2:
			cut := randomBarrierCut(rng)
			wantModel, wantReceipt, wantErr := model.flushWithCut(cut)
			gotReceipt, gotErr := device.FlushWithCut(cut)
			if !sameError(gotErr, wantErr) || gotReceipt != wantReceipt {
				t.Fatalf("command %d cut=%+v got=(%+v,%v), want=(%+v,%v)", command, cut, gotReceipt, gotErr, wantReceipt, wantErr)
			}
			if wantErr == nil {
				model = wantModel
			}
		case 3:
			device, model = restartPair(t, device, model)
		}
		assertAgainstModel(t, device, model)
	}
}

func FuzzHDDClientHistories(f *testing.F) {
	f.Add([]byte{0, 1, 1, 2, 3, 4, 5, 6, 7, 8, 2, 3, 2, 2, 0, 1, 3})
	f.Add([]byte{0, 7, 0xff, 0, 1, 2, 0, 0, 0, 0, 2, 0, 5, 0, 1, 3, 4})
	f.Add([]byte{3, 0, 2, 1, 0, 0, 0, 0, 3, 0, 2, 3})
	f.Fuzz(func(t *testing.T, input []byte) {
		if len(input) > 256 {
			t.Skip()
		}
		device := New()
		model := zeroModel()
		for cursor := 0; cursor < len(input); {
			op := input[cursor] % 5
			cursor++
			switch op {
			case 0:
				if cursor+modelBlockBytes+1 > len(input) {
					return
				}
				block := int64(input[cursor] % modelBlockCount)
				cursor++
				data := append([]byte(nil), input[cursor:cursor+modelBlockBytes]...)
				cursor += modelBlockBytes
				model.write(block, data)
				if err := device.WriteBlock(block, data); err != nil {
					t.Fatal(err)
				}
			case 1:
				if cursor >= len(input) {
					return
				}
				block := int64(input[cursor] % modelBlockCount)
				cursor++
				want := model.read(block)
				got, err := device.ReadBlock(block)
				if err != nil || !bytes.Equal(got, want[:]) {
					t.Fatalf("read block %d got %v err %v want %v", block, got, err, want)
				}
			case 2:
				cut := randomBarrierCutFromBytes(input, &cursor)
				wantModel, wantReceipt, wantErr := model.flushWithCut(cut)
				gotReceipt, gotErr := device.FlushWithCut(cut)
				if !sameError(gotErr, wantErr) || gotReceipt != wantReceipt {
					t.Fatalf("cut=%+v got=(%+v,%v), want=(%+v,%v)", cut, gotReceipt, gotErr, wantReceipt, wantErr)
				}
				if wantErr == nil {
					model = wantModel
				}
			case 3:
				model, _, _ = model.flushWithCut(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete})
				if err := device.Flush(); err != nil {
					t.Fatal(err)
				}
			case 4:
				device, model = restartPair(t, device, model)
			}
			assertAgainstModel(t, device, model)
		}
	})
}

type clientOperationKind uint8

const (
	clientRead clientOperationKind = iota
	clientWrite
	clientFlush
	clientRestart
)

type clientOperation struct {
	kind  clientOperationKind
	block int64
	data  []byte
}

type blockModel struct {
	durable [modelBlockCount][modelBlockBytes]byte
	pending [modelBlockCount][modelBlockBytes]byte
	dirty   [modelBlockCount]bool
}

func zeroModel() blockModel { return blockModel{} }

func (model *blockModel) write(block int64, data []byte) {
	copy(model.pending[block][:], data)
	model.dirty[block] = true
}

func (model blockModel) read(block int64) [modelBlockBytes]byte {
	if model.dirty[block] {
		return model.pending[block]
	}
	return model.durable[block]
}

func (model blockModel) flushWithCut(cut hddwritebackcache.BarrierCut) (blockModel, hddwritebackcache.BarrierReceipt, error) {
	dirty := make([]int, 0, modelBlockCount)
	for block, isDirty := range model.dirty {
		if isDirty {
			dirty = append(dirty, block)
		}
	}
	if err := modelValidateBarrier(dirty, cut); err != nil {
		return model, hddwritebackcache.BarrierReceipt{}, err
	}
	next := model
	receipt := hddwritebackcache.BarrierReceipt{}
	switch cut.Kind {
	case hddwritebackcache.BeforeFirstSector:
		next.dropPending()
		return next, receipt, nil
	case hddwritebackcache.AfterAllMedia, hddwritebackcache.BarrierComplete:
		for _, block := range dirty {
			next.durable[block] = next.pending[block]
			receipt.PriorCompletedSectors++
		}
		receipt.Acknowledged = cut.Kind == hddwritebackcache.BarrierComplete
		next.dropPending()
		return next, receipt, nil
	case hddwritebackcache.AtSector:
		ordinal := int(cut.SectorIndex)
		for index := 0; index < ordinal; index++ {
			block := dirty[index]
			next.durable[block] = next.pending[block]
			receipt.PriorCompletedSectors++
		}
		persisted, err := modelLowerPersistedPrefix(cut.MediaCut)
		if err != nil {
			return model, hddwritebackcache.BarrierReceipt{}, err
		}
		block := dirty[ordinal]
		copy(next.durable[block][:persisted], next.pending[block][:persisted])
		receipt.TerminalPrefixBytes = persisted
		next.dropPending()
		return next, receipt, nil
	default:
		return model, hddwritebackcache.BarrierReceipt{}, hddwritebackcache.ErrInvalidBarrierCut
	}
}

func modelValidateBarrier(dirty []int, cut hddwritebackcache.BarrierCut) error {
	switch cut.Kind {
	case hddwritebackcache.BeforeFirstSector, hddwritebackcache.AfterAllMedia, hddwritebackcache.BarrierComplete:
		if cut.SectorIndex != 0 || cut.MediaCut != (hddwritecrashcuts.Cut{}) {
			return hddwritebackcache.ErrInvalidBarrierCut
		}
		return nil
	case hddwritebackcache.AtSector:
		if cut.SectorIndex < 0 || cut.SectorIndex >= int64(len(dirty)) || cut.MediaCut.Kind == hddwritecrashcuts.Acknowledged {
			return hddwritebackcache.ErrInvalidBarrierCut
		}
		return nil
	default:
		return hddwritebackcache.ErrInvalidBarrierCut
	}
}

func modelLowerPersistedPrefix(cut hddwritecrashcuts.Cut) (int, error) {
	switch cut.Kind {
	case hddwritecrashcuts.BeforeMediaWrite:
		if cut.WrittenPrefix != nil {
			return 0, hddwritecrashcuts.ErrInvalidPrefix
		}
		return 0, nil
	case hddwritecrashcuts.DuringMediaWrite:
		if cut.WrittenPrefix == nil || *cut.WrittenPrefix < 1 || *cut.WrittenPrefix > modelMaximumPrefix {
			return 0, hddwritecrashcuts.ErrInvalidPrefix
		}
		return *cut.WrittenPrefix, nil
	case hddwritecrashcuts.AfterMediaWriteBeforeResponse:
		if cut.WrittenPrefix != nil {
			return 0, hddwritecrashcuts.ErrInvalidPrefix
		}
		return modelBlockBytes, nil
	case hddwritecrashcuts.Acknowledged:
		return 0, hddwritebackcache.ErrInvalidBarrierCut
	default:
		return 0, hddwritecrashcuts.ErrInvalidCut
	}
}

func (model *blockModel) dropPending() {
	model.pending = [modelBlockCount][modelBlockBytes]byte{}
	model.dirty = [modelBlockCount]bool{}
}

func dirtyThreeBlocks(t *testing.T) (*HDD, blockModel) {
	t.Helper()
	device := New()
	model := zeroModel()
	for index, block := range []int64{2, 5, 8} {
		data := blockBytes(byte(20 + index*10))
		model.write(block, data)
		if err := device.WriteBlock(block, data); err != nil {
			t.Fatal(err)
		}
	}
	model, _, err := model.flushWithCut(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete})
	if err != nil {
		t.Fatal(err)
	}
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	for index, block := range []int64{2, 5, 8} {
		data := blockBytes(byte(100 + index*10))
		model.write(block, data)
		if err := device.WriteBlock(block, data); err != nil {
			t.Fatal(err)
		}
	}
	return device, model
}

func assertAgainstModel(t *testing.T, device *HDD, model blockModel) {
	t.Helper()
	for block := int64(0); block < modelBlockCount; block++ {
		want := model.read(block)
		assertBlock(t, device, block, want[:])
	}
}

func assertBlock(t *testing.T, device blockdevicecontract.Device, block int64, want []byte) {
	t.Helper()
	got, err := device.ReadBlock(block)
	if err != nil {
		t.Fatalf("ReadBlock(%d): %v", block, err)
	}
	if !bytes.Equal(got, want) {
		t.Fatalf("ReadBlock(%d)=%v want %v", block, got, want)
	}
}

func assertClientsEqual(t *testing.T, left, right blockdevicecontract.Device) {
	t.Helper()
	for block := int64(0); block < modelBlockCount; block++ {
		first, firstErr := left.ReadBlock(block)
		second, secondErr := right.ReadBlock(block)
		if !sameError(firstErr, secondErr) || !bytes.Equal(first, second) {
			t.Fatalf("client block %d differs: (%v,%v) / (%v,%v)", block, first, firstErr, second, secondErr)
		}
	}
}

func restartClient(t *testing.T, client blockdevicecontract.Device) (blockdevicecontract.Device, error) {
	t.Helper()
	switch typed := client.(type) {
	case *memoryblockdevice.Memory:
		return typed.Restart()
	case *HDD:
		return typed.Restart()
	default:
		t.Fatalf("unknown client %T", client)
		return nil, nil
	}
}

func restartPair(t *testing.T, device *HDD, model blockModel) (*HDD, blockModel) {
	t.Helper()
	restarted, err := device.Restart()
	if err != nil {
		t.Fatal(err)
	}
	model.dropPending()
	return restarted, model
}

func blockBytes(value byte) []byte {
	data := make([]byte, modelBlockBytes)
	for index := range data {
		data[index] = value + byte(index)
	}
	return data
}

func sameError(left, right error) bool {
	if left == nil || right == nil {
		return left == right
	}
	return errors.Is(left, right) && errors.Is(right, left)
}

func randomBarrierCut(rng *rand.Rand) hddwritebackcache.BarrierCut {
	kind := rng.Intn(4)
	cut := hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierCutKind(kind)}
	if kind == int(hddwritebackcache.AtSector) {
		cut.SectorIndex = int64(rng.Intn(5) - 1)
		cut.MediaCut = randomMediaCut(rng)
	}
	return cut
}

func randomMediaCut(rng *rand.Rand) hddwritecrashcuts.Cut {
	kind := rng.Intn(6)
	cut := hddwritecrashcuts.Cut{}
	switch kind {
	case 0:
		cut.Kind = hddwritecrashcuts.BeforeMediaWrite
	case 1:
		cut.Kind = hddwritecrashcuts.DuringMediaWrite
		prefix := rng.Intn(modelBlockBytes+2) - 1
		cut.WrittenPrefix = &prefix
	case 2:
		cut.Kind = hddwritecrashcuts.AfterMediaWriteBeforeResponse
	case 3:
		cut.Kind = hddwritecrashcuts.Acknowledged
	case 4:
		cut.Kind = "unknown"
	case 5:
		cut.Kind = hddwritecrashcuts.BeforeMediaWrite
		prefix := 0
		cut.WrittenPrefix = &prefix
	}
	return cut
}

func randomBarrierCutFromBytes(input []byte, cursor *int) hddwritebackcache.BarrierCut {
	nextByte := func() byte {
		if *cursor >= len(input) {
			return 0
		}
		value := input[*cursor]
		*cursor++
		return value
	}
	cut := hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierCutKind(nextByte() % 4)}
	if cut.Kind == hddwritebackcache.AtSector {
		cut.SectorIndex = int64(nextByte()%5) - 1
		kind := nextByte() % 6
		switch kind {
		case 0:
			cut.MediaCut.Kind = hddwritecrashcuts.BeforeMediaWrite
		case 1:
			cut.MediaCut.Kind = hddwritecrashcuts.DuringMediaWrite
			prefix := int(nextByte()%10) - 1
			cut.MediaCut.WrittenPrefix = &prefix
		case 2:
			cut.MediaCut.Kind = hddwritecrashcuts.AfterMediaWriteBeforeResponse
		case 3:
			cut.MediaCut.Kind = hddwritecrashcuts.Acknowledged
		case 4:
			cut.MediaCut.Kind = "unknown"
		case 5:
			cut.MediaCut.Kind = hddwritecrashcuts.BeforeMediaWrite
			prefix := int(nextByte())
			cut.MediaCut.WrittenPrefix = &prefix
		}
	}
	return cut
}
