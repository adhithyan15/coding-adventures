package storagecrc32c

import (
	"encoding/binary"
	"hash/crc32"
	"math/rand"
	"testing"
)

var castagnoliTable = crc32.MakeTable(crc32.Castagnoli)

func stdChecksum(data []byte) uint32 { return crc32.Checksum(data, castagnoliTable) }

func TestPortableVectors(t *testing.T) {
	vectors := []struct {
		name string
		data []byte
		want uint32
	}{
		{"empty-nil", nil, 0},
		{"empty-slice", []byte{}, 0},
		{"standard-check", []byte("123456789"), 0xe3069283},
		{"zero-byte", []byte{0}, 0x527d5351},
		{"all-ones", []byte{0xff, 0xff, 0xff, 0xff}, 0xffffffff},
	}
	for _, vector := range vectors {
		t.Run(vector.name, func(t *testing.T) {
			if got := Checksum(vector.data); got != vector.want {
				t.Fatalf("Checksum(%x) = %08x, want %08x", vector.data, got, vector.want)
			}
			if got := Checksum(vector.data); got != stdChecksum(vector.data) {
				t.Fatalf("standard library mismatch: %08x != %08x", got, stdChecksum(vector.data))
			}
		})
	}
}

func TestEveryOneAndTwoByteInput(t *testing.T) {
	buffer := make([]byte, 2)
	for value := 0; value < 256; value++ {
		buffer[0] = byte(value)
		if got, want := Checksum(buffer[:1]), stdChecksum(buffer[:1]); got != want {
			t.Fatalf("one byte %02x: got %08x want %08x", value, got, want)
		}
	}
	for value := 0; value < 1<<16; value++ {
		binary.LittleEndian.PutUint16(buffer, uint16(value))
		if got, want := Checksum(buffer), stdChecksum(buffer); got != want {
			t.Fatalf("two bytes %04x: got %08x want %08x", value, got, want)
		}
	}
}

func TestUpdateSeedsAndEveryShortSplit(t *testing.T) {
	vectors := [][]byte{
		[]byte(""),
		[]byte("a"),
		[]byte("123456789"),
		{0x00, 0x01, 0x7f, 0x80, 0xff, 0x42},
	}
	seeds := []uint32{0, 1, 0xffffffff, 0x12345678, 0x80000000, 0xdeadbeef}
	for _, data := range vectors {
		for _, seed := range seeds {
			want := crc32.Update(seed, castagnoliTable, data)
			for split := 0; split <= len(data); split++ {
				got := Update(Update(seed, data[:split]), data[split:])
				if got != want {
					t.Fatalf("seed %08x split %d data %x: got %08x want %08x", seed, split, data, got, want)
				}
			}
		}
	}
}

func TestOneBitChangesMatchIndependentOracle(t *testing.T) {
	base := [][]byte{[]byte("123456789"), {0x00, 0x01, 0x7f, 0x80, 0xff}}
	for _, data := range base {
		original := stdChecksum(data)
		for index := range data {
			for bit := uint(0); bit < 8; bit++ {
				changed := append([]byte(nil), data...)
				changed[index] ^= 1 << bit
				got := Checksum(changed)
				if got != stdChecksum(changed) || got == original {
					t.Fatalf("one-bit change at %d/%d: got %08x original %08x", index, bit, got, original)
				}
			}
		}
	}
}

func TestFixedGeneratedBuffersAndArbitrarySeeds(t *testing.T) {
	rng := rand.New(rand.NewSource(0xDB10A))
	for sample := 0; sample < 8192; sample++ {
		length := rng.Intn(1025)
		data := make([]byte, length)
		_, _ = rng.Read(data)
		seed := rng.Uint32()
		if got, want := Update(seed, data), crc32.Update(seed, castagnoliTable, data); got != want {
			t.Fatalf("sample %d len=%d seed=%08x: got %08x want %08x", sample, length, seed, got, want)
		}
		if got, want := Checksum(data), stdChecksum(data); got != want {
			t.Fatalf("checksum sample %d: got %08x want %08x", sample, got, want)
		}
	}
}

func TestNilAndEmptyContinuation(t *testing.T) {
	for _, seed := range []uint32{0, 1, 0xffffffff, 0x12345678} {
		if got := Update(seed, nil); got != seed {
			t.Fatalf("Update(%08x,nil) = %08x", seed, got)
		}
		if got := Update(seed, []byte{}); got != seed {
			t.Fatalf("Update(%08x,empty) = %08x", seed, got)
		}
	}
}

func TestZeroAllocations(t *testing.T) {
	data := []byte("stable input to the checksum")
	if allocations := testing.AllocsPerRun(1000, func() { _ = Checksum(data) }); allocations != 0 {
		t.Fatalf("Checksum allocated %v times", allocations)
	}
	if allocations := testing.AllocsPerRun(1000, func() { _ = Update(0xdeadbeef, data) }); allocations != 0 {
		t.Fatalf("Update allocated %v times", allocations)
	}
}

func FuzzCRC32CReference(f *testing.F) {
	f.Add(uint32(0), []byte("123456789"))
	f.Add(uint32(0xffffffff), []byte{})
	f.Add(uint32(0x12345678), []byte{0, 1, 127, 128, 255})
	f.Fuzz(func(t *testing.T, seed uint32, data []byte) {
		if len(data) > 4096 {
			t.Skip()
		}
		want := crc32.Update(seed, castagnoliTable, data)
		if got := Update(seed, data); got != want {
			t.Fatalf("Update(%08x,%x) = %08x, want %08x", seed, data, got, want)
		}
		if seed == 0 {
			if got := Checksum(data); got != want {
				t.Fatalf("Checksum(%x) = %08x, want %08x", data, got, want)
			}
		}
	})
}
