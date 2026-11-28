package storagewalcheckpointrecord

import (
	"bytes"
	"encoding/hex"
	"errors"
	"fmt"
	"reflect"
	"testing"

	storagerecordframe "github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame"
)

const maxPortable uint64 = 9_007_199_254_740_991

func goldenRecord() Record { return Record{Phase: Resetting, Epoch: 1, PreviousLSN: 0} }

func framePayload(t *testing.T, payload []byte) []byte {
	t.Helper()
	frame, err := storagerecordframe.Encode(payload)
	if err != nil {
		t.Fatal(err)
	}
	return frame
}

func payloadFor(record Record) []byte {
	payload := make([]byte, 24)
	copy(payload, "DBCP")
	payload[4], payload[5] = 1, byte(record.Phase)
	for i := 0; i < 8; i++ {
		payload[8+i] = byte(record.Epoch >> (8 * i))
		payload[16+i] = byte(record.PreviousLSN >> (8 * i))
	}
	return payload
}

func TestIndependentGoldenFrameAndOwnership(t *testing.T) {
	want, err := hex.DecodeString("444252460100000018000000444243500101000001000000000000000000000000000000aeb4e057")
	if err != nil {
		t.Fatal(err)
	}
	first, err := Encode(goldenRecord())
	if err != nil || !bytes.Equal(first, want) || len(first) != FrameBytes {
		t.Fatalf("frame=%x err=%v", first, err)
	}
	second, err := Encode(goldenRecord())
	if err != nil || &first[0] == &second[0] {
		t.Fatal("encode did not return independent frames")
	}
	decoded, err := Decode(want)
	if err != nil || decoded != goldenRecord() {
		t.Fatalf("decoded=%+v err=%v", decoded, err)
	}
	first[0] ^= 0xff
	if !bytes.Equal(second, want) {
		t.Fatal("encoded frame aliases another result")
	}
}

func TestEncodePortableBoundsAndPrecedence(t *testing.T) {
	cases := []struct {
		name   string
		record Record
		want   error
	}{
		{"phase before epoch and rank", Record{Phase: 0, Epoch: 0, PreviousLSN: maxPortable + 1}, ErrPhase},
		{"epoch before rank", Record{Phase: Ready, Epoch: 0, PreviousLSN: maxPortable + 1}, ErrEpoch},
		{"rank ceiling", Record{Phase: Resetting, Epoch: 1, PreviousLSN: maxPortable + 1}, ErrPreviousLSN},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			frame, err := Encode(tc.record)
			if frame != nil || !errors.Is(err, tc.want) {
				t.Fatalf("Encode=(%x,%v), want %v", frame, err, tc.want)
			}
		})
	}
	for _, record := range []Record{{Phase: Resetting, Epoch: 1, PreviousLSN: 0}, {Phase: Ready, Epoch: maxPortable, PreviousLSN: maxPortable}} {
		frame, err := Encode(record)
		if err != nil {
			t.Fatalf("boundary rejected: %+v: %v", record, err)
		}
		got, err := Decode(frame)
		if err != nil || got != record {
			t.Fatalf("boundary round trip=(%+v,%v)", got, err)
		}
	}
}

func TestDecodeLowerFrameAndPayloadPrecedence(t *testing.T) {
	valid, _ := Encode(goldenRecord())
	badLength := append([]byte(nil), valid[:len(valid)-1]...)
	if got, err := Decode(badLength); !reflect.DeepEqual(got, Record{}) || !errors.Is(err, storagerecordframe.ErrPayloadLength) {
		t.Fatalf("short lower frame=(%+v,%v)", got, err)
	}
	if got, err := Decode(make([]byte, FrameBytes)); !reflect.DeepEqual(got, Record{}) || !errors.Is(err, storagerecordframe.ErrMagic) {
		t.Fatalf("lower magic=(%+v,%v)", got, err)
	}
	wrongSize, err := storagerecordframe.Encode(make([]byte, 23))
	if err != nil {
		t.Fatal(err)
	}
	if got, err := Decode(wrongSize); !reflect.DeepEqual(got, Record{}) || !errors.Is(err, ErrPayloadSize) {
		t.Fatalf("payload size=(%+v,%v)", got, err)
	}
}

func TestDecodePayloadStructuralAndFieldOrder(t *testing.T) {
	validPayload := payloadFor(goldenRecord())
	cases := []struct {
		name   string
		mutate func([]byte)
		want   error
	}{
		{"magic before version reserved fields", func(p []byte) { p[0] = 'X'; p[4] = 9; p[6] = 1 }, ErrMagic},
		{"version before reserved", func(p []byte) { p[4] = 9; p[6] = 1 }, ErrVersion},
		{"reserved", func(p []byte) { p[6] = 1 }, ErrReserved},
		{"phase before epoch and rank", func(p []byte) { p[5] = 9 }, ErrPhase},
		{"epoch before rank", func(p []byte) { p[8] = 0 }, ErrEpoch},
		{"previous rank", func(p []byte) {
			for i := 0; i < 8; i++ {
				p[16+i] = byte((maxPortable + 1) >> (8 * i))
			}
		}, ErrPreviousLSN},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			payload := append([]byte(nil), validPayload...)
			tc.mutate(payload)
			frame := framePayload(t, payload)
			got, err := Decode(frame)
			if !reflect.DeepEqual(got, Record{}) || !errors.Is(err, tc.want) {
				t.Fatalf("Decode=(%+v,%v), want %v", got, err, tc.want)
			}
		})
	}
}

func TestEverySingleBitMutationHasExactLowerRefusal(t *testing.T) {
	valid, err := Encode(Record{Phase: Ready, Epoch: 123, PreviousLSN: 17})
	if err != nil {
		t.Fatal(err)
	}
	for offset := range valid {
		for bit := uint(0); bit < 8; bit++ {
			t.Run(fmt.Sprintf("byte-%d-bit-%d", offset, bit), func(t *testing.T) {
				changed := append([]byte(nil), valid...)
				changed[offset] ^= 1 << bit
				got, err := Decode(changed)
				if !reflect.DeepEqual(got, Record{}) || err == nil {
					t.Fatalf("single bit accepted: %+v", got)
				}
				var want error
				switch {
				case offset < 4:
					want = storagerecordframe.ErrMagic
				case offset == 4:
					want = storagerecordframe.ErrVersion
				case offset < 8:
					want = storagerecordframe.ErrReserved
				case offset < 12:
					want = storagerecordframe.ErrPayloadLength
				default:
					want = storagerecordframe.ErrChecksum
				}
				if !errors.Is(err, want) {
					t.Fatalf("single bit error=%v want %v", err, want)
				}
			})
		}
	}
}

func FuzzDecodeFrameHistories(f *testing.F) {
	golden, _ := Encode(goldenRecord())
	f.Add(golden)
	f.Add([]byte{})
	f.Add(make([]byte, FrameBytes-1))
	f.Add(make([]byte, FrameBytes+1))
	f.Fuzz(func(t *testing.T, frame []byte) {
		if len(frame) > 128 {
			t.Skip()
		}
		record, err := Decode(frame)
		if err != nil {
			return
		}
		encoded, err := Encode(record)
		if err != nil || !bytes.Equal(encoded, frame) {
			t.Fatalf("accepted frame does not round-trip: record=%+v err=%v", record, err)
		}
	})
}

func ExampleDecode() {
	frame, _ := Encode(Record{Phase: Resetting, Epoch: 1})
	fmt.Printf("%d-byte framed checkpoint\n", len(frame))
	// Output: 40-byte framed checkpoint
}
