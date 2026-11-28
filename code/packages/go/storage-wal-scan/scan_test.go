package storagewalscan

import (
	"bytes"
	"encoding/binary"
	"errors"
	"testing"

	storagerecordframe "github.com/adhithyan15/coding-adventures/code/packages/go/storage-record-frame"
	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
)

const (
	choiceVacant = iota
	choiceDamaged
	choiceLSN1
	choiceLSN2
	choiceLSN3
	choiceLSN4
	choiceLSN7
)

func zeroScan(scan Scan) bool {
	return scan.Records == nil && scan.Tail == 0 && scan.TailPage == 0 && scan.Cause == nil
}

func pageForChoice(choice int) []byte {
	switch choice {
	case choiceVacant:
		return make([]byte, 56)
	case choiceDamaged:
		payload := make([]byte, 28)
		payload[0] = 99
		frame, err := storagerecordframe.Encode(payload)
		if err != nil {
			panic(err)
		}
		page := make([]byte, 56)
		binary.LittleEndian.PutUint32(page[:4], uint32(len(frame)))
		copy(page[4:], frame)
		return page
	default:
		lsn := uint64(0)
		switch choice {
		case choiceLSN1:
			lsn = 1
		case choiceLSN2:
			lsn = 2
		case choiceLSN3:
			lsn = 3
		case choiceLSN4:
			lsn = 4
		case choiceLSN7:
			lsn = 7
		default:
			panic("unknown model choice")
		}
		record := storagewalrecord.Record{Kind: storagewalrecord.WritePageImage, LSN: lsn, TxID: 5, PageID: int64(lsn), Image: []byte{byte(lsn), 0xa5}}
		page, err := storagewalpageslot.EncodePage(56, record)
		if err != nil {
			panic(err)
		}
		return page
	}
}

func recordForChoice(choice int) storagewalrecord.Record {
	lsn := map[int]uint64{choiceLSN1: 1, choiceLSN2: 2, choiceLSN3: 3, choiceLSN4: 4, choiceLSN7: 7}[choice]
	return storagewalrecord.Record{Kind: storagewalrecord.WritePageImage, LSN: lsn, TxID: 5, PageID: int64(lsn), Image: []byte{byte(lsn), 0xa5}}
}

func independentExpected(choices []int) (Scan, error) {
	var result Scan
	result.Tail = Full
	result.TailPage = int64(len(choices))
	nextLSN := uint64(1)
	tailSeen := false
	for index, choice := range choices {
		switch choice {
		case choiceVacant:
			if !tailSeen {
				tailSeen = true
				result.Tail = Vacant
				result.TailPage = int64(index)
			}
		case choiceDamaged:
			if tailSeen {
				return Scan{}, ErrPageAfterTail
			}
			tailSeen = true
			result.Tail = Damaged
			result.TailPage = int64(index)
			result.Cause = storagewalrecord.ErrKind
		default:
			if tailSeen {
				return Scan{}, ErrPageAfterTail
			}
			record := recordForChoice(choice)
			if record.LSN != nextLSN {
				return Scan{}, ErrSequence
			}
			result.Records = append(result.Records, record)
			nextLSN++
		}
	}
	return result, nil
}

func compareScan(t *testing.T, got, want Scan) {
	t.Helper()
	if got.Tail != want.Tail || got.TailPage != want.TailPage || got.Cause != want.Cause || len(got.Records) != len(want.Records) {
		t.Fatalf("scan = %#v, want %#v", got, want)
	}
	for index := range want.Records {
		left, right := got.Records[index], want.Records[index]
		if left.Kind != right.Kind || left.LSN != right.LSN || left.TxID != right.TxID || left.PageID != right.PageID || !bytes.Equal(left.Image, right.Image) {
			t.Fatalf("record %d = %#v, want %#v", index, left, right)
		}
	}
}

func pagesFor(choices []int) [][]byte {
	pages := make([][]byte, len(choices))
	for index, choice := range choices {
		pages[index] = pageForChoice(choice)
	}
	return pages
}

func TestEmptyAndFullReservations(t *testing.T) {
	scan, err := ScanPages(nil)
	if err != nil || scan.Tail != Full || scan.TailPage != 0 || scan.Cause != nil || len(scan.Records) != 0 {
		t.Fatalf("empty scan = %#v, %v", scan, err)
	}
	choices := []int{choiceLSN1, choiceLSN2, choiceLSN3, choiceLSN4}
	scan, err = ScanPages(pagesFor(choices))
	if err != nil {
		t.Fatal(err)
	}
	want, _ := independentExpected(choices)
	compareScan(t, scan, want)
}

func TestTailKindsAndVacantRemainders(t *testing.T) {
	for _, choices := range [][]int{
		{choiceVacant, choiceVacant, choiceVacant},
		{choiceLSN1, choiceLSN2, choiceVacant, choiceVacant},
		{choiceLSN1, choiceDamaged, choiceVacant, choiceVacant},
	} {
		scan, err := ScanPages(pagesFor(choices))
		if err != nil {
			t.Fatalf("choices %v: %v", choices, err)
		}
		want, _ := independentExpected(choices)
		compareScan(t, scan, want)
	}
	if scan, _ := ScanPages(pagesFor([]int{choiceLSN1, choiceDamaged, choiceVacant})); scan.Cause != storagewalrecord.ErrKind {
		t.Fatalf("damaged cause %v", scan.Cause)
	}
}

func TestRejectsMissingAndRepeatedRanks(t *testing.T) {
	for _, test := range []struct {
		name    string
		choices []int
	}{
		{"missing_rank", []int{choiceLSN1, choiceLSN3}},
		{"repeated_rank", []int{choiceLSN1, choiceLSN1}},
		{"wrong_first_rank", []int{choiceLSN2}},
	} {
		t.Run(test.name, func(t *testing.T) {
			scan, err := ScanPages(pagesFor(test.choices))
			if !zeroScan(scan) || !errors.Is(err, ErrSequence) {
				t.Fatalf("ScanPages = (%#v,%v), want zero,ErrSequence", scan, err)
			}
		})
	}
}

func TestFirstPageDamageIsTailUnlessItIsInvalidShape(t *testing.T) {
	damaged := make([]byte, 56)
	damaged[4] = 1 // Zero frame length with nonzero body.
	pages := [][]byte{damaged, make([]byte, 56), make([]byte, 56), make([]byte, 56), make([]byte, 56)}
	scan, err := ScanPages(pages)
	if err != nil || scan.Tail != Damaged || scan.TailPage != 0 || scan.Cause != storagewalpageslot.ErrVacantPageContents || len(scan.Records) != 0 {
		t.Fatalf("first damaged tail = (%#v,%v)", scan, err)
	}
	pages[1] = pageForChoice(choiceLSN1)
	if scan, err := ScanPages(pages); !zeroScan(scan) || !errors.Is(err, ErrPageAfterTail) {
		t.Fatalf("occupied after first damage = (%#v,%v)", scan, err)
	}
}

func TestTailRejectsLaterOccupiedOrDamaged(t *testing.T) {
	for _, test := range []struct {
		name    string
		choices []int
	}{
		{"occupied_after_vacant", []int{choiceLSN1, choiceVacant, choiceLSN2}},
		{"damaged_after_vacant", []int{choiceLSN1, choiceVacant, choiceDamaged}},
		{"occupied_after_damaged", []int{choiceLSN1, choiceDamaged, choiceLSN2}},
		{"damaged_after_damaged", []int{choiceLSN1, choiceDamaged, choiceDamaged}},
	} {
		t.Run(test.name, func(t *testing.T) {
			scan, err := ScanPages(pagesFor(test.choices))
			if !zeroScan(scan) || !errors.Is(err, ErrPageAfterTail) {
				t.Fatalf("ScanPages = (%#v,%v), want zero,ErrPageAfterTail", scan, err)
			}
		})
	}
}

func TestReservationShapeAndErrorPrecedence(t *testing.T) {
	invalidFirst := [][]byte{make([]byte, 47), make([]byte, 56)}
	if scan, err := ScanPages(invalidFirst); !zeroScan(scan) || !errors.Is(err, storagewalpageslot.ErrPageSize) {
		t.Fatalf("first row page-size error = (%#v,%v)", scan, err)
	}
	widthMismatch := [][]byte{make([]byte, 56), make([]byte, 55)}
	if scan, err := ScanPages(widthMismatch); !zeroScan(scan) || !errors.Is(err, ErrPageWidthMismatch) {
		t.Fatalf("width mismatch = (%#v,%v)", scan, err)
	}
	afterTailMismatch := [][]byte{make([]byte, 56), make([]byte, 55)}
	if scan, err := ScanPages(afterTailMismatch); !zeroScan(scan) || !errors.Is(err, ErrPageWidthMismatch) {
		t.Fatalf("width mismatch remains shape error after vacancy: (%#v,%v)", scan, err)
	}
	if scan, err := ScanPages([][]byte{pageForChoice(choiceLSN2)}); !zeroScan(scan) || !errors.Is(err, ErrSequence) {
		t.Fatalf("first occupied rank should be sequence error: (%#v,%v)", scan, err)
	}
}

func TestResultsOwnDecodedImages(t *testing.T) {
	pages := pagesFor([]int{choiceLSN1, choiceLSN2, choiceVacant})
	before := append([]byte(nil), pages[0]...)
	scan, err := ScanPages(pages)
	if err != nil || len(scan.Records) != 2 {
		t.Fatalf("ScanPages = %#v, %v", scan, err)
	}
	pages[0][4+12+28] ^= 0xff
	if !bytes.Equal(scan.Records[0].Image, []byte{1, 0xa5}) {
		t.Fatalf("scan image aliases input: %x", scan.Records[0].Image)
	}
	if bytes.Equal(pages[0], before) {
		t.Fatal("test failed to alter the source page")
	}
	scan.Records[0].Image[0] = 0xee
	again, err := ScanPages(pagesFor([]int{choiceLSN1, choiceLSN2, choiceVacant}))
	if err != nil || !bytes.Equal(again.Records[0].Image, []byte{1, 0xa5}) {
		t.Fatalf("returned record mutation escaped: %#v, %v", again, err)
	}
}

func TestExhaustiveIndependentFivePageModel(t *testing.T) {
	choices := []int{choiceVacant, choiceDamaged, choiceLSN1, choiceLSN2, choiceLSN3, choiceLSN4, choiceLSN7}
	cases := 0
	var visit func([]int, int)
	visit = func(history []int, remaining int) {
		if remaining == 0 {
			pages := pagesFor(history)
			want, wantErr := independentExpected(history)
			got, gotErr := ScanPages(pages)
			if wantErr != nil {
				if !zeroScan(got) || !errors.Is(gotErr, wantErr) {
					t.Fatalf("history %v = (%#v,%v), want zero,%v", history, got, gotErr, wantErr)
				}
			} else {
				if gotErr != nil {
					t.Fatalf("history %v: %v", history, gotErr)
				}
				compareScan(t, got, want)
			}
			cases++
			return
		}
		for _, choice := range choices {
			visit(append(history, choice), remaining-1)
		}
	}
	visit(nil, 5)
	if cases != 16_807 {
		t.Fatalf("executed %d five-page histories", cases)
	}
}

func FuzzWALScanModel(f *testing.F) {
	for _, seed := range [][]byte{
		{choiceVacant, choiceDamaged, choiceLSN1, choiceLSN2, choiceLSN3},
		{choiceLSN1, choiceLSN2, choiceVacant, choiceVacant, choiceVacant},
		{choiceLSN1, choiceVacant, choiceLSN2},
		{choiceLSN1, choiceLSN3},
		[]byte("000001"), // Sequence refusal occurs before the later width mismatch.
	} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, data []byte) {
		if len(data) > 64 {
			t.Skip()
		}
		count := len(data)
		if count > 5 {
			count = 5
		}
		choices := make([]int, count)
		for index := range choices {
			choices[index] = int(data[index] % 7)
		}
		pages := pagesFor(choices)
		if len(data) > 5 && len(pages) > 1 && data[5]&1 != 0 {
			pages[len(pages)-1] = pages[len(pages)-1][:len(pages[len(pages)-1])-1]
		}
		want, wantErr := independentExpected(choices)
		got, gotErr := ScanPages(pages)
		if len(data) > 5 && len(pages) > 1 && data[5]&1 != 0 {
			_, prefixErr := independentExpected(choices[:len(choices)-1])
			wantErr := prefixErr
			if wantErr == nil {
				wantErr = ErrPageWidthMismatch
			}
			if !zeroScan(got) || !errors.Is(gotErr, wantErr) {
				t.Fatalf("width-mutated history %v = (%#v,%v), want %v", choices, got, gotErr, wantErr)
			}
			return
		}
		if wantErr != nil {
			if !zeroScan(got) || !errors.Is(gotErr, wantErr) {
				t.Fatalf("history %v = (%#v,%v), want zero,%v", choices, got, gotErr, wantErr)
			}
			return
		}
		if gotErr != nil {
			t.Fatalf("history %v: %v", choices, gotErr)
		}
		compareScan(t, got, want)
	})
}
