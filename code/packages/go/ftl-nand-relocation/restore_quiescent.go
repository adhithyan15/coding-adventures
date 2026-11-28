package ftlnandrelocation

import (
	"errors"

	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

var (
	// ErrInvalidMediaBlock reports a raw-media block with the wrong page count.
	ErrInvalidMediaBlock = errors.New("invalid restored NAND media block")
	// ErrIncompleteMediaBlock rejects a raw block whose erase was interrupted.
	ErrIncompleteMediaBlock = errors.New("restored NAND media contains an incomplete erase")
	// ErrMediaRoleMismatch reports marker bits inconsistent with the restored page map.
	ErrMediaRoleMismatch = errors.New("restored NAND marker does not match page-map role")
)

const maxRestoredPhysicalPages = int64(128)

// RestoreQuiescent binds a detached page map to an ordered raw NAND image.
// The caller keeps both inputs stable until the call returns. Success owns
// every copied byte slice and starts with a fresh process-local identity for
// each current logical owner, but no active evacuation lifecycle.
func RestoreQuiescent(media MediaObservation, snapshot ftlpagemap.Snapshot) (Device, error) {
	return restoreMedia(media, snapshot, false)
}

// RestoreWithInterruptedErases rebuilds a page map and physical observation
// that includes feasible prefix progress from one or more interrupted erases.
// The caller keeps both input views stable until return; a refusal returns the
// zero device and a success owns detached page bytes and fresh owner identities.
func RestoreWithInterruptedErases(media MediaObservation, snapshot ftlpagemap.Snapshot) (Device, error) {
	return restoreMedia(media, snapshot, true)
}

func restoreMedia(media MediaObservation, snapshot ftlpagemap.Snapshot, allowInterrupted bool) (Device, error) {
	blockCount := int64(len(media.Blocks))
	pagesPerBlock := media.PagesPerBlock
	if blockCount < 1 || blockCount > maxRestoredPhysicalPages ||
		pagesPerBlock < 1 || pagesPerBlock > maxRestoredPhysicalPages ||
		blockCount > maxRestoredPhysicalPages/pagesPerBlock {
		return Device{}, ErrInvalidBlockGeometry
	}
	physicalCount := blockCount * pagesPerBlock

	table, err := ftlpagemap.Restore(media.LogicalPages, physicalCount, snapshot)
	if err != nil {
		return Device{}, err
	}
	if err := validatePageSize(media.PageBytes); err != nil {
		return Device{}, err
	}
	if media.MaxCompletedErases > maxSafeEraseCount {
		return Device{}, nandblockerasecuts.ErrInvalidEraseCount
	}

	for blockIndex, block := range media.Blocks {
		if len(block.Pages) != int(pagesPerBlock) {
			return Device{}, ErrInvalidMediaBlock
		}
		if block.CompletedErases > media.MaxCompletedErases {
			return Device{}, nandblockerasecuts.ErrInvalidEraseCount
		}
		if block.IncompleteErase {
			if !allowInterrupted {
				return Device{}, ErrIncompleteMediaBlock
			}
			if block.CompletedErases == media.MaxCompletedErases {
				return Device{}, nandblockerasecuts.ErrEraseLimit
			}
		}
		hasStale := false
		minimumPrefix := int64(0)
		maximumPrefix := pagesPerBlock - 1
		for pageIndex, page := range block.Pages {
			if len(page.Bytes) != media.PageBytes {
				return Device{}, ErrInvalidPageSize
			}
			if !page.Programmed && !restorePageIsErased(page.Bytes) {
				return Device{}, nandpageprogramcuts.ErrInvalidPageState
			}
			physical := int64(blockIndex)*pagesPerBlock + int64(pageIndex)
			role := snapshot.PhysicalStatus[int(physical)]
			switch role {
			case ftlpagemap.Free:
				if page.Programmed {
					return Device{}, ErrMediaRoleMismatch
				}
			case ftlpagemap.Live, ftlpagemap.Stale:
				if block.IncompleteErase && role == ftlpagemap.Live {
					return Device{}, ErrIncompleteMediaBlock
				}
				if role == ftlpagemap.Stale && block.IncompleteErase {
					hasStale = true
					if page.Programmed {
						if int64(pageIndex) < maximumPrefix {
							maximumPrefix = int64(pageIndex)
						}
					} else if int64(pageIndex)+1 > minimumPrefix {
						minimumPrefix = int64(pageIndex) + 1
					}
					continue
				}
				if !page.Programmed && !(block.IncompleteErase && role == ftlpagemap.Stale) {
					return Device{}, ErrMediaRoleMismatch
				}
			}
		}
		if block.IncompleteErase && (!hasStale || minimumPrefix > maximumPrefix) {
			return Device{}, ErrIncompleteMediaBlock
		}
	}

	// Build the successor only after all paired evidence passes validation.
	restored := Device{
		table:              table,
		pages:              make([]nandpageprogramcuts.PageState, int(physicalCount)),
		ownerIdentities:    make([]*ownerPublicationIdentity, len(snapshot.LogicalToPhysical)),
		pageSize:           media.PageBytes,
		pagesPerBlock:      pagesPerBlock,
		maxCompletedErases: media.MaxCompletedErases,
		blocks:             make([]BlockSnapshot, int(blockCount)),
	}
	for blockIndex, block := range media.Blocks {
		restored.blocks[blockIndex] = BlockSnapshot{
			CompletedErases: block.CompletedErases,
			IncompleteErase: block.IncompleteErase,
		}
		for pageIndex, page := range block.Pages {
			physical := blockIndex*int(pagesPerBlock) + pageIndex
			restored.pages[physical] = nandpageprogramcuts.PageState{
				Bytes:      append([]byte(nil), page.Bytes...),
				Programmed: page.Programmed,
			}
		}
	}
	for logical, physical := range snapshot.LogicalToPhysical {
		if physical >= 0 {
			restored.ownerIdentities[logical] = &ownerPublicationIdentity{marker: 1}
		}
	}
	return restored, nil
}

func restorePageIsErased(page []byte) bool {
	for _, value := range page {
		if value != 0xff {
			return false
		}
	}
	return true
}
