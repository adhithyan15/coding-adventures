package hddwritecrashcuts

import (
	"bytes"
	"errors"
	"math/rand"
	"reflect"
	"testing"
)

func modelApply(before [][]byte, lba int64, data []byte, cut Cut) (Result, error) {
	if len(before) != SectorCount {
		return Result{}, ErrInvalidImageShape
	}
	for _, sector := range before {
		if len(sector) != BytesPerSector {
			return Result{}, ErrInvalidImageShape
		}
	}
	if lba < 0 || lba >= SectorCount {
		return Result{}, ErrInvalidLBA
	}
	if len(data) != BytesPerSector {
		return Result{}, ErrInvalidDataLength
	}
	result := Result{Image: make([][]byte, SectorCount)}
	for sector, source := range before {
		result.Image[sector] = make([]byte, BytesPerSector)
		for offset := 0; offset < BytesPerSector; offset++ {
			result.Image[sector][offset] = source[offset]
		}
	}
	target := result.Image[int(lba)]
	switch cut.Kind {
	case BeforeMediaWrite:
		if cut.WrittenPrefix != nil {
			return Result{}, ErrInvalidPrefix
		}
		result.PersistedPrefixBytes = 0
	case DuringMediaWrite:
		if cut.WrittenPrefix == nil || *cut.WrittenPrefix < 1 || *cut.WrittenPrefix > 7 {
			return Result{}, ErrInvalidPrefix
		}
		for offset := 0; offset < *cut.WrittenPrefix; offset++ {
			target[offset] = data[offset]
		}
		result.PersistedPrefixBytes = *cut.WrittenPrefix
	case AfterMediaWriteBeforeResponse, Acknowledged:
		if cut.WrittenPrefix != nil {
			return Result{}, ErrInvalidPrefix
		}
		for offset := range target {
			target[offset] = data[offset]
		}
		result.PersistedPrefixBytes = BytesPerSector
		if cut.Kind == Acknowledged {
			value := Receipt{LBA: lba}
			for offset := range value.Data {
				value.Data[offset] = data[offset]
			}
			result.Receipt = &value
		}
	default:
		return Result{}, ErrInvalidCut
	}
	return result, nil
}

func patternedImage(seed byte) [][]byte {
	image := make([][]byte, SectorCount)
	for sector := range image {
		image[sector] = make([]byte, BytesPerSector)
		for offset := range image[sector] {
			image[sector][offset] = seed + byte(sector*17+offset*29)
		}
	}
	return image
}

func cloneTestImage(image [][]byte) [][]byte {
	copyOfImage := make([][]byte, len(image))
	for index := range image {
		copyOfImage[index] = append([]byte(nil), image[index]...)
	}
	return copyOfImage
}

func during(prefix int) Cut { return Cut{Kind: DuringMediaWrite, WrittenPrefix: &prefix} }

func allCuts() []Cut {
	cuts := make([]Cut, 0, BytesPerSector+1)
	for prefix := 1; prefix <= BytesPerSector-1; prefix++ {
		cuts = append(cuts, during(prefix))
	}
	return append(cuts,
		Cut{Kind: BeforeMediaWrite},
		Cut{Kind: AfterMediaWriteBeforeResponse},
		Cut{Kind: Acknowledged},
	)
}

func TestExhaustiveCutSweep(t *testing.T) {
	operations := 0
	for lba := int64(0); lba < SectorCount; lba++ {
		for _, cut := range allCuts() {
			before := patternedImage(byte(lba * 13))
			data := []byte{0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7}
			if lba == 0 && cut.Kind == DuringMediaWrite && *cut.WrittenPrefix == 3 {
				copy(before[lba], data)
			}
			beforeCopy := cloneTestImage(before)
			want, wantErr := modelApply(before, lba, data, cut)
			got, gotErr := Apply(before, lba, data, cut)
			if gotErr != nil || wantErr != nil {
				t.Fatalf("lba=%d cut=%+v error=%v; model error=%v", lba, cut, gotErr, wantErr)
			}
			if !bytes.Equal(got.Image[int(lba)], want.Image[int(lba)]) {
				if cut.Kind == DuringMediaWrite {
					t.Fatalf("prefix splice mismatch: lba=%d cut=%+v got=%x want=%x", lba, cut, got.Image[int(lba)], want.Image[int(lba)])
				}
				t.Fatalf("target image mismatch: lba=%d cut=%+v", lba, cut)
			}
			if cut.Kind != Acknowledged && got.Receipt != nil {
				t.Fatalf("crash cut returned receipt: lba=%d cut=%+v", lba, cut)
			}
			if cut.Kind == Acknowledged && got.Receipt == nil {
				t.Fatalf("acknowledged cut omitted receipt: lba=%d", lba)
			}
			if !reflectReceiptEqual(got.Receipt, want.Receipt) {
				t.Fatalf("receipt mismatch: lba=%d cut=%+v got=%+v want=%+v", lba, cut, got.Receipt, want.Receipt)
			}
			if got.PersistedPrefixBytes != want.PersistedPrefixBytes {
				if lba == 0 && cut.Kind == DuringMediaWrite && *cut.WrittenPrefix == 3 {
					t.Fatalf("equal old and requested bytes lost cut evidence: got=%d want=%d", got.PersistedPrefixBytes, want.PersistedPrefixBytes)
				}
				t.Fatalf("prefix evidence mismatch: lba=%d cut=%+v got=%d want=%d", lba, cut, got.PersistedPrefixBytes, want.PersistedPrefixBytes)
			}
			for sector := range before {
				if int64(sector) != lba && !bytes.Equal(got.Image[sector], before[sector]) {
					t.Fatalf("sibling sector changed: lba=%d cut=%+v sibling=%d", lba, cut, sector)
				}
			}
			if !reflect.DeepEqual(before, beforeCopy) {
				t.Fatalf("Apply changed caller input for lba=%d cut=%+v", lba, cut)
			}
			if lba == 0 && cut.Kind == DuringMediaWrite && *cut.WrittenPrefix == 3 && got.PersistedPrefixBytes != 3 {
				t.Fatalf("equal old and requested bytes lost cut evidence: %+v", got)
			}
			operations++
		}
	}
	if operations != 320 {
		t.Fatalf("checked %d cut/LBA cases; want 320", operations)
	}
	t.Logf("independent byte-array oracle checked all %d LBA/cut outcomes", operations)
}

func TestOwnershipAndReceiptIsolation(t *testing.T) {
	before := patternedImage(11)
	inputBefore := cloneTestImage(before)
	data := []byte{1, 2, 3, 4, 5, 6, 7, 8}
	inputData := append([]byte(nil), data...)
	result, err := Apply(before, 7, data, Cut{Kind: Acknowledged})
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(before, inputBefore) || !bytes.Equal(data, inputData) {
		t.Fatal("successful call changed caller-owned image or requested data")
	}
	before[7][0] ^= 0xff
	data[0] ^= 0xff
	if !bytes.Equal(result.Image[7], inputData) || result.Receipt == nil || result.Receipt.Data[0] != inputData[0] {
		t.Fatal("result or receipt aliases caller storage")
	}
	result.Image[7][1] ^= 0xff
	if result.Receipt.Data[1] != inputData[1] {
		t.Fatal("receipt aliases result image")
	}
	result.Receipt.Data[2] ^= 0xff
	if result.Image[7][2] != inputData[2] {
		t.Fatal("result image aliases receipt")
	}
	row9Before := append([]byte(nil), result.Image[9]...)
	result.Image[8][0] ^= 0xff
	if !bytes.Equal(result.Image[9], row9Before) {
		t.Fatal("result image rows unexpectedly alias each other")
	}
}

func TestRefusalsAreOrderedAtomicAndReturnZeroResult(t *testing.T) {
	valid := patternedImage(4)
	data := []byte{9, 8, 7, 6, 5, 4, 3, 2}
	badShape := cloneTestImage(valid)
	badShape[4] = badShape[4][:7]
	var zero Result
	for _, test := range []struct {
		name  string
		image [][]byte
		lba   int64
		data  []byte
		cut   Cut
		want  error
	}{
		{"shape precedes address", badShape, -1, nil, Cut{Kind: CutKind("unknown")}, ErrInvalidImageShape},
		{"negative lba precedes data", valid, -1, nil, Cut{Kind: CutKind("unknown")}, ErrInvalidLBA},
		{"high lba precedes data", valid, 32, nil, Cut{Kind: Acknowledged}, ErrInvalidLBA},
		{"data precedes cut", valid, 0, data[:7], Cut{Kind: CutKind("unknown")}, ErrInvalidDataLength},
		{"unknown cut", valid, 0, data, Cut{Kind: CutKind("unknown")}, ErrInvalidCut},
		{"missing during prefix", valid, 0, data, Cut{Kind: DuringMediaWrite}, ErrInvalidPrefix},
		{"zero during prefix", valid, 0, data, during(0), ErrInvalidPrefix},
		{"eight during prefix", valid, 0, data, during(8), ErrInvalidPrefix},
		{"unexpected before prefix", valid, 0, data, Cut{Kind: BeforeMediaWrite, WrittenPrefix: intPointer(1)}, ErrInvalidPrefix},
		{"unexpected after media prefix", valid, 0, data, Cut{Kind: AfterMediaWriteBeforeResponse, WrittenPrefix: intPointer(0)}, ErrInvalidPrefix},
		{"unexpected after prefix", valid, 0, data, Cut{Kind: Acknowledged, WrittenPrefix: intPointer(0)}, ErrInvalidPrefix},
	} {
		t.Run(test.name, func(t *testing.T) {
			before := cloneTestImage(test.image)
			got, err := Apply(test.image, test.lba, test.data, test.cut)
			if !errors.Is(err, test.want) || !reflect.DeepEqual(got, zero) {
				t.Fatalf("result=%+v error=%v; want zero result and %v", got, err, test.want)
			}
			if !reflect.DeepEqual(test.image, before) {
				t.Fatal("refusal changed caller image")
			}
		})
	}
}

func intPointer(value int) *int { return &value }

func TestSeededCutHistoriesMatchIndependentModel(t *testing.T) {
	random := rand.New(rand.NewSource(0x4d04a1))
	image := patternedImage(0x22)
	operations := 0
	cuts := allCuts()
	for iteration := 0; iteration < 5_000; iteration++ {
		lba := int64(random.Intn(SectorCount))
		data := make([]byte, BytesPerSector)
		for index := range data {
			data[index] = byte(random.Intn(256))
		}
		cut := cuts[random.Intn(len(cuts))]
		before := cloneTestImage(image)
		want, err := modelApply(before, lba, data, cut)
		if err != nil {
			t.Fatal(err)
		}
		got, err := Apply(image, lba, data, cut)
		if err != nil || !reflect.DeepEqual(got, want) {
			t.Fatalf("seeded step %d lba=%d cut=%+v got=%+v error=%v want=%+v", iteration, lba, cut, got, err, want)
		}
		image = got.Image
		operations++
	}
	if operations != 5_000 {
		t.Fatalf("history operation count=%d; want 5000", operations)
	}
	t.Logf("fixed-seed model checked %d compositional writes", operations)
}

func FuzzWriteCutHistory(f *testing.F) {
	f.Add([]byte{0, 1, 9, 31, 2, 3, 4, 5, 6})
	f.Add([]byte{9, 8, 7, 6, 5, 4, 3, 2})
	f.Fuzz(func(t *testing.T, commands []byte) {
		if len(commands) > 64 {
			return
		}
		image := patternedImage(0x34)
		cuts := allCuts()
		for index, command := range commands {
			lba := int64(command) % SectorCount
			data := make([]byte, BytesPerSector)
			for offset := range data {
				data[offset] = command + byte(index*7+offset*13)
			}
			cut := cuts[int(command)%len(cuts)]
			want, err := modelApply(cloneTestImage(image), lba, data, cut)
			if err != nil {
				t.Fatal(err)
			}
			got, err := Apply(image, lba, data, cut)
			if err != nil || !reflect.DeepEqual(got, want) {
				t.Fatalf("history step %d mismatch: got=%+v error=%v want=%+v", index, got, err, want)
			}
			image = got.Image
		}
	})
}
