package storagerecordframe

import (
	"bytes"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"hash/crc32"
	"math/rand"
	"testing"

	storagecrc32c "github.com/adhithyan15/coding-adventures/code/packages/go/storage-crc32c"
)

var independentTable = crc32.MakeTable(crc32.Castagnoli)

func independentEncode(payload []byte) []byte {
	frame := make([]byte, len(payload)+16)
	copy(frame, "DBRF")
	frame[4] = 1
	binary.LittleEndian.PutUint32(frame[8:12], uint32(len(payload)))
	copy(frame[12:], payload)
	binary.LittleEndian.PutUint32(frame[len(frame)-4:], crc32.Checksum(frame[:len(frame)-4], independentTable))
	return frame
}

func refreshChecksum(frame []byte) {
	binary.LittleEndian.PutUint32(frame[len(frame)-4:], crc32.Checksum(frame[:len(frame)-4], independentTable))
}

func TestEncodeExactPortableVectors(t *testing.T) {
	vectors := []struct {
		name    string
		payload []byte
		golden  string
	}{
		{"empty", nil, "4442524601000000000000003a21fb5a"},
		{"abc", []byte("abc"), "444252460100000003000000616263d81f586e"},
		{"standard-check", []byte("123456789"), "44425246010000000900000031323334353637383981b01873"},
	}
	for _, vector := range vectors {
		t.Run(vector.name, func(t *testing.T) {
			got, err := Encode(vector.payload)
			if err != nil {
				t.Fatal(err)
			}
			want := independentEncode(vector.payload)
			if vector.golden != "" {
				want, err = hex.DecodeString(vector.golden)
				if err != nil {
					t.Fatal(err)
				}
			}
			if !bytes.Equal(got, want) {
				t.Fatalf("frame %x, want %x", got, want)
			}
			decoded, err := Decode(got)
			if err != nil || !bytes.Equal(decoded, vector.payload) {
				t.Fatalf("round trip got %x, %v", decoded, err)
			}
		})
	}
}

func TestEncodeDecodeBoundariesAndOwnership(t *testing.T) {
	for _, length := range []int{0, 1, 15, 16, 255, 4096, MaxPayloadBytes} {
		payload := make([]byte, length)
		for i := range payload {
			payload[i] = byte(i*37 + 11)
		}
		before := append([]byte(nil), payload...)
		frame, err := Encode(payload)
		if err != nil {
			t.Fatalf("length %d: %v", length, err)
		}
		if len(frame) != length+16 || !bytes.Equal(frame, independentEncode(payload)) {
			t.Fatalf("length %d encoded bytes differ", length)
		}
		if length == MaxPayloadBytes {
			sum := sha256.Sum256(frame)
			if digest := hex.EncodeToString(sum[:]); digest != "f0c29ae2477f2d69651153c796693216a94ca946112de76a49ee950ec8ed0557" {
				t.Fatalf("maximum frame digest %s", digest)
			}
		}
		if !bytes.Equal(payload, before) {
			t.Fatalf("Encode changed input at length %d", length)
		}
		if got := binary.LittleEndian.Uint32(frame[8:12]); got != uint32(length) {
			t.Fatalf("length field %d, want %d", got, length)
		}
		if got := binary.LittleEndian.Uint32(frame[len(frame)-4:]); got != crc32.Checksum(frame[:len(frame)-4], independentTable) {
			t.Fatalf("CRC is not little-endian header+payload CRC at length %d", length)
		}
		decoded, err := Decode(frame)
		if err != nil {
			t.Fatalf("Decode length %d: %v", length, err)
		}
		if !bytes.Equal(decoded, before) {
			t.Fatalf("decoded bytes differ at length %d", length)
		}
		if length == 0 && decoded != nil {
			t.Fatalf("empty payload must decode to nil, got %#v", decoded)
		}
		if length != 0 && len(decoded) != 0 {
			frame[12] ^= 0xff
			if decoded[0] != before[0] {
				t.Fatal("decoded payload aliases frame")
			}
		}
	}
	if got, err := Encode(make([]byte, MaxPayloadBytes+1)); got != nil || !errors.Is(err, ErrPayloadLength) {
		t.Fatalf("oversized Encode = (%v,%v)", got, err)
	}
}

func TestEverySingleBitChangeIsRefused(t *testing.T) {
	payload := []byte("single-bit corruption coverage")
	frame := independentEncode(payload)
	for index := range frame {
		for bit := uint(0); bit < 8; bit++ {
			changed := append([]byte(nil), frame...)
			changed[index] ^= 1 << bit
			if got, err := Decode(changed); err == nil || got != nil {
				t.Fatalf("accepted one-bit corruption at byte %d bit %d: %x", index, bit, got)
			}
		}
	}
}

func TestDecodeRefusalOrderAndNilResults(t *testing.T) {
	valid := independentEncode([]byte("payload"))
	cases := []struct {
		name  string
		frame []byte
		want  error
	}{
		{"too-short", valid[:15], ErrFrameSize},
		{"too-large", make([]byte, maxFrameBytes+1), ErrFrameSize},
		{"magic-before-version", func() []byte { f := append([]byte(nil), valid...); f[0] = 'X'; f[4] = 2; return f }(), ErrMagic},
		{"version-before-reserved", func() []byte { f := append([]byte(nil), valid...); f[4] = 2; f[5] = 1; return f }(), ErrVersion},
		{"reserved-before-length", func() []byte {
			f := append([]byte(nil), valid...)
			f[5] = 1
			binary.LittleEndian.PutUint32(f[8:12], 0xffffffff)
			return f
		}(), ErrReserved},
		{"length-before-checksum", func() []byte {
			f := append([]byte(nil), valid...)
			binary.LittleEndian.PutUint32(f[8:12], 0xffffffff)
			return f
		}(), ErrPayloadLength},
		{"checksum", func() []byte { f := append([]byte(nil), valid...); f[len(f)-1] ^= 1; return f }(), ErrChecksum},
		{"trailing-byte", append(append([]byte(nil), valid...), 0), ErrPayloadLength},
	}
	for _, test := range cases {
		t.Run(test.name, func(t *testing.T) {
			got, err := Decode(test.frame)
			if got != nil || !errors.Is(err, test.want) {
				t.Fatalf("Decode = (%x,%v), want nil,%v", got, err, test.want)
			}
		})
	}
}

func TestIndependentGeneratedFramesAndPayloads(t *testing.T) {
	rng := rand.New(rand.NewSource(0xDB10B))
	for sample := 0; sample < 8192; sample++ {
		length := rng.Intn(4097)
		payload := make([]byte, length)
		_, _ = rng.Read(payload)
		before := append([]byte(nil), payload...)
		frame, err := Encode(payload)
		if err != nil {
			t.Fatalf("sample %d: %v", sample, err)
		}
		if !bytes.Equal(frame, independentEncode(payload)) {
			t.Fatalf("sample %d frame differs", sample)
		}
		decoded, err := Decode(frame)
		if err != nil || !bytes.Equal(decoded, payload) {
			t.Fatalf("sample %d decode = (%d,%v)", sample, len(decoded), err)
		}
		if !bytes.Equal(payload, before) {
			t.Fatalf("sample %d input changed", sample)
		}
		if length != 0 {
			frame[12] ^= 1
			if decoded[0] != before[0] {
				t.Fatalf("sample %d decode aliases frame", sample)
			}
		}
	}
}

func TestDecodeAllShortTruncations(t *testing.T) {
	frame := independentEncode([]byte("every truncation"))
	for length := 0; length < len(frame); length++ {
		got, err := Decode(frame[:length])
		if got != nil || err == nil {
			t.Fatalf("accepted truncation %d/%d", length, len(frame))
		}
	}
}

func FuzzStorageRecordFrame(f *testing.F) {
	f.Add([]byte(""))
	f.Add([]byte("DBRF"))
	f.Add([]byte("123456789"))
	f.Add([]byte{0, 1, 127, 128, 255})
	f.Fuzz(func(t *testing.T, payload []byte) {
		if len(payload) > MaxPayloadBytes {
			t.Skip()
		}
		frame, err := Encode(payload)
		if err != nil {
			t.Fatal(err)
		}
		if !bytes.Equal(frame, independentEncode(payload)) {
			t.Fatal("encoded bytes differ from standard-library oracle")
		}
		decoded, err := Decode(frame)
		if err != nil || !bytes.Equal(decoded, payload) {
			t.Fatalf("round trip: len=%d err=%v", len(decoded), err)
		}
		if len(frame) > 1 {
			mutated := append([]byte(nil), frame...)
			mutated[len(mutated)-1] ^= 1
			if got, err := Decode(mutated); got != nil || !errors.Is(err, ErrChecksum) {
				t.Fatalf("trailer mutation = (%x,%v)", got, err)
			}
		}
	})
}

func TestCRCDependencyIsPortableAndPresent(t *testing.T) {
	if got := storagecrc32c.Checksum([]byte("123456789")); got != 0xe3069283 {
		t.Fatalf("admitted CRC check = %08x", got)
	}
}
