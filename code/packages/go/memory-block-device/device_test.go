package memoryblockdevice

import (
	"bytes"
	"errors"
	"math"
	"math/rand"
	"testing"

	blockdevicecontract "github.com/adhithyan15/coding-adventures/code/packages/go/block-device-contract"
)

const maximumSafeBlockCount int64 = 9_007_199_254_740_991
const maximumBlockWidth int64 = 65_536

func TestConstructionAndZeroValueRefusals(t *testing.T) {
	for _, geometry := range []blockdevicecontract.Geometry{
		{}, {BlockCount: -1, BytesPerBlock: 1}, {BlockCount: 1, BytesPerBlock: 65_537},
	} {
		if device, err := New(geometry); device != nil || !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
			t.Fatalf("New(%+v)=(%v,%v), want nil, ErrInvalidGeometry", geometry, device, err)
		}
	}
	geometry := blockdevicecontract.Geometry{BlockCount: 0, BytesPerBlock: 0}
	var zero Memory
	var nilDevice *Memory
	for name, device := range map[string]*Memory{"zero": &zero, "nil": nilDevice} {
		t.Run(name, func(t *testing.T) {
			if got := device.Geometry(); got != geometry {
				t.Fatalf("Geometry()=%+v, want zero", got)
			}
			if _, err := device.ReadBlock(0); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
				t.Fatalf("ReadBlock error=%v, want ErrInvalidGeometry", err)
			}
			if err := device.WriteBlock(0, nil); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
				t.Fatalf("WriteBlock error=%v, want ErrInvalidGeometry", err)
			}
			if err := device.Flush(); !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
				t.Fatalf("Flush error=%v, want ErrInvalidGeometry", err)
			}
			if restarted, err := device.Restart(); restarted != nil || !errors.Is(err, blockdevicecontract.ErrInvalidGeometry) {
				t.Fatalf("Restart()=(%v,%v), want nil, ErrInvalidGeometry", restarted, err)
			}
		})
	}
}

func TestNewKeepsMaximumGeometrySparse(t *testing.T) {
	geometry := blockdevicecontract.Geometry{BlockCount: maximumSafeBlockCount, BytesPerBlock: maximumBlockWidth}
	device := mustNew(t, geometry)
	if len(device.durable) != 0 || len(device.pending) != 0 {
		t.Fatalf("New prepopulated sparse images: durable=%d pending=%d", len(device.durable), len(device.pending))
	}
}

func TestPendingVisibilityFlushAndRestart(t *testing.T) {
	device := mustNew(t, blockdevicecontract.Geometry{BlockCount: 3, BytesPerBlock: 2})
	wantZero := []byte{0, 0}
	assertRead(t, device, 1, wantZero)

	first := []byte{1, 2}
	if err := device.WriteBlock(1, first); err != nil {
		t.Fatal(err)
	}
	first[0] = 90
	assertRead(t, device, 1, []byte{1, 2})

	beforeFlush, err := device.Restart()
	if err != nil {
		t.Fatal(err)
	}
	assertRead(t, beforeFlush, 1, wantZero)
	assertRead(t, device, 1, []byte{1, 2})

	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	afterFlush, err := device.Restart()
	if err != nil {
		t.Fatal(err)
	}
	assertRead(t, afterFlush, 1, []byte{1, 2})

	second := []byte{3, 4}
	if err := device.WriteBlock(1, second); err != nil {
		t.Fatal(err)
	}
	second[1] = 99
	assertRead(t, device, 1, []byte{3, 4})
	assertRead(t, afterFlush, 1, []byte{1, 2})

	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	latest, err := device.Restart()
	if err != nil {
		t.Fatal(err)
	}
	assertRead(t, latest, 1, []byte{3, 4})
	assertRead(t, afterFlush, 1, []byte{1, 2})
}

func TestEmptyFlushAndRestartPreserveLineages(t *testing.T) {
	device := mustNew(t, blockdevicecontract.Geometry{BlockCount: 2, BytesPerBlock: 1})
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	initialRestart, err := device.Restart()
	if err != nil {
		t.Fatal(err)
	}
	if err := device.WriteBlock(0, []byte{7}); err != nil {
		t.Fatal(err)
	}
	pendingRestart, err := device.Restart()
	if err != nil {
		t.Fatal(err)
	}
	assertRead(t, pendingRestart, 0, []byte{0})
	if err := device.Flush(); err != nil {
		t.Fatal(err)
	}
	flushedRestart, err := device.Restart()
	if err != nil {
		t.Fatal(err)
	}
	assertRead(t, flushedRestart, 0, []byte{7})
	if err := device.WriteBlock(0, []byte{8}); err != nil {
		t.Fatal(err)
	}
	assertRead(t, device, 0, []byte{8})
	assertRead(t, flushedRestart, 0, []byte{7})
	assertRead(t, initialRestart, 0, []byte{0})
}

func TestReadBuffersAreDetachedAndValidHugeGeometryIsSparse(t *testing.T) {
	geometry := blockdevicecontract.Geometry{BlockCount: maximumSafeBlockCount, BytesPerBlock: 3}
	device := mustNew(t, geometry)
	if got := device.Geometry(); got != geometry {
		t.Fatalf("Geometry()=%+v, want %+v", got, geometry)
	}
	last := maximumSafeBlockCount - 1
	if err := device.WriteBlock(last, []byte{0xff, 0x00, 0x7f}); err != nil {
		t.Fatal(err)
	}
	firstRead, err := device.ReadBlock(last)
	if err != nil {
		t.Fatal(err)
	}
	firstRead[0] = 0
	secondRead, err := device.ReadBlock(last)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(secondRead, []byte{0xff, 0x00, 0x7f}) {
		t.Fatalf("mutating returned read changed device bytes: %v", secondRead)
	}
	if _, err := device.ReadBlock(maximumSafeBlockCount); !errors.Is(err, blockdevicecontract.ErrInvalidBlock) {
		t.Fatalf("upper-bound read error=%v, want ErrInvalidBlock", err)
	}
}

func TestRefusalOrderAndStateAtomicity(t *testing.T) {
	device := mustNew(t, blockdevicecontract.Geometry{BlockCount: 2, BytesPerBlock: 2})
	if err := device.WriteBlock(0, []byte{4, 5}); err != nil {
		t.Fatal(err)
	}
	cases := []struct {
		name  string
		block int64
		data  []byte
		want  error
	}{
		{"negative before data width", -1, nil, blockdevicecontract.ErrInvalidBlock},
		{"upper bound before data width", 2, []byte{1}, blockdevicecontract.ErrInvalidBlock},
		{"width after address", 0, []byte{1}, blockdevicecontract.ErrInvalidBlockData},
		{"maximum signed address", math.MaxInt64, nil, blockdevicecontract.ErrInvalidBlock},
		{"minimum signed address", math.MinInt64, nil, blockdevicecontract.ErrInvalidBlock},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			before := snapshot(t, device, 2)
			if err := device.WriteBlock(tc.block, tc.data); !errors.Is(err, tc.want) {
				t.Fatalf("WriteBlock error=%v, want %v", err, tc.want)
			}
			if tc.want == blockdevicecontract.ErrInvalidBlockData {
				if _, err := device.ReadBlock(tc.block); err != nil {
					t.Fatalf("read valid block unexpectedly failed: %v", err)
				}
			}
			if after := snapshot(t, device, 2); !equalImages(before, after) {
				t.Fatalf("refused write changed state: before=%v after=%v", before, after)
			}
		})
	}
	if err := device.WriteBlock(0, []byte{8, 9}); err != nil {
		t.Fatal(err)
	}
	assertRead(t, device, 0, []byte{8, 9})
}

func TestIndependentFiniteHistories(t *testing.T) {
	commands := []command{
		{kind: writeCommand, block: 0, data: []byte{1, 2}},
		{kind: writeCommand, block: 0, data: []byte{8, 9}},
		{kind: writeCommand, block: 1, data: []byte{3, 4}},
		{kind: flushCommand},
		{kind: restartCommand},
	}
	for encoded := 0; encoded < 5*5*5*5; encoded++ {
		device := mustNew(t, blockdevicecontract.Geometry{BlockCount: 2, BytesPerBlock: 2})
		model := newModel(blockdevicecontract.Geometry{BlockCount: 2, BytesPerBlock: 2})
		state := encoded
		for step := 0; step < 4; step++ {
			cmd := commands[state%len(commands)]
			state /= len(commands)
			applyAndCompare(t, &device, &model, cmd)
		}
	}
}

func TestSeededHistories(t *testing.T) {
	const seed = int64(0x6b06b2b1)
	const count = 65_536
	rng := rand.New(rand.NewSource(seed))
	geometry := blockdevicecontract.Geometry{BlockCount: 4, BytesPerBlock: 3}
	device := mustNew(t, geometry)
	model := newModel(geometry)
	for index := 0; index < count; index++ {
		cmd := command{kind: commandKind(rng.Intn(3)), block: int64(rng.Intn(4)), data: []byte{byte(rng.Intn(256)), byte(rng.Intn(256)), byte(rng.Intn(256))}}
		applyAndCompare(t, &device, &model, cmd)
	}
}

func FuzzMemoryHistories(f *testing.F) {
	f.Add([]byte{0, 0, 0, 10, 2, 0, 0, 20, 1, 2})
	f.Add([]byte{0, 3, 0, 1, 0xff, 0x80, 0x00, 2, 1, 0, 1})
	f.Add([]byte{2, 2, 1, 0, 9, 8, 7, 2, 2, 1, 0, 6, 5, 4})
	f.Fuzz(func(t *testing.T, encoded []byte) {
		if len(encoded) > 4096 {
			t.Skip()
		}
		geometry := blockdevicecontract.Geometry{BlockCount: 4, BytesPerBlock: 2}
		device := mustNew(t, geometry)
		model := newModel(geometry)
		for cursor := 0; cursor < len(encoded); {
			tag := encoded[cursor] % 3
			cursor++
			cmd := command{kind: commandKind(tag)}
			if tag == byte(writeCommand) {
				if cursor+3 > len(encoded) {
					break
				}
				cmd.block = int64(encoded[cursor] % 4)
				cmd.data = append([]byte(nil), encoded[cursor+1:cursor+3]...)
				cursor += 3
			}
			applyAndCompare(t, &device, &model, cmd)
		}
	})
}

type commandKind byte

const (
	writeCommand commandKind = iota
	flushCommand
	restartCommand
)

type command struct {
	kind  commandKind
	block int64
	data  []byte
}

type model struct {
	geometry blockdevicecontract.Geometry
	durable  map[int64][]byte
	pending  map[int64][]byte
}

func newModel(geometry blockdevicecontract.Geometry) model {
	return model{geometry: geometry, durable: map[int64][]byte{}, pending: map[int64][]byte{}}
}

func applyAndCompare(t *testing.T, device **Memory, expected *model, cmd command) {
	t.Helper()
	switch cmd.kind {
	case writeCommand:
		want := expected.write(cmd.block, cmd.data)
		got := (*device).WriteBlock(cmd.block, cmd.data)
		if !errors.Is(got, want) || (want == nil && got != nil) {
			t.Fatalf("WriteBlock(%d,%v) error=%v want=%v", cmd.block, cmd.data, got, want)
		}
	case flushCommand:
		want := expected.flush()
		got := (*device).Flush()
		if !errors.Is(got, want) || (want == nil && got != nil) {
			t.Fatalf("Flush() error=%v want=%v", got, want)
		}
	case restartCommand:
		var wantErr error
		if err := expected.geometry.Validate(); err != nil {
			wantErr = blockdevicecontract.ErrInvalidGeometry
		}
		next, gotErr := (*device).Restart()
		if !errors.Is(gotErr, wantErr) || (wantErr == nil && gotErr != nil) {
			t.Fatalf("Restart() error=%v want=%v", gotErr, wantErr)
		}
		if gotErr == nil {
			*device = next
			expected.pending = map[int64][]byte{}
			expected.durable = cloneImage(expected.durable)
		}
	default:
		t.Fatalf("unknown command %d", cmd.kind)
	}
	for block := int64(0); block < expected.geometry.BlockCount && block < 4; block++ {
		want := expected.read(block)
		got, err := (*device).ReadBlock(block)
		if err != nil {
			t.Fatalf("ReadBlock(%d): %v", block, err)
		}
		if !bytes.Equal(got, want) {
			t.Fatalf("state after %v block %d: got %v want %v", cmd, block, got, want)
		}
	}
}

func (m *model) write(block int64, data []byte) error {
	if err := m.geometry.CheckWrite(block, data); err != nil {
		return err
	}
	m.pending[block] = append([]byte(nil), data...)
	return nil
}

func (m *model) flush() error {
	if err := m.geometry.Validate(); err != nil {
		return blockdevicecontract.ErrInvalidGeometry
	}
	for block, data := range m.pending {
		m.durable[block] = append([]byte(nil), data...)
	}
	m.pending = map[int64][]byte{}
	return nil
}

func (m *model) read(block int64) []byte {
	if err := m.geometry.CheckBlock(block); err != nil {
		return nil
	}
	if value, exists := m.pending[block]; exists {
		return append([]byte(nil), value...)
	}
	if value, exists := m.durable[block]; exists {
		return append([]byte(nil), value...)
	}
	return make([]byte, int(m.geometry.BytesPerBlock))
}

func cloneImage(source map[int64][]byte) map[int64][]byte {
	clone := make(map[int64][]byte, len(source))
	for block, data := range source {
		clone[block] = append([]byte(nil), data...)
	}
	return clone
}

func mustNew(t *testing.T, geometry blockdevicecontract.Geometry) *Memory {
	t.Helper()
	device, err := New(geometry)
	if err != nil {
		t.Fatalf("New(%+v): %v", geometry, err)
	}
	return device
}

func assertRead(t *testing.T, device *Memory, block int64, want []byte) {
	t.Helper()
	got, err := device.ReadBlock(block)
	if err != nil {
		t.Fatalf("ReadBlock(%d): %v", block, err)
	}
	if !bytes.Equal(got, want) {
		t.Fatalf("ReadBlock(%d)=%v, want %v", block, got, want)
	}
}

func snapshot(t *testing.T, device *Memory, blocks int64) [][]byte {
	t.Helper()
	image := make([][]byte, blocks)
	for block := int64(0); block < blocks; block++ {
		image[block], _ = device.ReadBlock(block)
	}
	return image
}

func equalImages(left, right [][]byte) bool {
	if len(left) != len(right) {
		return false
	}
	for index := range left {
		if !bytes.Equal(left[index], right[index]) {
			return false
		}
	}
	return true
}
