package storagewalrecord

import (
	"bytes"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"hash/crc32"
	"math/rand"
	"testing"

	storagerecordframe "github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame"
)

var castagnoli = crc32.MakeTable(crc32.Castagnoli)

func independentFrame(payload []byte) []byte {
	frame := make([]byte, len(payload)+16)
	copy(frame, "DBRF")
	frame[4] = 1
	binary.LittleEndian.PutUint32(frame[8:12], uint32(len(payload)))
	copy(frame[12:], payload)
	binary.LittleEndian.PutUint32(frame[len(frame)-4:], crc32.Checksum(frame[:len(frame)-4], castagnoli))
	return frame
}

func independentEncode(record Record) []byte {
	payload := make([]byte, fixedPayloadBytes+len(record.Image))
	payload[0] = byte(record.Kind)
	binary.LittleEndian.PutUint64(payload[4:12], record.LSN)
	binary.LittleEndian.PutUint64(payload[12:20], record.TxID)
	binary.LittleEndian.PutUint64(payload[20:28], uint64(record.PageID))
	copy(payload[fixedPayloadBytes:], record.Image)
	return independentFrame(payload)
}

func isZeroRecord(record Record) bool {
	return record.Kind == 0 && record.LSN == 0 && record.TxID == 0 && record.PageID == 0 && record.Image == nil
}

func validPayload(kind byte, lsn, txID, pageID uint64, image []byte) []byte {
	payload := make([]byte, fixedPayloadBytes+len(image))
	payload[0] = kind
	binary.LittleEndian.PutUint64(payload[4:12], lsn)
	binary.LittleEndian.PutUint64(payload[12:20], txID)
	binary.LittleEndian.PutUint64(payload[20:28], pageID)
	copy(payload[fixedPayloadBytes:], image)
	return payload
}

func TestExactPortableVectors(t *testing.T) {
	vectors := []struct {
		name   string
		record Record
		golden string
	}{
		{"begin", Record{Kind: BeginTransaction, LSN: 1, TxID: 2}, "44425246010000001c00000001000000010000000000000002000000000000000000000000000000c98ac8ea"},
		{"write-abc", Record{Kind: WritePageImage, LSN: 7, TxID: 99, PageID: 2, Image: []byte("abc")}, "44425246010000001f00000002000000070000000000000063000000000000000200000000000000616263e6115ef7"},
		{"commit-maximum-identifiers", Record{Kind: CommitTransaction, LSN: maxIdentifier, TxID: maxIdentifier}, "44425246010000001c00000003000000ffffffffffff1f00ffffffffffff1f000000000000000000648a1766"},
	}
	for _, vector := range vectors {
		t.Run(vector.name, func(t *testing.T) {
			got, err := Encode(vector.record)
			if err != nil {
				t.Fatal(err)
			}
			want, err := hex.DecodeString(vector.golden)
			if err != nil {
				t.Fatal(err)
			}
			if !bytes.Equal(got, want) || !bytes.Equal(got, independentEncode(vector.record)) {
				t.Fatalf("frame %x, want %x", got, want)
			}
			decoded, err := Decode(got)
			if err != nil || decoded.Kind != vector.record.Kind || decoded.LSN != vector.record.LSN || decoded.TxID != vector.record.TxID || decoded.PageID != vector.record.PageID || !bytes.Equal(decoded.Image, vector.record.Image) {
				t.Fatalf("decode %#v, %v", decoded, err)
			}
			if vector.record.Kind != WritePageImage && decoded.Image != nil {
				t.Fatal("begin/commit image must be nil")
			}
		})
	}
}

func TestEncodeValidationOrder(t *testing.T) {
	cases := []struct {
		name   string
		record Record
		want   error
	}{
		{"kind-before-lsn", Record{Kind: 9, LSN: 0, TxID: 0, PageID: -1, Image: []byte{1}}, ErrKind},
		{"lsn-before-txid", Record{Kind: BeginTransaction, LSN: 0, TxID: 0, PageID: -1, Image: []byte{1}}, ErrLSN},
		{"txid-before-page-id", Record{Kind: BeginTransaction, LSN: 1, TxID: 0, PageID: -1, Image: []byte{1}}, ErrTxID},
		{"page-id-before-image", Record{Kind: BeginTransaction, LSN: 1, TxID: 1, PageID: -1, Image: []byte{1}}, ErrPageID},
		{"begin-page-before-image", Record{Kind: BeginTransaction, LSN: 1, TxID: 1, PageID: 1, Image: []byte{1}}, ErrPageID},
		{"begin-image", Record{Kind: BeginTransaction, LSN: 1, TxID: 1, Image: []byte{1}}, ErrImageLength},
		{"commit-image", Record{Kind: CommitTransaction, LSN: 1, TxID: 1, Image: []byte{1}}, ErrImageLength},
		{"write-empty-image", Record{Kind: WritePageImage, LSN: 1, TxID: 1}, ErrImageLength},
		{"lsn-upper", Record{Kind: BeginTransaction, LSN: maxIdentifier + 1, TxID: 1}, ErrLSN},
		{"txid-upper", Record{Kind: BeginTransaction, LSN: 1, TxID: maxIdentifier + 1}, ErrTxID},
		{"page-id-upper", Record{Kind: WritePageImage, LSN: 1, TxID: 1, PageID: int64(maxIdentifier + 1), Image: []byte{1}}, ErrPageID},
		{"negative-page", Record{Kind: WritePageImage, LSN: 1, TxID: 1, PageID: -1, Image: []byte{1}}, ErrPageID},
		{"image-too-large", Record{Kind: WritePageImage, LSN: 1, TxID: 1, Image: make([]byte, maxImageBytes+1)}, ErrImageLength},
	}
	for _, test := range cases {
		t.Run(test.name, func(t *testing.T) {
			frame, err := Encode(test.record)
			if frame != nil || !errors.Is(err, test.want) {
				t.Fatalf("Encode = (%v,%v), want nil,%v", frame, err, test.want)
			}
		})
	}
}

func TestDecodeFrameErrorsPassThrough(t *testing.T) {
	valid := independentFrame(validPayload(byte(BeginTransaction), 1, 1, 0, nil))
	alter := func(index int, value byte) []byte {
		frame := append([]byte(nil), valid...)
		frame[index] = value
		return frame
	}
	checksum := append([]byte(nil), valid...)
	checksum[len(checksum)-1] ^= 1
	for _, test := range []struct {
		name  string
		frame []byte
		want  error
	}{
		{"size", nil, storagerecordframe.ErrFrameSize},
		{"magic", alter(0, 'X'), storagerecordframe.ErrMagic},
		{"version", alter(4, 2), storagerecordframe.ErrVersion},
		{"reserved", alter(5, 1), storagerecordframe.ErrReserved},
		{"payload-length", alter(8, 27), storagerecordframe.ErrPayloadLength},
		{"huge-payload-length", func() []byte {
			f := append([]byte(nil), valid...)
			for i := 8; i < 12; i++ {
				f[i] = 0xff
			}
			return f
		}(), storagerecordframe.ErrPayloadLength},
		{"checksum", checksum, storagerecordframe.ErrChecksum},
	} {
		t.Run(test.name, func(t *testing.T) {
			record, err := Decode(test.frame)
			if !isZeroRecord(record) || !errors.Is(err, test.want) {
				t.Fatalf("Decode=(%#v,%v), want zero,%v", record, err, test.want)
			}
		})
	}
}

func TestDecodePayloadValidationOrderAndZeroRecord(t *testing.T) {
	for length := 0; length < fixedPayloadBytes; length++ {
		frame, err := storagerecordframe.Encode(make([]byte, length))
		if err != nil {
			t.Fatal(err)
		}
		record, err := Decode(frame)
		if !isZeroRecord(record) || !errors.Is(err, ErrHeaderSize) {
			t.Fatalf("payload length %d decoded as %#v, %v", length, record, err)
		}
	}
	cases := []struct {
		name    string
		payload []byte
		want    error
	}{
		{"header-before-kind", []byte{0}, ErrHeaderSize},
		{"kind-before-reserved", func() []byte { p := validPayload(9, 0, 0, 1, []byte{1}); p[1] = 1; return p }(), ErrKind},
		{"reserved-before-lsn", func() []byte { p := validPayload(byte(BeginTransaction), 0, 0, 1, []byte{1}); p[1] = 1; return p }(), ErrReserved},
		{"lsn-before-txid", validPayload(byte(BeginTransaction), 0, 0, 1, []byte{1}), ErrLSN},
		{"txid-before-page", validPayload(byte(BeginTransaction), 1, 0, 1, []byte{1}), ErrTxID},
		{"page-before-image", validPayload(byte(BeginTransaction), 1, 1, 1, []byte{1}), ErrPageID},
		{"begin-image", validPayload(byte(BeginTransaction), 1, 1, 0, []byte{1}), ErrImageLength},
		{"commit-image", validPayload(byte(CommitTransaction), 1, 1, 0, []byte{1}), ErrImageLength},
		{"write-empty", validPayload(byte(WritePageImage), 1, 1, 0, nil), ErrImageLength},
		{"lsn-upper", validPayload(byte(BeginTransaction), maxIdentifier+1, 1, 0, nil), ErrLSN},
		{"txid-upper", validPayload(byte(BeginTransaction), 1, maxIdentifier+1, 0, nil), ErrTxID},
		{"page-upper", validPayload(byte(WritePageImage), 1, 1, maxIdentifier+1, []byte{1}), ErrPageID},
	}
	for _, test := range cases {
		t.Run(test.name, func(t *testing.T) {
			frame, err := storagerecordframe.Encode(test.payload)
			if err != nil {
				t.Fatal(err)
			}
			record, err := Decode(frame)
			if !isZeroRecord(record) || !errors.Is(err, test.want) {
				t.Fatalf("Decode=(%#v,%v), want zero,%v", record, err, test.want)
			}
		})
	}
}

func TestEncodeDecodeBoundsAndOwnership(t *testing.T) {
	records := []Record{
		{Kind: BeginTransaction, LSN: 1, TxID: 1},
		{Kind: WritePageImage, LSN: maxIdentifier, TxID: maxIdentifier, PageID: int64(maxIdentifier), Image: []byte{0xff}},
		{Kind: WritePageImage, LSN: 9, TxID: 7, PageID: 5, Image: make([]byte, maxImageBytes)},
		{Kind: CommitTransaction, LSN: 2, TxID: 4},
	}
	for _, original := range records {
		before := append([]byte(nil), original.Image...)
		frame, err := Encode(original)
		if err != nil {
			t.Fatal(err)
		}
		if !bytes.Equal(frame, independentEncode(original)) {
			t.Fatal("frame differs from independent encoder")
		}
		if !bytes.Equal(original.Image, before) {
			t.Fatal("Encode changed image input")
		}
		if original.Kind == WritePageImage {
			original.Image[0] ^= 0xff
			if frame[12+fixedPayloadBytes] != before[0] {
				t.Fatal("Encode frame aliases input image")
			}
			original.Image[0] ^= 0xff
		}
		frameBefore := append([]byte(nil), frame...)
		decoded, err := Decode(frame)
		if err != nil {
			t.Fatal(err)
		}
		if !bytes.Equal(frame, frameBefore) {
			t.Fatal("Decode changed frame input")
		}
		if decoded.Kind != original.Kind || decoded.LSN != original.LSN || decoded.TxID != original.TxID || decoded.PageID != original.PageID || !bytes.Equal(decoded.Image, before) {
			t.Fatalf("decoded record differs: %#v", decoded)
		}
		if original.Kind == WritePageImage {
			frame[12+fixedPayloadBytes] ^= 0xff
			if decoded.Image[0] != before[0] {
				t.Fatal("decoded image aliases frame")
			}
		}
	}
}

func TestChangedHeaderAndImageBytesRefuse(t *testing.T) {
	original := Record{Kind: WritePageImage, LSN: 22, TxID: 35, PageID: 9, Image: []byte("payload bytes")}
	frame, err := Encode(original)
	if err != nil {
		t.Fatal(err)
	}
	for _, index := range []int{12, 16, 20, 28, 40, len(frame) - 1} {
		changed := append([]byte(nil), frame...)
		changed[index] ^= 1
		record, err := Decode(changed)
		if err == nil || !isZeroRecord(record) {
			t.Fatalf("accepted changed byte %d: %#v", index, record)
		}
	}
}

func TestSeededIndependentRecordVectors(t *testing.T) {
	rng := rand.New(rand.NewSource(0xDB12A))
	for i := 0; i < 8192; i++ {
		kind := Kind(1 + rng.Intn(3))
		record := Record{Kind: kind, LSN: uint64(rng.Intn(100000) + 1), TxID: uint64(rng.Intn(100000) + 1)}
		if kind == WritePageImage {
			record.PageID = int64(rng.Intn(1 << 20))
			record.Image = make([]byte, rng.Intn(1025)+1)
			_, _ = rng.Read(record.Image)
		}
		frame, err := Encode(record)
		if err != nil {
			t.Fatalf("seed %d: %v", i, err)
		}
		if !bytes.Equal(frame, independentEncode(record)) {
			t.Fatalf("seed %d differs", i)
		}
		decoded, err := Decode(frame)
		if err != nil || decoded.Kind != record.Kind || decoded.LSN != record.LSN || decoded.TxID != record.TxID || decoded.PageID != record.PageID || !bytes.Equal(decoded.Image, record.Image) {
			t.Fatalf("seed %d round trip %#v %v", i, decoded, err)
		}
	}
}

func rawRecordInput(kind byte, lsn, txID, pageID uint64, image []byte) []byte {
	raw := make([]byte, 25+len(image))
	raw[0] = kind
	binary.LittleEndian.PutUint64(raw[1:9], lsn)
	binary.LittleEndian.PutUint64(raw[9:17], txID)
	binary.LittleEndian.PutUint64(raw[17:25], pageID)
	copy(raw[25:], image)
	return raw
}

func FuzzWALRecordRoundTrip(f *testing.F) {
	f.Add(rawRecordInput(byte(BeginTransaction), 1, 1, 0, nil))
	f.Add(rawRecordInput(byte(WritePageImage), 2, 3, 9, []byte{0x41, 0xff}))
	f.Add(rawRecordInput(byte(CommitTransaction), maxIdentifier, maxIdentifier, 0, nil))
	f.Fuzz(func(t *testing.T, raw []byte) {
		if len(raw) > 4096 {
			t.Skip()
		}
		var record Record
		if len(raw) > 0 {
			record.Kind = Kind(raw[0] % 5)
		}
		if len(raw) >= 9 {
			record.LSN = binary.LittleEndian.Uint64(raw[1:9]) & maxIdentifier
		}
		if len(raw) >= 17 {
			record.TxID = binary.LittleEndian.Uint64(raw[9:17]) & maxIdentifier
		}
		if len(raw) >= 25 {
			record.PageID = int64(binary.LittleEndian.Uint64(raw[17:25]) & uint64(maxIdentifier))
		}
		if record.Kind == WritePageImage && len(raw) > 25 {
			record.Image = append([]byte(nil), raw[25:]...)
		}
		if err := validateExpected(record); err != nil {
			frame, got := Encode(record)
			if frame != nil || !errors.Is(got, err) {
				t.Fatalf("Encode refusal got (%v,%v), want %v", frame, got, err)
			}
			return
		}
		frame, err := Encode(record)
		if err != nil {
			t.Fatal(err)
		}
		if !bytes.Equal(frame, independentEncode(record)) {
			t.Fatal("fuzz frame differs from standard-library oracle")
		}
		decoded, err := Decode(frame)
		if err != nil || decoded.Kind != record.Kind || decoded.LSN != record.LSN || decoded.TxID != record.TxID || decoded.PageID != record.PageID || !bytes.Equal(decoded.Image, record.Image) {
			t.Fatalf("fuzz round trip: %#v %v", decoded, err)
		}
	})
}

func validateExpected(record Record) error {
	if !validKind(record.Kind) {
		return ErrKind
	}
	if record.LSN == 0 || record.LSN > maxIdentifier {
		return ErrLSN
	}
	if record.TxID == 0 || record.TxID > maxIdentifier {
		return ErrTxID
	}
	if record.PageID < 0 || uint64(record.PageID) > maxIdentifier {
		return ErrPageID
	}
	if record.Kind != WritePageImage && record.PageID != 0 {
		return ErrPageID
	}
	if record.Kind == WritePageImage {
		if len(record.Image) < 1 || len(record.Image) > maxImageBytes {
			return ErrImageLength
		}
	} else if len(record.Image) != 0 {
		return ErrImageLength
	}
	return nil
}
