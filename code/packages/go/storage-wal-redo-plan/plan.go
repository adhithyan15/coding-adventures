// Package storagewalredoplan selects committed WAL page images for redo.
package storagewalredoplan

import (
	"errors"

	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
)

var (
	ErrTransactionExists   = errors.New("WAL transaction already began in this prefix")
	ErrTransactionInactive = errors.New("WAL transaction is not active")
	ErrRecordKind          = errors.New("WAL record kind is unknown")
	ErrImageWidth          = errors.New("WAL page image has the wrong width")
)

// RedoWrite pairs one committed page image with the commit that selected it.
type RedoWrite struct {
	Record    storagewalrecord.Record
	CommitLSN uint64
}

type transaction struct {
	active bool
	writes []storagewalrecord.Record
}

// Plan returns page-image records belonging to committed transactions. The
// supplied record sequence is already admitted, contiguous and stable.
func Plan(layout storagepagelayout.Layout, records []storagewalrecord.Record) ([]RedoWrite, error) {
	if err := layout.Validate(); err != nil {
		return nil, err
	}

	transactions := make(map[uint64]*transaction)
	var plan []RedoWrite
	for _, record := range records {
		switch record.Kind {
		case storagewalrecord.BeginTransaction:
			if _, exists := transactions[record.TxID]; exists {
				return nil, ErrTransactionExists
			}
			transactions[record.TxID] = &transaction{active: true}
		case storagewalrecord.WritePageImage:
			state := transactions[record.TxID]
			if state == nil || !state.active {
				return nil, ErrTransactionInactive
			}
			span, err := layout.Span(record.PageID)
			if err != nil {
				return nil, err
			}
			if len(record.Image) != int(span.BytesPerPage) {
				return nil, ErrImageWidth
			}
			state.writes = append(state.writes, record)
		case storagewalrecord.CommitTransaction:
			state := transactions[record.TxID]
			if state == nil || !state.active {
				return nil, ErrTransactionInactive
			}
			state.active = false
			for _, write := range state.writes {
				owned := write
				owned.Image = append([]byte(nil), write.Image...)
				plan = append(plan, RedoWrite{Record: owned, CommitLSN: record.LSN})
			}
			state.writes = nil
		default:
			return nil, ErrRecordKind
		}
	}

	return plan, nil
}
