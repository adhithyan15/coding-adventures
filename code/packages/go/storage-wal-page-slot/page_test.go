package storagewalpageslot

import (
	"bytes"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"hash/crc32"
	"math/rand"
	"testing"

	storagerecordframe "github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
)

var castagnoli = crc32.MakeTable(crc32.Castagnoli)

func zeroRecord(record storagewalrecord.Record) bool {
	return record.Kind == 0 && record.LSN == 0 && record.TxID == 0 && record.PageID == 0 && record.Image == nil
}

func recordFrame(record storagewalrecord.Record) []byte {
	frame, err := storagewalrecord.Encode(record)
	if err != nil {
		panic(err)
	}
	return frame
}

func framePayload(payload []byte) []byte {
	frame, err := storagerecordframe.Encode(payload)
	if err != nil {
		panic(err)
	}
	return frame
}

func pageWithFrame(width int, frame []byte) []byte {
	page := make([]byte, width)
	binary.LittleEndian.PutUint32(page[:lengthBytes], uint32(len(frame)))
	copy(page[lengthBytes:], frame)
	return page
}

func TestExactPortablePageVectors(t *testing.T) {
	vectors := []struct {
		name   string
		width  int64
		record storagewalrecord.Record
		golden string
	}{
		{"vacancy", 48, storagewalrecord.Record{}, "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"},
		{"begin", 48, storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 2}, "2c00000044425246010000001c00000001000000010000000000000002000000000000000000000000000000c98ac8ea"},
		{"redo-eight", 56, storagewalrecord.Record{Kind: storagewalrecord.WritePageImage, LSN: 2, TxID: 3, PageID: 7, Image: []byte{1, 2, 3, 4, 5, 6, 7, 8}}, "34000000444252460100000024000000020000000200000000000000030000000000000007000000000000000102030405060708c4391724"},
		{"commit", 48, storagewalrecord.Record{Kind: storagewalrecord.CommitTransaction, LSN: 4, TxID: 5}, "2c00000044425246010000001c000000030000000400000000000000050000000000000000000000000000001c035b26"},
	}
	for _, vector := range vectors {
		t.Run(vector.name, func(t *testing.T) {
			want, err := hex.DecodeString(vector.golden)
			if err != nil {
				t.Fatal(err)
			}
			var page []byte
			if vector.name == "vacancy" {
				page = make([]byte, int(vector.width))
			} else {
				page, err = EncodePage(vector.width, vector.record)
				if err != nil {
					t.Fatal(err)
				}
			}
			if !bytes.Equal(page, want) {
				t.Fatalf("page %x, want %x", page, want)
			}
			record, present, err := DecodePage(page)
			if err != nil {
				t.Fatal(err)
			}
			if vector.name == "vacancy" {
				if present || !zeroRecord(record) {
					t.Fatalf("vacancy = (%#v,%v)", record, present)
				}
				return
			}
			if !present || record.Kind != vector.record.Kind || record.LSN != vector.record.LSN || record.TxID != vector.record.TxID || record.PageID != vector.record.PageID || !bytes.Equal(record.Image, vector.record.Image) {
				t.Fatalf("decode = (%#v,%v), want %#v", record, present, vector.record)
			}
		})
	}
}

func TestMaximumPageWidthAndDetachedEncoding(t *testing.T) {
	record := storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 1}
	page, err := EncodePage(MaxPageBytes, record)
	if err != nil {
		t.Fatal(err)
	}
	if len(page) != MaxPageBytes {
		t.Fatalf("page length %d, want %d", len(page), MaxPageBytes)
	}
	decoded, present, err := DecodePage(page)
	if err != nil || !present || decoded.Kind != record.Kind || decoded.LSN != record.LSN || decoded.TxID != record.TxID {
		t.Fatalf("max page decode = %#v, %v, %v", decoded, present, err)
	}
}

func TestPageSizeAndEncodeErrorOrder(t *testing.T) {
	invalid := storagewalrecord.Record{Kind: 99}
	for _, size := range []int64{-1, 0, 47, 65_537, 1 << 62} {
		if page, err := EncodePage(size, invalid); page != nil || !errors.Is(err, ErrPageSize) {
			t.Fatalf("EncodePage size %d = (%v,%v)", size, page, err)
		}
	}
	for _, page := range [][]byte{make([]byte, 47), make([]byte, 65_537)} {
		record, present, err := DecodePage(page)
		if !zeroRecord(record) || present || !errors.Is(err, ErrPageSize) {
			t.Fatalf("DecodePage invalid size = (%#v,%v,%v)", record, present, err)
		}
	}
	if page, err := EncodePage(48, invalid); page != nil || !errors.Is(err, storagewalrecord.ErrKind) {
		t.Fatalf("record error must precede fit: (%v,%v)", page, err)
	}
	tooWide := storagewalrecord.Record{Kind: storagewalrecord.WritePageImage, LSN: 1, TxID: 1, Image: []byte{1, 2, 3, 4, 5}}
	if page, err := EncodePage(48, tooWide); page != nil || !errors.Is(err, ErrSlotFrameLength) {
		t.Fatalf("frame fit = (%v,%v)", page, err)
	}
	if page, err := EncodePage(48, storagewalrecord.Record{Kind: storagewalrecord.WritePageImage, LSN: 0, TxID: 0, Image: make([]byte, MaxPageBytes)}); page != nil || !errors.Is(err, storagewalrecord.ErrLSN) {
		t.Fatalf("record refusal must precede fit: (%v,%v)", page, err)
	}
}

func TestDecodeVacancyFrameAndPaddingOrder(t *testing.T) {
	zero := make([]byte, 56)
	if record, present, err := DecodePage(zero); err != nil || present || !zeroRecord(record) {
		t.Fatalf("zero page = %#v,%v,%v", record, present, err)
	}
	for _, index := range []int{4, 17, 55} {
		page := append([]byte(nil), zero...)
		page[index] = 1
		record, present, err := DecodePage(page)
		if !zeroRecord(record) || present || !errors.Is(err, ErrVacantPageContents) {
			t.Fatalf("vacancy byte %d = %#v,%v,%v", index, record, present, err)
		}
	}
	for _, length := range []uint32{1, 43, 53} {
		page := make([]byte, 56)
		binary.LittleEndian.PutUint32(page[:4], length)
		record, present, err := DecodePage(page)
		if !zeroRecord(record) || present || !errors.Is(err, ErrSlotFrameLength) {
			t.Fatalf("length %d = %#v,%v,%v", length, record, present, err)
		}
	}
	page := make([]byte, 56)
	binary.LittleEndian.PutUint32(page[:4], 1000)
	if record, present, err := DecodePage(page); !zeroRecord(record) || present || !errors.Is(err, ErrSlotFrameLength) {
		t.Fatalf("oversized length = %#v,%v,%v", record, present, err)
	}

	badFrame := make([]byte, 44)
	page = pageWithFrame(56, badFrame)
	page[55] = 1
	if record, present, err := DecodePage(page); !zeroRecord(record) || present || !errors.Is(err, storagerecordframe.ErrMagic) {
		t.Fatalf("frame error must precede padding: %#v,%v,%v", record, present, err)
	}
	validFrame := recordFrame(storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 1})
	page = pageWithFrame(56, validFrame)
	page[len(page)-1] = 1
	if record, present, err := DecodePage(page); !zeroRecord(record) || present || !errors.Is(err, ErrPadding) {
		t.Fatalf("padding = %#v,%v,%v", record, present, err)
	}
	badPayload := make([]byte, 28)
	badPayload[0] = 99
	page = pageWithFrame(56, framePayload(badPayload))
	page[len(page)-1] = 1
	if record, present, err := DecodePage(page); !zeroRecord(record) || present || !errors.Is(err, storagewalrecord.ErrKind) {
		t.Fatalf("record error must precede padding: %#v,%v,%v", record, present, err)
	}
}

func TestDecodePreservesWalAndFrameErrors(t *testing.T) {
	validPayload := make([]byte, 28)
	validPayload[0] = byte(storagewalrecord.BeginTransaction)
	binary.LittleEndian.PutUint64(validPayload[4:12], 1)
	binary.LittleEndian.PutUint64(validPayload[12:20], 1)
	cases := []struct {
		name   string
		mutate func([]byte)
		want   error
	}{
		{"frame-magic", func(frame []byte) { frame[0] = 'X' }, storagerecordframe.ErrMagic},
		{"frame-version", func(frame []byte) { frame[4] = 2 }, storagerecordframe.ErrVersion},
		{"frame-reserved", func(frame []byte) { frame[5] = 1 }, storagerecordframe.ErrReserved},
		{"frame-length", func(frame []byte) { frame[8] = 27 }, storagerecordframe.ErrPayloadLength},
		{"frame-checksum", func(frame []byte) { frame[len(frame)-1] ^= 1 }, storagerecordframe.ErrChecksum},
		{"wal-kind", func(frame []byte) { frame[12] = 99; refreshFrameCRC(frame) }, storagewalrecord.ErrKind},
		{"wal-reserved", func(frame []byte) { frame[13] = 1; refreshFrameCRC(frame) }, storagewalrecord.ErrReserved},
	}
	for _, test := range cases {
		t.Run(test.name, func(t *testing.T) {
			frame := framePayload(validPayload)
			test.mutate(frame)
			page := pageWithFrame(64, frame)
			record, present, err := DecodePage(page)
			if !zeroRecord(record) || present || !errors.Is(err, test.want) {
				t.Fatalf("DecodePage=(%#v,%v,%v), want %v", record, present, err, test.want)
			}
		})
	}
}

func refreshFrameCRC(frame []byte) {
	binary.LittleEndian.PutUint32(frame[len(frame)-4:], crc32.Checksum(frame[:len(frame)-4], castagnoli))
}

func TestEveryBytePrefixFromVacancy(t *testing.T) {
	records := []storagewalrecord.Record{
		{Kind: storagewalrecord.BeginTransaction, LSN: 1, TxID: 1},
		{Kind: storagewalrecord.WritePageImage, LSN: 3, TxID: 4, PageID: 5, Image: []byte{0, 1, 2, 3, 4, 5, 6, 7}},
		{Kind: storagewalrecord.CommitTransaction, LSN: 8, TxID: 9},
	}
	widths := []int64{48, 56, 64, 256}
	checked, unchangedBytes := 0, 0
	for _, record := range records {
		for _, width := range widths {
			page, err := EncodePage(width, record)
			if err != nil {
				if int64(len(record.Image)) > width-48 {
					continue
				}
				t.Fatal(err)
			}
			for prefix := 0; prefix <= len(page); prefix++ {
				state := make([]byte, len(page))
				copy(state, page[:prefix])
				complete := bytes.Equal(state, page)
				decoded, present, err := DecodePage(state)
				if complete {
					if err != nil || !present || decoded.Kind != record.Kind || decoded.LSN != record.LSN || decoded.TxID != record.TxID || decoded.PageID != record.PageID || !bytes.Equal(decoded.Image, record.Image) {
						t.Fatalf("complete prefix %d/%d = %#v,%v,%v", prefix, len(page), decoded, present, err)
					}
				} else if err == nil && present {
					t.Fatalf("partial prefix %d/%d was admitted", prefix, len(page))
				} else if !complete && !zeroRecord(decoded) {
					t.Fatalf("partial prefix returned record %#v", decoded)
				}
				if prefix > 0 && page[prefix-1] == 0 {
					unchangedBytes++
				}
				checked++
			}
		}
	}
	if checked < 600 || unchangedBytes < 50 {
		t.Fatalf("prefix observations=%d unchanged cut bytes=%d", checked, unchangedBytes)
	}
}

func TestInputAndResultOwnership(t *testing.T) {
	record := storagewalrecord.Record{Kind: storagewalrecord.WritePageImage, LSN: 1, TxID: 2, PageID: 3, Image: []byte{9, 8, 7, 6, 5, 4, 3, 2}}
	original := append([]byte(nil), record.Image...)
	page, err := EncodePage(64, record)
	if err != nil {
		t.Fatal(err)
	}
	record.Image[0] ^= 0xff
	if page[lengthBytes+12+28] != original[0] {
		t.Fatal("encoded page aliases record image")
	}
	decoded, present, err := DecodePage(page)
	if err != nil || !present {
		t.Fatalf("decode %v %v", present, err)
	}
	page[lengthBytes+12+28] ^= 0xff
	if decoded.Image[0] != original[0] {
		t.Fatal("decoded image aliases page input")
	}
}

func TestSeededPageHistories(t *testing.T) {
	rng := rand.New(rand.NewSource(0xDB12B1))
	for i := 0; i < 20_000; i++ {
		image := make([]byte, rng.Intn(32)+1)
		_, _ = rng.Read(image)
		record := storagewalrecord.Record{Kind: storagewalrecord.WritePageImage, LSN: uint64(i + 3), TxID: uint64(rng.Intn(1000) + 1), PageID: int64(rng.Intn(1 << 20)), Image: image}
		width := int64(rng.Intn(128) + 48)
		page, err := EncodePage(width, record)
		if width < int64(len(recordFrame(record)))+lengthBytes {
			if page != nil || !errors.Is(err, ErrSlotFrameLength) {
				t.Fatalf("short slot %d: (%v,%v)", width, page, err)
			}
			continue
		}
		if err != nil {
			t.Fatal(err)
		}
		decoded, present, err := DecodePage(page)
		if err != nil || !present || decoded.Kind != record.Kind || decoded.LSN != record.LSN || decoded.TxID != record.TxID || decoded.PageID != record.PageID || !bytes.Equal(decoded.Image, record.Image) {
			t.Fatalf("history %d: %#v, %v", i, decoded, err)
		}
	}
}

func FuzzWALPageSlot(f *testing.F) {
	f.Add([]byte{0, 1, 2, 3, 4, 5, 6, 7})
	f.Add([]byte("record-page-slot"))
	f.Add([]byte{1})
	f.Fuzz(func(t *testing.T, data []byte) {
		if len(data) > 4096 {
			t.Skip()
		}
		if len(data) == 0 {
			return
		}
		record := storagewalrecord.Record{Kind: storagewalrecord.WritePageImage, LSN: 1, TxID: 2, PageID: 3, Image: data}
		page, err := EncodePage(int64(len(data)+48), record)
		if err != nil {
			t.Fatal(err)
		}
		decoded, present, err := DecodePage(page)
		if err != nil || !present || decoded.Kind != record.Kind || decoded.LSN != record.LSN || decoded.TxID != record.TxID || decoded.PageID != record.PageID || !bytes.Equal(decoded.Image, data) {
			t.Fatalf("round trip = %#v,%v,%v", decoded, present, err)
		}
		prefix := len(page) / 2
		partial := make([]byte, len(page))
		copy(partial, page[:prefix])
		got, present, err := DecodePage(partial)
		if err == nil && present {
			t.Fatalf("admitted non-identical prefix %d: %#v", prefix, got)
		}
	})
}
