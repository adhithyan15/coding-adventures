// Package storagewalscan scans one ordered reservation of fixed-width WAL pages.
package storagewalscan

import (
	"errors"

	storagewalpageslot "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-page-slot"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
)

// TailKind classifies the terminal WAL reservation position.
type TailKind uint8

const (
	// Full means every reserved page contains the next WAL record.
	Full TailKind = iota + 1
	// Vacant means the tail begins at an all-zero WAL slot.
	Vacant
	// Damaged means the tail begins at a malformed WAL slot.
	Damaged
)

// Scan reports the contiguous accepted record prefix and its tail.
type Scan struct {
	Records  []storagewalrecord.Record
	Tail     TailKind
	TailPage int64
	Cause    error
}

var (
	ErrPageWidthMismatch = errors.New("WAL scan pages have different widths")
	ErrSequence          = errors.New("WAL record LSN sequence is not contiguous")
	ErrPageAfterTail     = errors.New("occupied or damaged WAL page follows the tail")
)

// ScanPages scans pages in physical order and returns the accepted prefix and
// the first vacant or damaged tail position.
func ScanPages(pages [][]byte) (Scan, error) {
	if len(pages) == 0 {
		return Scan{Tail: Full, TailPage: 0}, nil
	}

	pageWidth := len(pages[0])
	var records []storagewalrecord.Record
	var tail TailKind
	var tailPage int64
	var cause error
	nextLSN := uint64(1)

	for index, page := range pages {
		if index > 0 && len(page) != pageWidth {
			return Scan{}, ErrPageWidthMismatch
		}

		record, present, err := storagewalpageslot.DecodePage(page)
		if index == 0 && err == storagewalpageslot.ErrPageSize {
			return Scan{}, err
		}
		if tail != 0 {
			if err != nil || present {
				return Scan{}, ErrPageAfterTail
			}
			continue
		}
		if err != nil {
			tail = Damaged
			tailPage = int64(index)
			cause = err
			continue
		}
		if !present {
			tail = Vacant
			tailPage = int64(index)
			continue
		}
		if record.LSN != nextLSN {
			return Scan{}, ErrSequence
		}
		records = append(records, record)
		nextLSN++
	}

	if tail == 0 {
		return Scan{Records: records, Tail: Full, TailPage: int64(len(pages))}, nil
	}
	return Scan{Records: records, Tail: tail, TailPage: tailPage, Cause: cause}, nil
}
