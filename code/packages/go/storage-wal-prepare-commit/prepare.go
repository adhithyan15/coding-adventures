// Package storagewalpreparecommit validates one proposed redo-only transaction.
package storagewalpreparecommit

import (
	"errors"

	storagepagelayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-page-layout"
	storagewaldatalayout "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-data-layout"
	storagewalrecord "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-record"
	storagewalredoplan "github.com/adhithyan15/coding-adventures/code/packages/go/storage-wal-redo-plan"
)

var ErrInsufficientSlots = errors.New("WAL reservation has insufficient transaction slots")

// PageImage is one full page update in the transaction's requested order.
type PageImage struct {
	PageID int64
	Image  []byte
}

// Prepare admits a complete begin/images/commit sequence against the captured
// prefix and returns detached proposed records in their append order.
func Prepare(log, data storagepagelayout.Layout, prefix []storagewalrecord.Record, txID uint64, images []PageImage) ([]storagewalrecord.Record, error) {
	if err := storagewaldatalayout.Validate(log, data); err != nil {
		return nil, err
	}

	remainingSlots := log.PageCount - int64(len(prefix))
	if remainingSlots < 2 || int64(len(images)) > remainingSlots-2 {
		return nil, ErrInsufficientSlots
	}

	firstLSN := uint64(len(prefix) + 1)
	begin := storagewalrecord.Record{Kind: storagewalrecord.BeginTransaction, LSN: firstLSN, TxID: txID}
	if _, err := storagewalrecord.Encode(begin); err != nil {
		return nil, err
	}

	proposed := make([]storagewalrecord.Record, len(images)+2)
	proposed[0] = begin
	for index, image := range images {
		proposed[index+1] = storagewalrecord.Record{
			Kind:   storagewalrecord.WritePageImage,
			LSN:    firstLSN + uint64(index+1),
			TxID:   txID,
			PageID: image.PageID,
			Image:  image.Image,
		}
	}
	proposed[len(proposed)-1] = storagewalrecord.Record{
		Kind: storagewalrecord.CommitTransaction,
		LSN:  firstLSN + uint64(len(images)+1),
		TxID: txID,
	}

	combined := make([]storagewalrecord.Record, 0, len(prefix)+len(proposed))
	combined = append(combined, prefix...)
	combined = append(combined, proposed...)
	plan, err := storagewalredoplan.Plan(data, combined)
	if err != nil {
		return nil, err
	}

	// The newly added transaction commits last, so its images occupy the tail
	// of the admitted plan. Reuse the planner's detached image copies.
	for index := range images {
		planIndex := len(plan) - len(images) + index
		proposed[index+1].Image = plan[planIndex].Record.Image
	}
	return proposed, nil
}
