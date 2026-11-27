package nandflash

import (
	"bytes"
	"encoding/hex"
	"errors"
	"math/rand"
	"reflect"
	"sync"
	"testing"
)

func testDevice(t *testing.T, geometry Geometry, limit *uint64) *Device {
	t.Helper()
	device, err := New(Config{Geometry: geometry, MaxEraseCycles: limit})
	if err != nil {
		t.Fatalf("New(%+v): %v", geometry, err)
	}
	return device
}

func TestConstructorGeometryBoundsAndProducts(t *testing.T) {
	valid := []Geometry{
		{Blocks: 1, PagesPerBlock: 1, PageSizeBytes: 1},
		{Blocks: maxBlocks, PagesPerBlock: 1, PageSizeBytes: 1},
		{Blocks: 1, PagesPerBlock: maxPagesPerBlock, PageSizeBytes: 1},
		{Blocks: 1, PagesPerBlock: 1, PageSizeBytes: maxPageSizeBytes},
		{Blocks: 1, PagesPerBlock: maxPagesPerBlock, PageSizeBytes: maxPageSizeBytes}, // exactly 16 MiB
	}
	for _, geometry := range valid {
		t.Run("valid", func(t *testing.T) {
			device := testDevice(t, geometry, nil)
			got, err := device.Geometry()
			if err != nil || got != geometry {
				t.Fatalf("Geometry() = %+v, %v; want %+v", got, err, geometry)
			}
		})
	}
	maxInt := int(^uint(0) >> 1)
	invalid := []Geometry{
		{},
		{Blocks: 0, PagesPerBlock: 1, PageSizeBytes: 1},
		{Blocks: -1, PagesPerBlock: 1, PageSizeBytes: 1},
		{Blocks: maxBlocks + 1, PagesPerBlock: 1, PageSizeBytes: 1},
		{Blocks: 1, PagesPerBlock: 0, PageSizeBytes: 1},
		{Blocks: 1, PagesPerBlock: -1, PageSizeBytes: 1},
		{Blocks: 1, PagesPerBlock: maxPagesPerBlock + 1, PageSizeBytes: 1},
		{Blocks: 1, PagesPerBlock: 1, PageSizeBytes: 0},
		{Blocks: 1, PagesPerBlock: 1, PageSizeBytes: -1},
		{Blocks: 1, PagesPerBlock: 1, PageSizeBytes: maxPageSizeBytes + 1},
		{Blocks: 2, PagesPerBlock: maxPagesPerBlock, PageSizeBytes: maxPageSizeBytes},
		{Blocks: maxInt, PagesPerBlock: maxInt, PageSizeBytes: maxInt},
	}
	for _, geometry := range invalid {
		if device, err := New(Config{Geometry: geometry}); !errors.Is(err, ErrGeometry) || device != nil {
			t.Errorf("New(%+v) = %v, %v; want nil, ErrGeometry", geometry, device, err)
		}
	}
	tooLarge := maxSafeEraseCycles + 1
	if device, err := New(Config{Geometry: Geometry{1, 1, 1}, MaxEraseCycles: &tooLarge}); !errors.Is(err, ErrGeometry) || device != nil {
		t.Fatalf("New with excessive erase ceiling = %v, %v", device, err)
	}
}

func TestInitialStateAndGeometryCopy(t *testing.T) {
	device := testDevice(t, Geometry{Blocks: 2, PagesPerBlock: 2, PageSizeBytes: 2}, nil)
	geometry, err := device.Geometry()
	if err != nil {
		t.Fatal(err)
	}
	geometry.Blocks = 900
	geometry.PagesPerBlock = 0
	geometry.PageSizeBytes = 0
	gotGeometry, err := device.Geometry()
	if err != nil || gotGeometry != (Geometry{2, 2, 2}) {
		t.Fatalf("Geometry after caller mutation = %+v, %v", gotGeometry, err)
	}
	want := Snapshot{
		Pages:       [][]string{{"ffff", "ffff"}, {"ffff", "ffff"}},
		EraseCounts: []uint64{0, 0},
		Programmed:  [][]bool{{false, false}, {false, false}},
	}
	got, err := device.Snapshot()
	if err != nil || !reflect.DeepEqual(got, want) {
		t.Fatalf("initial Snapshot() = %#v, %v; want %#v", got, err, want)
	}
}

func TestSentinelErrorsAndRefusalPrecedence(t *testing.T) {
	checks := []struct {
		name string
		err  error
		msg  string
	}{
		{"invalid argument", ErrInvalidArgument, "invalid API argument"},
		{"geometry", ErrGeometry, "geometry values are outside supported bounds"},
		{"block", ErrBlock, "block index out of range"},
		{"page", ErrPage, "page index out of range"},
		{"payload", ErrPayload, "page data must be bytes"},
		{"payload size", ErrPayloadSize, "page data must match page_size_bytes"},
		{"programmed", ErrProgrammed, "page has already been programmed since erase"},
		{"erase limit", ErrEraseLimit, "block erase limit reached"},
	}
	for _, check := range checks {
		t.Run(check.name, func(t *testing.T) {
			if check.err.Error() != check.msg {
				t.Fatalf("message = %q; want %q", check.err.Error(), check.msg)
			}
		})
	}
	var nilDevice *Device
	if _, err := nilDevice.Read(-1, -1); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("nil Read precedence error = %v", err)
	}
	if _, err := (&Device{}).Geometry(); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("zero Device Geometry error = %v", err)
	}
	if err := nilDevice.Program(-1, -1, nil); !errors.Is(err, ErrInvalidArgument) {
		t.Fatalf("nil Program precedence error = %v", err)
	}
	device := testDevice(t, Geometry{1, 1, 2}, nil)
	if _, err := device.Read(-1, -1); !errors.Is(err, ErrBlock) {
		t.Fatalf("block before page error = %v", err)
	}
	if _, err := device.Read(0, -1); !errors.Is(err, ErrPage) {
		t.Fatalf("page error = %v", err)
	}
	if err := device.Program(1, -1, nil); !errors.Is(err, ErrBlock) {
		t.Fatalf("block before payload error = %v", err)
	}
	if err := device.Program(0, -1, nil); !errors.Is(err, ErrPage) {
		t.Fatalf("page before payload error = %v", err)
	}
	if err := device.Program(0, 0, nil); !errors.Is(err, ErrPayload) {
		t.Fatalf("nil payload error = %v", err)
	}
	if err := device.Program(0, 0, []byte{}); !errors.Is(err, ErrPayloadSize) {
		t.Fatalf("non-nil empty payload error = %v", err)
	}
	if err := device.Program(0, 0, []byte{1, 2}); err != nil {
		t.Fatal(err)
	}
	if err := device.Program(0, 0, nil); !errors.Is(err, ErrPayload) {
		t.Fatalf("nil payload must precede marker error: %v", err)
	}
	if err := device.Program(0, 0, []byte{1}); !errors.Is(err, ErrPayloadSize) {
		t.Fatalf("wrong size must precede marker error: %v", err)
	}
	if err := device.Program(0, 0, []byte{3, 4}); !errors.Is(err, ErrProgrammed) {
		t.Fatalf("marker error = %v", err)
	}
	zeroLimit := uint64(0)
	limited := testDevice(t, Geometry{1, 1, 1}, &zeroLimit)
	if err := limited.EraseBlock(-1); !errors.Is(err, ErrBlock) {
		t.Fatalf("block before limit error = %v", err)
	}
	if err := limited.EraseBlock(0); !errors.Is(err, ErrEraseLimit) {
		t.Fatalf("zero erase budget error = %v", err)
	}
}

func TestInvalidDevicesRejectEveryMethodWithZeroResults(t *testing.T) {
	var devices []*Device
	devices = append(devices, nil, &Device{})
	for index, device := range devices {
		if geometry, err := device.Geometry(); geometry != (Geometry{}) || !errors.Is(err, ErrInvalidArgument) {
			t.Errorf("device %d Geometry = %+v, %v", index, geometry, err)
		}
		if data, err := device.Read(0, 0); data != nil || !errors.Is(err, ErrInvalidArgument) {
			t.Errorf("device %d Read = %v, %v", index, data, err)
		}
		if err := device.Program(0, 0, []byte{0}); !errors.Is(err, ErrInvalidArgument) {
			t.Errorf("device %d Program = %v", index, err)
		}
		if err := device.EraseBlock(0); !errors.Is(err, ErrInvalidArgument) {
			t.Errorf("device %d EraseBlock = %v", index, err)
		}
		if count, err := device.EraseCount(0); count != 0 || !errors.Is(err, ErrInvalidArgument) {
			t.Errorf("device %d EraseCount = %d, %v", index, count, err)
		}
		if marker, err := device.IsProgrammed(0, 0); marker || !errors.Is(err, ErrInvalidArgument) {
			t.Errorf("device %d IsProgrammed = %t, %v", index, marker, err)
		}
		if snapshot, err := device.Snapshot(); !reflect.DeepEqual(snapshot, Snapshot{}) || !errors.Is(err, ErrInvalidArgument) {
			t.Errorf("device %d Snapshot = %#v, %v", index, snapshot, err)
		}
	}
}

func TestProgramAndReadOwnTheirBuffers(t *testing.T) {
	device := testDevice(t, Geometry{1, 1, 3}, nil)
	input := []byte{0xF0, 0x0F, 0xAA}
	if err := device.Program(0, 0, input); err != nil {
		t.Fatal(err)
	}
	input[0] = 0
	readA, err := device.Read(0, 0)
	if err != nil {
		t.Fatal(err)
	}
	readA[1] = 0
	readB, err := device.Read(0, 0)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(readB, []byte{0xF0, 0x0F, 0xAA}) {
		t.Fatalf("device/read copy changed: %x", readB)
	}
	if &readA[0] == &readB[0] {
		t.Fatal("successive read results unexpectedly share backing memory")
	}
}

func TestConfigEraseLimitIsCopiedAndCountsAreIndependent(t *testing.T) {
	limit := uint64(1)
	device := testDevice(t, Geometry{2, 1, 1}, &limit)
	limit = 99
	if err := device.EraseBlock(0); err != nil {
		t.Fatal(err)
	}
	before, err := device.Snapshot()
	if err != nil {
		t.Fatal(err)
	}
	if err := device.EraseBlock(0); !errors.Is(err, ErrEraseLimit) {
		t.Fatalf("mutating source limit changed device budget: %v", err)
	}
	if err := device.EraseBlock(1); err != nil {
		t.Fatalf("block budgets should be independent: %v", err)
	}
	count0, err := device.EraseCount(0)
	if err != nil || count0 != 1 {
		t.Fatalf("erase count 0 = %d, %v", count0, err)
	}
	count1, err := device.EraseCount(1)
	if err != nil || count1 != 1 {
		t.Fatalf("erase count 1 = %d, %v", count1, err)
	}
	if !reflect.DeepEqual(before.EraseCounts, []uint64{1, 0}) {
		t.Fatalf("snapshot count copy = %v", before.EraseCounts)
	}
	device.eraseCounts[0] = maxSafeEraseCycles
	if err := device.EraseBlock(0); !errors.Is(err, ErrEraseLimit) {
		t.Fatalf("absolute erase-count ceiling error = %v", err)
	}
}

func TestSnapshotIsDetachedFromDeviceAndOtherSnapshots(t *testing.T) {
	device := testDevice(t, Geometry{2, 2, 1}, nil)
	if err := device.Program(0, 0, []byte{0}); err != nil {
		t.Fatal(err)
	}
	first, err := device.Snapshot()
	if err != nil {
		t.Fatal(err)
	}
	first.Pages[0][0] = "dead"
	first.Pages[0][1] = "beef"
	first.EraseCounts[0] = 99
	first.Programmed[0][0] = false
	second, err := device.Snapshot()
	if err != nil {
		t.Fatal(err)
	}
	if second.Pages[0][0] != "00" || second.Pages[0][1] != "ff" || second.EraseCounts[0] != 0 || !second.Programmed[0][0] {
		t.Fatalf("mutated snapshot leaked into device: %#v", second)
	}
	second.Pages[0][0] = "f00f"
	third, err := device.Snapshot()
	if err != nil || third.Pages[0][0] != "00" {
		t.Fatalf("snapshots share mutable storage: %#v, %v", third, err)
	}
}

func TestRejectedOperationsAreAtomic(t *testing.T) {
	limit := uint64(1)
	device := testDevice(t, Geometry{1, 2, 2}, &limit)
	if err := device.Program(0, 0, []byte{0, 1}); err != nil {
		t.Fatal(err)
	}
	if err := device.EraseBlock(0); err != nil {
		t.Fatal(err)
	}
	if err := device.Program(0, 0, []byte{2, 3}); err != nil {
		t.Fatal(err)
	}
	checkAtomic := func(name string, operation func() error) {
		t.Helper()
		before, err := device.Snapshot()
		if err != nil {
			t.Fatal(err)
		}
		if err := operation(); err == nil {
			t.Errorf("%s unexpectedly succeeded", name)
		}
		after, err := device.Snapshot()
		if err != nil || !reflect.DeepEqual(before, after) {
			t.Errorf("%s changed state: before %#v, after %#v, err %v", name, before, after, err)
		}
	}
	checkAtomic("second program", func() error { return device.Program(0, 0, []byte{1, 2}) })
	checkAtomic("wrong payload size", func() error { return device.Program(0, 1, []byte{1}) })
	checkAtomic("erase after budget", func() error { return device.EraseBlock(0) })
}

type modelDevice struct {
	pages      [][][]byte
	programmed [][]bool
	erases     []uint64
	limit      uint64
}

func newModel(blocks, pages, pageBytes int, limit uint64) *modelDevice {
	model := &modelDevice{
		pages:      make([][][]byte, blocks),
		programmed: make([][]bool, blocks),
		erases:     make([]uint64, blocks),
		limit:      limit,
	}
	for block := 0; block < blocks; block++ {
		model.pages[block] = make([][]byte, pages)
		model.programmed[block] = make([]bool, pages)
		for page := 0; page < pages; page++ {
			model.pages[block][page] = bytes.Repeat([]byte{0xFF}, pageBytes)
		}
	}
	return model
}

func (model *modelDevice) expectedSnapshot() Snapshot {
	state := Snapshot{
		Pages:       make([][]string, len(model.pages)),
		EraseCounts: append([]uint64(nil), model.erases...),
		Programmed:  make([][]bool, len(model.pages)),
	}
	for block := range model.pages {
		state.Pages[block] = make([]string, len(model.pages[block]))
		state.Programmed[block] = append([]bool(nil), model.programmed[block]...)
		for page := range model.pages[block] {
			state.Pages[block][page] = hex.EncodeToString(model.pages[block][page])
		}
	}
	return state
}

func compareModelState(t *testing.T, device *Device, model *modelDevice) {
	t.Helper()
	got, err := device.Snapshot()
	if err != nil {
		t.Fatal(err)
	}
	want := model.expectedSnapshot()
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("device/model state mismatch\n got: %#v\nwant: %#v", got, want)
	}
}

func TestSeededPropertyOperationsAgainstIndependentModel(t *testing.T) {
	rng := rand.New(rand.NewSource(0xD801))
	for trace := 0; trace < 8; trace++ {
		const eraseLimit = uint64(200)
		device := testDevice(t, Geometry{2, 2, 4}, ptrUint64(eraseLimit))
		model := newModel(2, 2, 4, eraseLimit)
		for step := 0; step < 600; step++ {
			block := rng.Intn(2)
			page := rng.Intn(2)
			switch rng.Intn(3) {
			case 0:
				got, err := device.Read(block, page)
				if err != nil || !bytes.Equal(got, model.pages[block][page]) {
					t.Fatalf("trace %d step %d read = %x, %v; want %x", trace, step, got, err, model.pages[block][page])
				}
			case 1:
				payload := make([]byte, 4)
				_, _ = rng.Read(payload)
				err := device.Program(block, page, payload)
				if model.programmed[block][page] {
					if !errors.Is(err, ErrProgrammed) {
						t.Fatalf("trace %d step %d program error = %v; want ErrProgrammed", trace, step, err)
					}
				} else {
					if err != nil {
						t.Fatalf("trace %d step %d program error = %v", trace, step, err)
					}
					model.pages[block][page] = append([]byte(nil), payload...)
					model.programmed[block][page] = true
				}
			case 2:
				err := device.EraseBlock(block)
				if model.erases[block] >= model.limit {
					if !errors.Is(err, ErrEraseLimit) {
						t.Fatalf("trace %d step %d erase error = %v; want ErrEraseLimit", trace, step, err)
					}
				} else {
					if err != nil {
						t.Fatalf("trace %d step %d erase error = %v", trace, step, err)
					}
					model.erases[block]++
					for page := range model.pages[block] {
						model.pages[block][page] = []byte{0xFF, 0xFF, 0xFF, 0xFF}
						model.programmed[block][page] = false
					}
				}
			}
			compareModelState(t, device, model)
		}
	}
}

func TestExhaustiveBoundedHistories(t *testing.T) {
	const historyLength = 6
	for encoded := 0; encoded < 1<<(2*historyLength); encoded++ {
		device := testDevice(t, Geometry{1, 1, 1}, ptrUint64(1))
		model := newModel(1, 1, 1, 1)
		sequence := encoded
		for step := 0; step < historyLength; step++ {
			op := sequence & 3
			sequence >>= 2
			switch op {
			case 0, 1:
				value := byte(0xFF)
				if op == 1 {
					value = 0
				}
				err := device.Program(0, 0, []byte{value})
				if model.programmed[0][0] {
					if !errors.Is(err, ErrProgrammed) {
						t.Fatalf("history %d step %d program = %v", encoded, step, err)
					}
				} else {
					if err != nil {
						t.Fatalf("history %d step %d program = %v", encoded, step, err)
					}
					model.pages[0][0][0] = value
					model.programmed[0][0] = true
				}
			case 2:
				err := device.EraseBlock(0)
				if model.erases[0] == model.limit {
					if !errors.Is(err, ErrEraseLimit) {
						t.Fatalf("history %d step %d erase = %v", encoded, step, err)
					}
				} else {
					if err != nil {
						t.Fatalf("history %d step %d erase = %v", encoded, step, err)
					}
					model.erases[0]++
					model.pages[0][0][0] = 0xFF
					model.programmed[0][0] = false
				}
			case 3:
				if err := device.Program(0, 1, []byte{0}); !errors.Is(err, ErrPage) {
					t.Fatalf("history %d step %d invalid page = %v", encoded, step, err)
				}
			}
			compareModelState(t, device, model)
		}
	}
}

func TestIndependentDevicesCanBeUsedInParallel(t *testing.T) {
	const workers = 12
	var wait sync.WaitGroup
	for worker := 0; worker < workers; worker++ {
		wait.Add(1)
		go func(value byte) {
			defer wait.Done()
			device, err := New(Config{Geometry: Geometry{1, 1, 1}})
			if err != nil {
				t.Errorf("New: %v", err)
				return
			}
			for turn := 0; turn < 40; turn++ {
				if err := device.Program(0, 0, []byte{value}); err != nil {
					t.Errorf("Program on independent device: %v", err)
					return
				}
				read, err := device.Read(0, 0)
				if err != nil || len(read) != 1 || read[0] != value {
					t.Errorf("Read = %x, %v", read, err)
					return
				}
				if err := device.EraseBlock(0); err != nil {
					t.Errorf("EraseBlock: %v", err)
					return
				}
				read, err = device.Read(0, 0)
				if err != nil || len(read) != 1 || read[0] != 0xFF {
					t.Errorf("Read after erase = %x, %v", read, err)
					return
				}
			}
		}(byte(worker))
	}
	wait.Wait()
}

func TestAbsoluteEraseCeilingAndDeviceQueries(t *testing.T) {
	device := testDevice(t, Geometry{1, 1, 1}, nil)
	device.eraseCounts[0] = maxSafeEraseCycles - 1
	if err := device.Program(0, 0, []byte{0}); err != nil {
		t.Fatal(err)
	}
	if err := device.EraseBlock(0); err != nil {
		t.Fatalf("last safe erase should succeed: %v", err)
	}
	if count, err := device.EraseCount(0); err != nil || count != maxSafeEraseCycles {
		t.Fatalf("counter after last safe erase = %d, %v", count, err)
	}
	if marker, err := device.IsProgrammed(0, 0); err != nil || marker {
		t.Fatalf("marker after last safe erase = %v, %v", marker, err)
	}
	before, err := device.Snapshot()
	if err != nil {
		t.Fatal(err)
	}
	if err := device.EraseBlock(0); !errors.Is(err, ErrEraseLimit) {
		t.Fatalf("absolute safe erase ceiling error = %v", err)
	}
	after, err := device.Snapshot()
	if err != nil || !reflect.DeepEqual(before, after) {
		t.Fatalf("refused absolute-ceiling erase changed state: before %#v after %#v err %v", before, after, err)
	}
	if _, err := device.EraseCount(-1); !errors.Is(err, ErrBlock) {
		t.Fatalf("EraseCount invalid block = %v", err)
	}
	if programmed, err := device.IsProgrammed(0, 0); err != nil || programmed {
		t.Fatalf("initial marker = %v, %v", programmed, err)
	}
	if _, err := device.IsProgrammed(0, 1); !errors.Is(err, ErrPage) {
		t.Fatalf("IsProgrammed invalid page = %v", err)
	}
}

func ptrUint64(value uint64) *uint64 { return &value }

func TestNoCallerVisibleMutationOnAddressErrors(t *testing.T) {
	device := testDevice(t, Geometry{1, 1, 1}, nil)
	if result, err := device.Read(1, 0); !errors.Is(err, ErrBlock) || result != nil {
		t.Fatalf("invalid read = %x, %v", result, err)
	}
	if result, err := device.Read(0, 1); !errors.Is(err, ErrPage) || result != nil {
		t.Fatalf("invalid page read = %x, %v", result, err)
	}
	if err := device.Program(0, 0, []byte{0}); err != nil {
		t.Fatal(err)
	}
	if err := device.Program(0, 0, []byte{0, 0}); !errors.Is(err, ErrPayloadSize) {
		t.Fatalf("wrong payload size on marked page = %v", err)
	}
}
