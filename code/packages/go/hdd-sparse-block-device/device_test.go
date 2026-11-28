package hddsparseblockdevice

import (
	"fmt"
	"math"
	"math/rand"
	"reflect"
	"sort"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
	hddgeometry "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-geometry"
	hddseektiming "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-seek-timing"
	hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"
	hddwritebackcache "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-writeback-cache"
)

const modelMaxTick uint64 = 9_007_199_254_740_991

type visibleDurableModel struct {
	geometry hddgeometry.Geometry
	logical  blockdevicecontract.Geometry
	state    hddseektiming.State
	durable  map[int64][bytesPerBlock]byte
	pending  map[int64][bytesPerBlock]byte
}

func newModel(geometry hddgeometry.Geometry) *visibleDurableModel {
	logical, _ := geometry.BlockGeometry()
	return &visibleDurableModel{geometry: geometry, logical: logical, durable: make(map[int64][bytesPerBlock]byte), pending: make(map[int64][bytesPerBlock]byte)}
}

func (model *visibleDurableModel) read(block int64) ([]byte, error) {
	if err := model.logical.CheckBlock(block); err != nil {
		return nil, err
	}
	if value, ok := model.pending[block]; ok {
		return copyBlock(value), nil
	}
	if _, end, err := steppedTiming(model.geometry, model.state, block); err != nil {
		return nil, err
	} else {
		model.state = end
	}
	return copyBlock(model.durable[block]), nil
}

func (model *visibleDurableModel) write(block int64, data []byte) error {
	if err := model.logical.CheckWrite(block, data); err != nil {
		return err
	}
	var value [bytesPerBlock]byte
	copy(value[:], data)
	model.pending[block] = value
	return nil
}

func (model *visibleDurableModel) flush(cut hddwritebackcache.BarrierCut) (hddwritebackcache.BarrierReceipt, error) {
	dirty := make([]int64, 0, len(model.pending))
	for block := range model.pending {
		dirty = append(dirty, block)
	}
	sort.Slice(dirty, func(i, j int) bool { return dirty[i] < dirty[j] })
	terminalPrefix, stepCount, receipt, err := modelBarrier(dirty, cut)
	if err != nil {
		return hddwritebackcache.BarrierReceipt{}, err
	}
	ends := make([]hddseektiming.State, stepCount)
	state := model.state
	for index := 0; index < stepCount; index++ {
		_, end, err := steppedTiming(model.geometry, state, dirty[index])
		if err != nil {
			return hddwritebackcache.BarrierReceipt{}, err
		}
		ends[index] = end
		state = end
	}
	for index := 0; index < stepCount; index++ {
		block := dirty[index]
		prefix := bytesPerBlock
		if cut.Kind == hddwritebackcache.AtSector && index == int(cut.SectorIndex) {
			prefix = terminalPrefix
		}
		old := model.durable[block]
		newValue := model.pending[block]
		copy(old[:prefix], newValue[:prefix])
		if old == ([bytesPerBlock]byte{}) {
			delete(model.durable, block)
		} else {
			model.durable[block] = old
		}
		model.state = ends[index]
		if cut.Kind == hddwritebackcache.AtSector && index == int(cut.SectorIndex) && prefix == 0 {
			model.state.Tick--
		}
	}
	model.pending = make(map[int64][bytesPerBlock]byte)
	return receipt, nil
}

func modelBarrier(dirty []int64, cut hddwritebackcache.BarrierCut) (prefix, count int, receipt hddwritebackcache.BarrierReceipt, err error) {
	zero := hddwritecrashcuts.Cut{}
	switch cut.Kind {
	case hddwritebackcache.BeforeFirstSector, hddwritebackcache.AfterAllMedia, hddwritebackcache.BarrierComplete:
		if cut.SectorIndex != 0 || cut.MediaCut != zero {
			return 0, 0, receipt, hddwritebackcache.ErrInvalidBarrierCut
		}
		if cut.Kind == hddwritebackcache.BeforeFirstSector {
			return 0, 0, receipt, nil
		}
		count = len(dirty)
		if cut.Kind == hddwritebackcache.BarrierComplete {
			receipt.Acknowledged = true
		}
		receipt.PriorCompletedSectors = count
		return 0, count, receipt, nil
	case hddwritebackcache.AtSector:
		if cut.SectorIndex < 0 || cut.SectorIndex >= int64(len(dirty)) || cut.MediaCut.Kind == hddwritecrashcuts.Acknowledged {
			return 0, 0, receipt, hddwritebackcache.ErrInvalidBarrierCut
		}
		prefix, _, err = modelCut(cut.MediaCut)
		if err != nil {
			return 0, 0, receipt, err
		}
		count = int(cut.SectorIndex) + 1
		receipt.PriorCompletedSectors = int(cut.SectorIndex)
		receipt.TerminalPrefixBytes = prefix
		return prefix, count, receipt, nil
	default:
		return 0, 0, receipt, hddwritebackcache.ErrInvalidBarrierCut
	}
}

func modelCut(cut hddwritecrashcuts.Cut) (int, bool, error) {
	switch cut.Kind {
	case hddwritecrashcuts.BeforeMediaWrite:
		if cut.WrittenPrefix != nil {
			return 0, false, hddwritecrashcuts.ErrInvalidPrefix
		}
		return 0, false, nil
	case hddwritecrashcuts.DuringMediaWrite:
		if cut.WrittenPrefix == nil || *cut.WrittenPrefix < 1 || *cut.WrittenPrefix > bytesPerBlock-1 {
			return 0, false, hddwritecrashcuts.ErrInvalidPrefix
		}
		return *cut.WrittenPrefix, false, nil
	case hddwritecrashcuts.AfterMediaWriteBeforeResponse:
		if cut.WrittenPrefix != nil {
			return 0, false, hddwritecrashcuts.ErrInvalidPrefix
		}
		return bytesPerBlock, false, nil
	case hddwritecrashcuts.Acknowledged:
		if cut.WrittenPrefix != nil {
			return 0, false, hddwritecrashcuts.ErrInvalidPrefix
		}
		return bytesPerBlock, true, nil
	default:
		return 0, false, hddwritecrashcuts.ErrInvalidCut
	}
}

// steppedTiming uses cylinder, sector, and transfer ticks rather than the
// production timing planner.
func steppedTiming(geometry hddgeometry.Geometry, current hddseektiming.State, block int64) (hddseektiming.Result, hddseektiming.State, error) {
	logical, err := geometry.BlockGeometry()
	if err != nil {
		return hddseektiming.Result{}, hddseektiming.State{}, err
	}
	if err := logical.CheckBlock(block); err != nil {
		return hddseektiming.Result{}, hddseektiming.State{}, err
	}
	target := hddgeometry.Address{
		Cylinder: (block / geometry.SectorsPerTrack) / geometry.Surfaces,
		Surface:  (block / geometry.SectorsPerTrack) % geometry.Surfaces,
		Sector:   block % geometry.SectorsPerTrack,
	}
	if current.HeadCylinder < 0 || current.HeadCylinder >= geometry.Cylinders {
		return hddseektiming.Result{}, hddseektiming.State{}, hddseektiming.ErrHeadCylinder
	}
	if current.Tick > modelMaxTick {
		return hddseektiming.Result{}, hddseektiming.State{}, hddseektiming.ErrClockOverflow
	}
	state := current
	var seek, rotation uint64
	for state.HeadCylinder != target.Cylinder {
		if state.Tick > modelMaxTick-3 {
			return hddseektiming.Result{}, hddseektiming.State{}, hddseektiming.ErrClockOverflow
		}
		if state.HeadCylinder < target.Cylinder {
			state.HeadCylinder++
		} else {
			state.HeadCylinder--
		}
		state.Tick += 3
		seek += 3
	}
	period := uint64(geometry.SectorsPerTrack)
	for state.Tick%period != uint64(target.Sector) {
		if state.Tick == modelMaxTick {
			return hddseektiming.Result{}, hddseektiming.State{}, hddseektiming.ErrClockOverflow
		}
		state.Tick++
		rotation++
	}
	if state.Tick == modelMaxTick {
		return hddseektiming.Result{}, hddseektiming.State{}, hddseektiming.ErrClockOverflow
	}
	state.Tick++
	result := hddseektiming.Result{
		Target: target, End: state, SeekTicks: seek, RotationTicks: rotation,
		TransferTicks: 1, TotalTicks: seek + rotation + 1,
	}
	return result, state, nil
}

func compareModel(t testing.TB, device *Device, model *visibleDurableModel) {
	t.Helper()
	snapshot, err := device.medium.Snapshot()
	if err != nil {
		t.Fatal(err)
	}
	want := make(map[int64][bytesPerBlock]byte, len(model.durable))
	for block, value := range model.durable {
		if value != ([bytesPerBlock]byte{}) {
			want[block] = value
		}
	}
	if snapshot.Geometry != model.geometry || snapshot.State != model.state || !reflect.DeepEqual(snapshot.Sectors, want) {
		t.Fatalf("durable snapshot got=%+v want geometry=%+v state=%+v sectors=%v", snapshot, model.geometry, model.state, want)
	}
	if !reflect.DeepEqual(device.pending, model.pending) {
		t.Fatalf("pending overlay got=%v want=%v", device.pending, model.pending)
	}
	position, err := device.medium.Position()
	if err != nil || position != model.state {
		t.Fatalf("position got=%+v,%v want=%+v", position, err, model.state)
	}
}

func dataFor(block int64, salt byte) []byte {
	data := make([]byte, bytesPerBlock)
	for index := range data {
		data[index] = byte((int(block)*29+index*37+int(salt)*11)%251 + 1)
	}
	return data
}

func allBarrierCuts(dirtyCount int) []hddwritebackcache.BarrierCut {
	cuts := []hddwritebackcache.BarrierCut{
		{Kind: hddwritebackcache.BeforeFirstSector},
		{Kind: hddwritebackcache.AfterAllMedia},
		{Kind: hddwritebackcache.BarrierComplete},
	}
	lower := []hddwritecrashcuts.Cut{{Kind: hddwritecrashcuts.BeforeMediaWrite}}
	for prefix := 1; prefix < bytesPerBlock; prefix++ {
		value := prefix
		lower = append(lower, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &value})
	}
	lower = append(lower, hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse})
	for ordinal := 0; ordinal < dirtyCount; ordinal++ {
		for _, mediaCut := range lower {
			cuts = append(cuts, hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: int64(ordinal), MediaCut: mediaCut})
		}
	}
	return cuts
}

func TestDeviceContractAndNilZeroAdmission(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16}
	device, err := New(geometry)
	if err != nil {
		t.Fatal(err)
	}
	wantGeometry := blockdevicecontract.Geometry{BlockCount: 512, BytesPerBlock: 8}
	if got := device.Geometry(); got != wantGeometry {
		t.Fatalf("geometry=%+v want=%+v", got, wantGeometry)
	}
	if _, err := New(hddgeometry.Geometry{}); err != blockdevicecontract.ErrInvalidGeometry {
		t.Fatalf("invalid construction error=%v", err)
	}
	var nilDevice *Device
	if nilDevice.Geometry() != (blockdevicecontract.Geometry{}) {
		t.Fatalf("nil geometry=%+v", nilDevice.Geometry())
	}
	if got, err := nilDevice.ReadBlock(0); err != blockdevicecontract.ErrInvalidGeometry || got != nil {
		t.Fatalf("nil read=(%v,%v)", got, err)
	}
	if err := nilDevice.WriteBlock(0, make([]byte, 8)); err != blockdevicecontract.ErrInvalidGeometry {
		t.Fatalf("nil write error=%v", err)
	}
	if receipt, err := nilDevice.FlushWithCut(hddwritebackcache.BarrierCut{}); err != blockdevicecontract.ErrInvalidGeometry || receipt != (hddwritebackcache.BarrierReceipt{}) {
		t.Fatalf("nil flush=(%+v,%v)", receipt, err)
	}
	if err := nilDevice.Flush(); err != blockdevicecontract.ErrInvalidGeometry {
		t.Fatalf("nil Flush error=%v", err)
	}
	if restarted, err := nilDevice.Restart(); err != blockdevicecontract.ErrInvalidGeometry || restarted != nil {
		t.Fatalf("nil restart=(%v,%v)", restarted, err)
	}
	zero := &Device{}
	if zero.Geometry() != (blockdevicecontract.Geometry{}) {
		t.Fatalf("zero geometry=%+v", zero.Geometry())
	}
	if got, err := zero.ReadBlock(0); err != blockdevicecontract.ErrInvalidGeometry || got != nil {
		t.Fatalf("zero read=(%v,%v)", got, err)
	}
	if err := zero.WriteBlock(0, make([]byte, 8)); err != blockdevicecontract.ErrInvalidGeometry {
		t.Fatalf("zero write error=%v", err)
	}
	if receipt, err := zero.FlushWithCut(hddwritebackcache.BarrierCut{}); err != blockdevicecontract.ErrInvalidGeometry || receipt != (hddwritebackcache.BarrierReceipt{}) {
		t.Fatalf("zero flush=(%+v,%v)", receipt, err)
	}
	if err := zero.Flush(); err != blockdevicecontract.ErrInvalidGeometry {
		t.Fatalf("zero Flush error=%v", err)
	}
	if restarted, err := zero.Restart(); err != blockdevicecontract.ErrInvalidGeometry || restarted != nil {
		t.Fatalf("zero restart=(%v,%v)", restarted, err)
	}
}

func TestPendingReadHasNoDeviceEffectAndBytesAreDetached(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16}
	device, _ := New(geometry)
	model := newModel(geometry)
	pending := dataFor(19, 1)
	input := append([]byte(nil), pending...)
	if err := device.WriteBlock(19, pending); err != nil {
		t.Fatal(err)
	}
	if err := model.write(19, input); err != nil {
		t.Fatal(err)
	}
	for index := range pending {
		pending[index] ^= 0xff
	}
	compareModel(t, device, model)
	before, err := device.medium.Position()
	if err != nil {
		t.Fatal(err)
	}
	got, err := device.ReadBlock(19)
	if err != nil || !reflect.DeepEqual(got, input) {
		t.Fatalf("pending read=(%v,%v), want=%v", got, err, input)
	}
	got[0] ^= 0xff
	again, err := device.ReadBlock(19)
	if err != nil || !reflect.DeepEqual(again, input) {
		t.Fatalf("mutated pending read escaped: (%v,%v)", again, err)
	}
	after, err := device.medium.Position()
	if err != nil || after != before {
		t.Fatalf("dirty read advanced medium: before=%+v after=%+v err=%v", before, after, err)
	}
	compareModel(t, device, model)
	miss, err := device.ReadBlock(20)
	if err != nil || !reflect.DeepEqual(miss, make([]byte, 8)) {
		t.Fatalf("sparse miss=(%v,%v)", miss, err)
	}
	wantMiss, modelErr := model.read(20)
	if modelErr != nil || !reflect.DeepEqual(miss, wantMiss) {
		t.Fatalf("miss model=(%v,%v), want=%v", miss, modelErr, wantMiss)
	}
	compareModel(t, device, model)
}

func TestWriteBlockRemainsVolatileBeforeFlush(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16}
	device, err := New(geometry)
	if err != nil {
		t.Fatal(err)
	}
	data := dataFor(21, 4)
	if err := device.WriteBlock(21, data); err != nil {
		t.Fatal(err)
	}
	snapshot, err := device.medium.Snapshot()
	if err != nil {
		t.Fatal(err)
	}
	position, err := device.medium.Position()
	if err != nil {
		t.Fatal(err)
	}
	if len(snapshot.Sectors) != 0 || position != (hddseektiming.State{}) {
		t.Fatalf("write changed durable media before Flush: sectors=%v position=%+v", snapshot.Sectors, position)
	}
	visible, err := device.ReadBlock(21)
	if err != nil || !reflect.DeepEqual(visible, data) {
		t.Fatalf("pending visible read=(%v,%v) want=%v", visible, err, data)
	}
	snapshot, err = device.medium.Snapshot()
	if err != nil || len(snapshot.Sectors) != 0 {
		t.Fatalf("dirty read changed durable media before Flush: snapshot=%+v error=%v", snapshot, err)
	}
}

func TestRestartDropsPendingAndPreservesOnlyDurableImage(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16}
	device, err := New(geometry)
	if err != nil {
		t.Fatal(err)
	}
	model := newModel(geometry)
	durable := dataFor(11, 8)
	if err := device.WriteBlock(11, durable); err != nil {
		t.Fatal(err)
	}
	if err := model.write(11, durable); err != nil {
		t.Fatal(err)
	}
	receipt, err := device.FlushWithCut(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete})
	wantReceipt, modelErr := model.flush(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete})
	if err != nil || modelErr != nil || receipt != wantReceipt {
		t.Fatalf("seed flush=(%+v,%v) want=(%+v,%v)", receipt, err, wantReceipt, modelErr)
	}
	pending := dataFor(12, 9)
	if err := device.WriteBlock(12, pending); err != nil {
		t.Fatal(err)
	}
	if err := model.write(12, pending); err != nil {
		t.Fatal(err)
	}
	var expectedPending [bytesPerBlock]byte
	copy(expectedPending[:], pending)
	before, _ := device.medium.Snapshot()
	restarted, err := device.Restart()
	if err != nil {
		t.Fatal(err)
	}
	model.pending = make(map[int64][bytesPerBlock]byte)
	model.state = hddseektiming.State{}
	after, err := restarted.medium.Snapshot()
	if err != nil || after.Geometry != geometry || after.State != (hddseektiming.State{}) || !reflect.DeepEqual(after.Sectors, before.Sectors) {
		t.Fatalf("restart snapshot=(%+v,%v), before=%+v", after, err, before)
	}
	compareModel(t, restarted, model)
	got, err := restarted.ReadBlock(12)
	if err != nil || !reflect.DeepEqual(got, make([]byte, bytesPerBlock)) {
		t.Fatalf("discarded pending read=(%v,%v)", got, err)
	}
	if _, err := device.ReadBlock(12); err != nil { // Original device still owns its pending write.
		t.Fatal(err)
	}
	original, _ := device.medium.Snapshot()
	if len(original.Sectors) != len(before.Sectors) || device.pending[12] != expectedPending {
		t.Fatalf("restart shared volatile state with original: original=%+v pending=%v", original, device.pending)
	}
}

func TestDurableReadBytesAreDetachedAndZeroBlocksStaySparse(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16}
	device, err := New(geometry)
	if err != nil {
		t.Fatal(err)
	}
	want := dataFor(6, 0x51)
	if err := device.WriteBlock(6, want); err != nil {
		t.Fatal(err)
	}
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	first, err := device.ReadBlock(6)
	if err != nil || !reflect.DeepEqual(first, want) {
		t.Fatalf("durable read=(%v,%v) want=%v", first, err, want)
	}
	first[0] ^= 0xff
	second, err := device.ReadBlock(6)
	if err != nil || !reflect.DeepEqual(second, want) {
		t.Fatalf("durable result mutation escaped: (%v,%v) want=%v", second, err, want)
	}
	zero := make([]byte, bytesPerBlock)
	if err := device.WriteBlock(7, zero); err != nil {
		t.Fatal(err)
	}
	if got, err := device.ReadBlock(7); err != nil || !reflect.DeepEqual(got, zero) {
		t.Fatalf("zero pending block=(%v,%v)", got, err)
	}
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	snapshot, err := device.medium.Snapshot()
	if err != nil || len(snapshot.Sectors) != 1 || snapshot.Sectors[6] != [bytesPerBlock]byte{want[0], want[1], want[2], want[3], want[4], want[5], want[6], want[7]} {
		t.Fatalf("zero block affected sparse durable image: %+v error=%v", snapshot, err)
	}
}

func TestCacheMissReturnsTimedMediumErrorWithoutChangingState(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 1, Surfaces: 1, SectorsPerTrack: int64(modelMaxTick)}
	device, err := New(geometry)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := device.ReadBlock(int64(modelMaxTick - 2)); err != nil {
		t.Fatal(err)
	}
	before, err := device.medium.Position()
	if err != nil || before.Tick != modelMaxTick-1 {
		t.Fatalf("near-ceiling setup=(%+v,%v)", before, err)
	}
	if got, err := device.ReadBlock(0); err != hddseektiming.ErrClockOverflow || got != nil {
		t.Fatalf("cache miss did not preserve timed error: (%v,%v)", got, err)
	}
	after, err := device.medium.Position()
	if err != nil || after != before {
		t.Fatalf("refused cache miss changed time: before=%+v after=%+v error=%v", before, after, err)
	}
}

func TestWriteValidationAndCachePathsAcrossBothProfiles(t *testing.T) {
	profiles := []hddgeometry.Geometry{
		{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16},
		{Cylinders: 64, Surfaces: 2, SectorsPerTrack: 4},
	}
	for _, geometry := range profiles {
		t.Run(fmt.Sprintf("sectors-%d", geometry.SectorsPerTrack), func(t *testing.T) {
			device, err := New(geometry)
			if err != nil {
				t.Fatal(err)
			}
			model := newModel(geometry)
			for block := int64(0); block < geometry.Cylinders*geometry.Surfaces*geometry.SectorsPerTrack; block++ {
				data := dataFor(block, byte(block))
				if err := device.WriteBlock(block, data); err != nil {
					t.Fatal(err)
				}
				if err := model.write(block, data); err != nil {
					t.Fatal(err)
				}
				if block%17 == 0 {
					got, err := device.ReadBlock(block)
					want, modelErr := model.read(block)
					if err != nil || modelErr != nil || !reflect.DeepEqual(got, want) {
						t.Fatalf("pending read block %d: %v,%v want %v,%v", block, got, err, want, modelErr)
					}
				}
			}
			compareModel(t, device, model)
			receipt, err := device.FlushWithCut(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete})
			modelReceipt, modelErr := model.flush(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete})
			if err != nil || modelErr != nil || receipt != modelReceipt || receipt.PriorCompletedSectors != int(geometry.Cylinders*geometry.Surfaces*geometry.SectorsPerTrack) || !receipt.Acknowledged {
				t.Fatalf("complete page write=(%+v,%v) want (%+v,%v)", receipt, err, modelReceipt, modelErr)
			}
			compareModel(t, device, model)
			for block := int64(0); block < geometry.Cylinders*geometry.Surfaces*geometry.SectorsPerTrack; block++ {
				got, err := device.ReadBlock(block)
				want, modelErr := model.read(block)
				if err != nil || modelErr != nil || !reflect.DeepEqual(got, want) {
					t.Fatalf("durable read block %d: %v,%v want %v,%v", block, got, err, want, modelErr)
				}
			}
			compareModel(t, device, model)
			if err := device.WriteBlock(-1, nil); err != blockdevicecontract.ErrInvalidBlock {
				t.Fatalf("address before width: %v", err)
			}
			if err := device.WriteBlock(0, make([]byte, 7)); err != blockdevicecontract.ErrInvalidBlockData {
				t.Fatalf("width error=%v", err)
			}
			if block, err := device.ReadBlock(math.MaxInt64); err != blockdevicecontract.ErrInvalidBlock || block != nil {
				t.Fatalf("signed read=(%v,%v)", block, err)
			}
		})
	}
}

func TestEveryBarrierCutMatchesVisibleDurableModel(t *testing.T) {
	profiles := []hddgeometry.Geometry{
		{Cylinders: 16, Surfaces: 2, SectorsPerTrack: 16},
		{Cylinders: 64, Surfaces: 2, SectorsPerTrack: 4},
	}
	checked := 0
	for _, geometry := range profiles {
		for _, cut := range allBarrierCuts(3) {
			device, err := New(geometry)
			if err != nil {
				t.Fatal(err)
			}
			model := newModel(geometry)
			// Seed distinct durable bytes before staging the captured dirty set.
			for _, block := range []int64{2, 5, 8} {
				data := dataFor(block, 0x22)
				if err := device.WriteBlock(block, data); err != nil {
					t.Fatal(err)
				}
				if err := model.write(block, data); err != nil {
					t.Fatal(err)
				}
			}
			if err := device.Flush(); err != nil {
				t.Fatal(err)
			}
			if _, err := model.flush(hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete}); err != nil {
				t.Fatal(err)
			}
			for _, block := range []int64{8, 2, 5} {
				data := dataFor(block, byte(checked+3))
				if err := device.WriteBlock(block, data); err != nil {
					t.Fatal(err)
				}
				if err := model.write(block, data); err != nil {
					t.Fatal(err)
				}
			}
			compareModel(t, device, model)
			receipt, err := device.FlushWithCut(cut)
			wantReceipt, modelErr := model.flush(cut)
			if err != nil || modelErr != nil || receipt != wantReceipt {
				t.Fatalf("geometry=%+v cut=%+v got=(%+v,%v) want=(%+v,%v)", geometry, cut, receipt, err, wantReceipt, modelErr)
			}
			compareModel(t, device, model)
			// Pending data is cleared after every accepted barrier, including a
			// prefix cut, lost response, and the before-first power-loss cut.
			for _, block := range []int64{2, 5, 8} {
				got, err := device.ReadBlock(block)
				want, modelErr := model.read(block)
				if err != nil || modelErr != nil || !reflect.DeepEqual(got, want) {
					t.Fatalf("after barrier block=%d cut=%+v got=(%v,%v) want=(%v,%v)", block, cut, got, err, want, modelErr)
				}
			}
			compareModel(t, device, model)
			checked++
		}
	}
	if checked != 60 {
		t.Fatalf("barrier cases=%d, want 60", checked)
	}
	t.Logf("independent visible/durable model checked %d cuts across both 512-block profiles", checked)
}

func applyHistoryCommand(t testing.TB, device *Device, model *visibleDurableModel, command byte, step int) {
	t.Helper()
	logical := int64(command>>3) % model.logical.BlockCount
	switch command & 3 {
	case 0:
		got, err := device.ReadBlock(logical)
		want, modelErr := model.read(logical)
		if err != nil || modelErr != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("history read step=%d block=%d got=(%v,%v) want=(%v,%v)", step, logical, got, err, want, modelErr)
		}
	case 1:
		data := dataFor(logical, byte(command+byte(step)))
		if err := device.WriteBlock(logical, data); err != nil {
			t.Fatalf("history write step=%d: %v", step, err)
		}
		if err := model.write(logical, data); err != nil {
			t.Fatalf("history model write step=%d: %v", step, err)
		}
		for index := range data {
			data[index] ^= 0xff
		}
	case 2:
		dirty := make([]int64, 0, len(model.pending))
		for block := range model.pending {
			dirty = append(dirty, block)
		}
		sort.Slice(dirty, func(i, j int) bool { return dirty[i] < dirty[j] })
		cut := hddwritebackcache.BarrierCut{}
		switch (command >> 2) % 3 {
		case 0:
			cut.Kind = hddwritebackcache.BeforeFirstSector
		case 1:
			cut.Kind = hddwritebackcache.AfterAllMedia
		case 2:
			cut.Kind = hddwritebackcache.BarrierComplete
		}
		if command&0x20 != 0 && len(dirty) > 0 {
			ordinal := int(command>>6) % len(dirty)
			cut = hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: int64(ordinal)}
			switch (command >> 4) % 9 {
			case 0:
				cut.MediaCut = hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.BeforeMediaWrite}
			case 1, 2, 3, 4, 5, 6, 7:
				prefix := int((command>>4)%7) + 1
				cut.MediaCut = hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &prefix}
			case 8:
				cut.MediaCut = hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.AfterMediaWriteBeforeResponse}
			}
		}
		got, err := device.FlushWithCut(cut)
		want, modelErr := model.flush(cut)
		if err != nil || modelErr != nil || got != want {
			t.Fatalf("history flush step=%d cut=%+v got=(%+v,%v) want=(%+v,%v)", step, cut, got, err, want, modelErr)
		}
	case 3:
		restarted, err := device.Restart()
		if err != nil {
			t.Fatalf("history restart step=%d: %v", step, err)
		}
		*device = *restarted
		model.pending = make(map[int64][bytesPerBlock]byte)
		model.state = hddseektiming.State{}
	}
	compareModel(t, device, model)
}

func TestFixedSeededVisibleDurableHistories(t *testing.T) {
	geometry := hddgeometry.Geometry{Cylinders: 4, Surfaces: 2, SectorsPerTrack: 4}
	device, err := New(geometry)
	if err != nil {
		t.Fatal(err)
	}
	model := newModel(geometry)
	random := rand.New(rand.NewSource(0x6d06b2d3c2))
	for step := 0; step < 10_000; step++ {
		applyHistoryCommand(t, device, model, byte(random.Intn(256)), step)
	}
	t.Log("independent visible/durable model checked 10000 deterministic operations")
}

func FuzzSparseBlockDeviceHistory(f *testing.F) {
	f.Add([]byte{0, 1, 2, 3, 0x20, 0x30, 0x70, 0x21, 0x41, 0x62})
	f.Add([]byte{0xff, 0x01, 0x81, 0x42, 0x23, 0xc3, 0x64, 0xa5})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 128 {
			return
		}
		geometry := hddgeometry.Geometry{Cylinders: 4, Surfaces: 2, SectorsPerTrack: 4}
		device, err := New(geometry)
		if err != nil {
			t.Fatal(err)
		}
		model := newModel(geometry)
		for step, command := range commands {
			applyHistoryCommand(t, device, model, command, step)
		}
	})
}

func TestPreflightRefusalPreservesEarlierBlocksAndVolatileWrites(t *testing.T) {
	sectorsPerTrack := (int64(modelMaxTick) - 1) / 2
	geometry := hddgeometry.Geometry{Cylinders: 1, Surfaces: 2, SectorsPerTrack: sectorsPerTrack}
	device, _ := New(geometry)
	model := newModel(geometry)
	for index := range 2 {
		if _, err := device.ReadBlock(sectorsPerTrack - 2); err != nil {
			t.Fatalf("clock positioning read %d: %v", index, err)
		}
	}
	position, err := device.medium.Position()
	if err != nil || position != (hddseektiming.State{HeadCylinder: 0, Tick: modelMaxTick - 2}) {
		t.Fatalf("clock positioning ended at (%+v,%v)", position, err)
	}
	model.state = position
	for _, block := range []int64{sectorsPerTrack - 1, sectorsPerTrack, sectorsPerTrack + 1} {
		data := dataFor(block, 5)
		if err := device.WriteBlock(block, data); err != nil {
			t.Fatal(err)
		}
		if err := model.write(block, data); err != nil {
			t.Fatal(err)
		}
	}
	before, _ := device.medium.Snapshot()
	cut := hddwritebackcache.BarrierCut{Kind: hddwritebackcache.BarrierComplete}
	if receipt, err := device.FlushWithCut(cut); err != hddseektiming.ErrClockOverflow || receipt != (hddwritebackcache.BarrierReceipt{}) {
		t.Fatalf("later-step refusal=(%+v,%v)", receipt, err)
	}
	if _, err := model.flush(cut); err != hddseektiming.ErrClockOverflow {
		t.Fatalf("independent later-step error=%v", err)
	}
	after, _ := device.medium.Snapshot()
	if !reflect.DeepEqual(before, after) {
		t.Fatalf("refused plan changed durable medium: before=%+v after=%+v", before, after)
	}
	compareModel(t, device, model)
	position, err = device.medium.Position()
	if err != nil || position.Tick != modelMaxTick-2 {
		t.Fatalf("timing refusal changed clock: position=%+v error=%v", position, err)
	}
	invalid := hddwritebackcache.BarrierCut{Kind: hddwritebackcache.AtSector, SectorIndex: 9}
	if receipt, err := device.FlushWithCut(invalid); err != hddwritebackcache.ErrInvalidBarrierCut || receipt != (hddwritebackcache.BarrierReceipt{}) {
		t.Fatalf("invalid cut refusal=(%+v,%v)", receipt, err)
	}
	compareModel(t, device, model)
}
