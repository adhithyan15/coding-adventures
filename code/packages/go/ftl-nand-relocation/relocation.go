package ftlnandrelocation

import (
	"errors"

	ftlevacuationplan "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-evacuation-plan"
	ftlpagemap "github.com/adhithyan15/coding-adventures/code/packages/go/ftl-page-map"
	nandblockerasecuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-block-erase-cuts"
	nandpageprogramcuts "github.com/adhithyan15/coding-adventures/code/packages/go/nand-page-program-cuts"
)

const (
	maxPageSize       = 65_536
	maxSafeEraseCount = uint64(9_007_199_254_740_991)
)

var (
	ErrInvalidPageSize            = nandpageprogramcuts.ErrInvalidPageSize
	ErrInvalidBlockGeometry       = errors.New("invalid FTL erase-block geometry")
	ErrEraseDisabled              = errors.New("block erase is disabled for this device")
	ErrInvalidBlockIndex          = errors.New("invalid FTL erase-block index")
	ErrIneligibleBlock            = errors.New("FTL erase block contains no stale page or contains a live page")
	ErrEvacuationBlockIncomplete  = errors.New("evacuation block erase is incomplete")
	ErrInvalidCopySource          = errors.New("invalid evacuation copy source")
	ErrCopySourceChanged          = errors.New("evacuation copy source no longer owns the logical page")
	ErrInvalidCopyDestination     = errors.New("invalid evacuation copy destination")
	ErrCopyDestinationUnavailable = errors.New("evacuation copy destination is unavailable")
	ErrEvacuationActive           = errors.New("an evacuation is already active for this device or block")
	ErrNoActiveEvacuation         = errors.New("no evacuation is active")
	ErrEvacuationNoLivePages      = errors.New("evacuation has no remaining live pages")
	ErrEvacuationStillLive        = errors.New("evacuation target still contains live pages")
	ErrOwnerObservationStale      = errors.New("owner observation is stale or does not match the current owner")
)

// Cut is the deterministic power-loss point used by DB04b1.
type Cut = nandpageprogramcuts.Cut

// EraseCut is the deterministic power-loss point used by DB04d.
type EraseCut = nandblockerasecuts.Cut

// Receipt separates publication to the mapping from caller acknowledgement.
type Receipt struct {
	Physical     int64
	Published    bool
	Acknowledged bool
}

// UnownedProgramReceipt identifies the physical page consumed by a raw
// program. It deliberately carries no logical publication state.
type UnownedProgramReceipt struct {
	Physical     int64
	Programmed   bool
	Acknowledged bool
}

// Snapshot owns a detached page-map view and detached physical page bytes.
type Snapshot struct {
	Map   ftlpagemap.Snapshot
	Pages []nandpageprogramcuts.PageState
}

// BlockSnapshot reports erase progress. Completed erases count media results,
// whether or not the caller received acknowledgement.
type BlockSnapshot struct {
	CompletedErases uint64
	IncompleteErase bool
}

// PageOwner identifies a logical value's current physical page.
type PageOwner struct {
	Logical  int64
	Physical int64
}

type ownerPublicationIdentity struct {
	marker byte
}

// OwnerObservation identifies one published logical-to-physical ownership
// event within this process. Its publication identity is intentionally opaque.
type OwnerObservation struct {
	// Owner is the current logical and physical address pair.
	Owner    PageOwner
	identity *ownerPublicationIdentity
}

// ReclamationSnapshot is a detached view of equal-sized physical erase blocks.
type ReclamationSnapshot struct {
	PagesPerBlock      int64
	MaxCompletedErases uint64
	Blocks             []BlockSnapshot
}

// ReclamationObservation is the compact role and progress view used to choose
// physical reclamation candidates. Unlike Snapshot and ObserveMedia, it never
// copies page payload bytes or exposes logical addresses.
type ReclamationObservation struct {
	Blocks             []BlockSnapshot
	PhysicalStatus     []ftlpagemap.PhysicalStatus
	PagesPerBlock      int64
	MaxCompletedErases uint64
	Active             bool
	ActiveBlock        int64
}

// EraseReceipt distinguishes metadata release from caller acknowledgement.
type EraseReceipt struct {
	Block        int64
	Released     bool
	Acknowledged bool
}

// Device is a value-owned media image and translation table. Its zero value
// is unconfigured; methods never mutate a device value already held by caller.
type Device struct {
	table              ftlpagemap.Table
	pages              []nandpageprogramcuts.PageState
	ownerIdentities    []*ownerPublicationIdentity
	pageSize           int
	pagesPerBlock      int64
	maxCompletedErases uint64
	blocks             []BlockSnapshot
	evacuationActive   bool
	evacuationBlock    int64
}

// New checks geometry before page width, then creates erased unprogrammed NAND.
func New(logicalCount, physicalCount int64, pageSize int) (Device, error) {
	table, err := ftlpagemap.New(logicalCount, physicalCount)
	if err != nil {
		return Device{}, err
	}
	if err := validatePageSize(pageSize); err != nil {
		return Device{}, err
	}
	return newErasedDevice(table, pageSize), nil
}

// NewWithEraseBlocks constructs a relocation device with equal-sized erase
// blocks and a per-block completed-erase ceiling.
func NewWithEraseBlocks(logicalCount, physicalCount int64, pageSize int, pagesPerBlock int64, maxCompletedErases uint64) (Device, error) {
	table, err := ftlpagemap.New(logicalCount, physicalCount)
	if err != nil {
		return Device{}, err
	}
	if err := validatePageSize(pageSize); err != nil {
		return Device{}, err
	}
	if pagesPerBlock < 1 || pagesPerBlock > physicalCount || physicalCount%pagesPerBlock != 0 {
		return Device{}, ErrInvalidBlockGeometry
	}
	if maxCompletedErases > maxSafeEraseCount {
		return Device{}, nandblockerasecuts.ErrInvalidEraseCount
	}
	device := newErasedDevice(table, pageSize)
	device.pagesPerBlock = pagesPerBlock
	device.maxCompletedErases = maxCompletedErases
	device.blocks = make([]BlockSnapshot, int(physicalCount/pagesPerBlock))
	return device, nil
}

func validatePageSize(pageSize int) error {
	if pageSize < 1 || pageSize > maxPageSize {
		return ErrInvalidPageSize
	}
	return nil
}

func newErasedDevice(table ftlpagemap.Table, pageSize int) Device {
	snapshot := table.Snapshot()
	physicalCount := len(snapshot.PhysicalStatus)
	pages := make([]nandpageprogramcuts.PageState, physicalCount)
	for index := range pages {
		pages[index].Bytes = erasedPage(pageSize)
	}
	return Device{
		table: table, pages: pages, pageSize: pageSize,
		ownerIdentities: make([]*ownerPublicationIdentity, len(snapshot.LogicalToPhysical)),
	}
}

func (device Device) configured() bool {
	return device.configuredWithSnapshot(device.table.Snapshot())
}

func (device Device) configuredWithSnapshot(snapshot ftlpagemap.Snapshot) bool {
	if device.pageSize < 1 || device.pageSize > maxPageSize {
		return false
	}
	if len(snapshot.LogicalToPhysical) < 1 || len(snapshot.LogicalToPhysical) > 64 ||
		len(snapshot.PhysicalStatus) < 2 || len(snapshot.PhysicalStatus) > 128 ||
		len(snapshot.PhysicalStatus) <= len(snapshot.LogicalToPhysical) ||
		len(device.pages) != len(snapshot.PhysicalStatus) ||
		len(device.ownerIdentities) != len(snapshot.LogicalToPhysical) || snapshot.Reservation != nil {
		return false
	}
	blockProfile := device.pagesPerBlock != 0
	if blockProfile {
		if device.pagesPerBlock < 1 || device.pagesPerBlock > int64(len(device.pages)) || int64(len(device.pages))%device.pagesPerBlock != 0 ||
			device.maxCompletedErases > maxSafeEraseCount || len(device.blocks) != int(int64(len(device.pages))/device.pagesPerBlock) {
			return false
		}
		for _, block := range device.blocks {
			if block.CompletedErases > device.maxCompletedErases {
				return false
			}
		}
		if device.evacuationActive && (device.evacuationBlock < 0 || device.evacuationBlock >= int64(len(device.blocks))) {
			return false
		}
	} else if device.maxCompletedErases != 0 || len(device.blocks) != 0 {
		return false
	} else if device.evacuationActive {
		return false
	}
	for index, page := range device.pages {
		if len(page.Bytes) != device.pageSize {
			return false
		}
		switch snapshot.PhysicalStatus[index] {
		case ftlpagemap.Free:
			if page.Programmed {
				return false
			}
		case ftlpagemap.Reserved:
			return false
		case ftlpagemap.Live:
			if !page.Programmed {
				return false
			}
			if blockProfile && device.blocks[index/int(device.pagesPerBlock)].IncompleteErase {
				return false
			}
		case ftlpagemap.Stale:
			if !page.Programmed {
				if !blockProfile || !device.blocks[index/int(device.pagesPerBlock)].IncompleteErase {
					return false
				}
			}
		default:
			return false
		}
	}
	return true
}

// Lookup reports the current physical page for a logical page. Absence is
// distinct from a present mapping to physical page zero.
func (device Device) Lookup(logical int64) (physical int64, present bool, err error) {
	if !device.configured() {
		return 0, false, ftlpagemap.ErrInvalidGeometry
	}
	return device.table.Lookup(logical)
}

// ObserveOwner returns the current published owner and its opaque identity.
// An unmapped logical page returns the zero observation and false.
func (device Device) ObserveOwner(logical int64) (OwnerObservation, bool, error) {
	physical, present, err := device.Lookup(logical)
	if err != nil || !present {
		return OwnerObservation{}, false, err
	}
	return OwnerObservation{
		Owner:    PageOwner{Logical: logical, Physical: physical},
		identity: device.ownerIdentities[int(logical)],
	}, true, nil
}

// MatchesOwner reports whether an observation still identifies the current
// publication at both of its recorded coordinates.
func (device Device) MatchesOwner(observation OwnerObservation) bool {
	if observation.identity == nil {
		return false
	}
	physical, present, err := device.Lookup(observation.Owner.Logical)
	if err != nil || !present || physical != observation.Owner.Physical {
		return false
	}
	return device.ownerIdentities[int(observation.Owner.Logical)] == observation.identity
}

// InvalidateOwner removes one exactly observed current mapping and clears only
// that logical page's publication identity. The physical media is unchanged.
func (device Device) InvalidateOwner(observation OwnerObservation) (Device, error) {
	if !device.configured() {
		return device, ftlpagemap.ErrInvalidGeometry
	}
	if !device.MatchesOwner(observation) {
		return device, ErrOwnerObservationStale
	}
	table, err := device.table.InvalidateCurrent(observation.Owner.Logical, observation.Owner.Physical)
	if err != nil {
		return device, err
	}
	next := device
	next.table = table
	next.ownerIdentities = append([]*ownerPublicationIdentity(nil), device.ownerIdentities...)
	next.ownerIdentities[int(observation.Owner.Logical)] = nil
	return next, nil
}

// Read returns a detached copy of the currently mapped page or nil when the
// logical address has no current owner.
func (device Device) Read(logical int64) ([]byte, bool, error) {
	physical, present, err := device.Lookup(logical)
	if err != nil {
		return nil, false, err
	}
	if !present {
		return nil, false, nil
	}
	return append([]byte(nil), device.pages[int(physical)].Bytes...), true, nil
}

// Write reserves the lowest free page, computes the explicit media cut, and
// returns a successor value. A refusal rolls back the internal reservation by
// returning the original device with a zero Receipt.
func (device Device) Write(logical int64, payload []byte, cut Cut) (Device, Receipt, error) {
	return device.WriteExcluding(logical, payload, cut, nil)
}

// WriteExcluding reserves the lowest free page outside the caller's exclusion
// mask and the device's existing erase and evacuation fences. The mask is
// borrowed only during this call and is never retained or changed.
func (device Device) WriteExcluding(logical int64, payload []byte, cut Cut, excludedPages []bool) (Device, Receipt, error) {
	snapshot := device.table.Snapshot()
	if !device.configuredWithSnapshot(snapshot) {
		if snapshot.Reservation != nil {
			// Let the page map preserve logical-before-pending-before-width
			// validation for the only intermediate it can expose.
			_, _, err := device.table.BeginExcluding(logical, excludedPages)
			return device, Receipt{}, err
		}
		return device, Receipt{}, ftlpagemap.ErrInvalidGeometry
	}
	if len(excludedPages) != 0 && len(excludedPages) != len(device.pages) {
		// Preserve page-map logical and pending-reservation precedence for a
		// malformed caller mask instead of reimplementing its checks here.
		_, _, err := device.table.BeginExcluding(logical, excludedPages)
		return device, Receipt{}, err
	}
	fence := excludedPages
	if device.pagesPerBlock != 0 {
		// Copy the caller's mask before adding the device-owned fences, so
		// combining the constraints cannot modify a slice held by the caller.
		fence = make([]bool, len(device.pages))
		copy(fence, excludedPages)
		for blockIndex, block := range device.blocks {
			if !block.IncompleteErase && (!device.evacuationActive || int64(blockIndex) != device.evacuationBlock) {
				continue
			}
			first := blockIndex * int(device.pagesPerBlock)
			for offset := 0; offset < int(device.pagesPerBlock); offset++ {
				fence[first+offset] = true
			}
		}
	}
	reservedTable, reservation, err := device.table.BeginExcluding(logical, fence)
	if err != nil {
		return device, Receipt{}, err
	}
	return device.programReservation(reservedTable, reservation, payload, cut)
}

// ProgramUnownedPage applies NAND program cuts to one caller-selected page
// without publishing a logical mapping. Consumed media is recorded as stale
// in the page map, and the returned receipt reports its absolute page index.
func (device Device) ProgramUnownedPage(block, page int64, payload []byte, cut Cut) (Device, UnownedProgramReceipt, error) {
	snapshot := device.table.Snapshot()
	first, end, err := device.validatedBlockRange(snapshot, block)
	if err != nil {
		return device, UnownedProgramReceipt{}, err
	}
	if page < 0 || page >= end-first {
		return device, UnownedProgramReceipt{}, ftlpagemap.ErrInvalidPhysicalPage
	}
	physical := first + page
	if device.blocks[int(block)].IncompleteErase {
		return device, UnownedProgramReceipt{}, ErrEvacuationBlockIncomplete
	}
	if device.evacuationActive && block == device.evacuationBlock {
		return device, UnownedProgramReceipt{}, ErrEvacuationActive
	}
	if snapshot.PhysicalStatus[int(physical)] != ftlpagemap.Free {
		return device, UnownedProgramReceipt{}, ftlpagemap.ErrPhysicalPageUnavailable
	}

	program, err := nandpageprogramcuts.Apply(device.pageSize, device.pages[int(physical)], payload, cut)
	if err != nil {
		return device, UnownedProgramReceipt{}, err
	}
	if !program.State.Programmed {
		return device, UnownedProgramReceipt{Physical: physical}, nil
	}
	table := device.table
	table, err = table.RecordUnownedProgram(physical)
	if err != nil {
		return device, UnownedProgramReceipt{}, err
	}
	pages := append([]nandpageprogramcuts.PageState(nil), device.pages...)
	pages[int(physical)] = program.State
	next := device
	next.table = table
	next.pages = pages
	receipt := UnownedProgramReceipt{
		Physical: physical, Programmed: program.State.Programmed,
		Acknowledged: program.Acknowledged,
	}
	return next, receipt, nil
}

// CopyLivePage executes one previously planned live-page move after
// rechecking its source and exact destination against the current device.
func (device Device) CopyLivePage(blockIndex int64, move ftlevacuationplan.Copy, cut Cut) (Device, Receipt, error) {
	return device.copyLivePage(blockIndex, move, cut, false)
}

func (device Device) copyLivePage(blockIndex int64, move ftlevacuationplan.Copy, cut Cut, allowActive bool) (Device, Receipt, error) {
	if !device.configured() {
		return device, Receipt{}, ftlpagemap.ErrInvalidGeometry
	}
	if device.pagesPerBlock == 0 {
		return device, Receipt{}, ErrEraseDisabled
	}
	if blockIndex < 0 || blockIndex >= int64(len(device.blocks)) {
		return device, Receipt{}, ErrInvalidBlockIndex
	}
	if device.evacuationActive && int64(blockIndex) == device.evacuationBlock && !allowActive {
		return device, Receipt{}, ErrEvacuationActive
	}
	if device.blocks[int(blockIndex)].IncompleteErase {
		return device, Receipt{}, ErrEvacuationBlockIncomplete
	}
	snapshot := device.table.Snapshot()
	if move.Logical < 0 || move.Logical >= int64(len(snapshot.LogicalToPhysical)) {
		return device, Receipt{}, ftlpagemap.ErrInvalidLogicalPage
	}
	first := blockIndex * device.pagesPerBlock
	if move.SourcePhysical < first || move.SourcePhysical >= first+device.pagesPerBlock {
		return device, Receipt{}, ErrInvalidCopySource
	}
	if snapshot.LogicalToPhysical[int(move.Logical)] != move.SourcePhysical {
		return device, Receipt{}, ErrCopySourceChanged
	}
	if move.DestinationPhysical < 0 || move.DestinationPhysical >= int64(len(snapshot.PhysicalStatus)) {
		return device, Receipt{}, ErrInvalidCopyDestination
	}
	if move.DestinationPhysical >= first && move.DestinationPhysical < first+device.pagesPerBlock {
		return device, Receipt{}, ErrInvalidCopyDestination
	}
	destinationBlock := move.DestinationPhysical / device.pagesPerBlock
	if (device.evacuationActive && destinationBlock == device.evacuationBlock) || device.blocks[int(destinationBlock)].IncompleteErase ||
		snapshot.PhysicalStatus[int(move.DestinationPhysical)] != ftlpagemap.Free {
		return device, Receipt{}, ErrCopyDestinationUnavailable
	}

	// Exclude every slot except the exact planned destination. The page map
	// still performs the reservation through its normal immutable transition.
	fence := make([]bool, len(device.pages))
	for index := range fence {
		fence[index] = true
	}
	fence[int(move.DestinationPhysical)] = false
	reservedTable, reservation, err := device.table.BeginExcluding(move.Logical, fence)
	if err != nil {
		return device, Receipt{}, err
	}
	return device.programReservation(reservedTable, reservation, device.pages[int(move.SourcePhysical)].Bytes, cut)
}

func (device Device) programReservation(reservedTable ftlpagemap.Table, reservation ftlpagemap.Reservation, payload []byte, cut Cut) (Device, Receipt, error) {
	program, err := nandpageprogramcuts.Apply(
		device.pageSize,
		device.pages[int(reservation.Physical)],
		payload,
		cut,
	)
	if err != nil {
		return device, Receipt{}, err
	}

	disposition := ftlpagemap.AbortFresh
	published := false
	switch cut.Kind {
	case nandpageprogramcuts.BeforeAccept:
		disposition = ftlpagemap.AbortFresh
	case nandpageprogramcuts.AfterPrefix:
		disposition = ftlpagemap.AbortSpent
	case nandpageprogramcuts.AfterMedia, nandpageprogramcuts.Complete:
		disposition = ftlpagemap.MediaReady
		published = true
	}
	finishedTable, err := reservedTable.Finish(disposition)
	if err != nil {
		return device, Receipt{}, err
	}

	pages := make([]nandpageprogramcuts.PageState, len(device.pages))
	for index, page := range device.pages {
		if index == int(reservation.Physical) {
			// Apply returns owned bytes for the destination page.
			pages[index] = program.State
			continue
		}
		pages[index] = clonePage(page)
	}
	next := Device{
		table: finishedTable, pages: pages, pageSize: device.pageSize,
		ownerIdentities: device.ownerIdentities,
		pagesPerBlock:   device.pagesPerBlock, maxCompletedErases: device.maxCompletedErases,
		blocks:           append([]BlockSnapshot(nil), device.blocks...),
		evacuationActive: device.evacuationActive, evacuationBlock: device.evacuationBlock,
	}
	if published {
		next.ownerIdentities = append([]*ownerPublicationIdentity(nil), device.ownerIdentities...)
		next.ownerIdentities[int(reservation.Logical)] = &ownerPublicationIdentity{marker: 1}
	}
	receipt := Receipt{
		Physical:     reservation.Physical,
		Published:    published,
		Acknowledged: program.Acknowledged,
	}
	return next, receipt, nil
}

// ReclamationSnapshot returns detached block progress for the block profile.
// Legacy and unconfigured devices expose the zero value.
func (device Device) ReclamationSnapshot() ReclamationSnapshot {
	if !device.configured() || device.pagesPerBlock == 0 {
		return ReclamationSnapshot{}
	}
	return ReclamationSnapshot{
		PagesPerBlock:      device.pagesPerBlock,
		MaxCompletedErases: device.maxCompletedErases,
		Blocks:             append([]BlockSnapshot(nil), device.blocks...),
	}
}

// ObserveReclamation returns detached physical roles, per-block erase progress
// and the current evacuation fence without copying NAND page payload bytes.
// The method validates the configured device before requiring its erase-block
// profile, matching the ordered physical-capture refusal semantics.
func (device Device) ObserveReclamation() (ReclamationObservation, error) {
	snapshot := device.table.Snapshot()
	if !device.configuredWithSnapshot(snapshot) {
		return ReclamationObservation{}, ftlpagemap.ErrInvalidGeometry
	}
	if device.pagesPerBlock == 0 {
		return ReclamationObservation{}, ErrEraseDisabled
	}

	// These are the only per-page and per-block values required by victim
	// selection. In particular, the payload-bearing page slice is never copied.
	blocks := append([]BlockSnapshot(nil), device.blocks...)
	// Table.Snapshot returns its own detached status slice, so it is safe to
	// transfer that one copy directly into this result.
	physicalStatus := snapshot.PhysicalStatus
	observation := ReclamationObservation{
		Blocks:             blocks,
		PhysicalStatus:     physicalStatus,
		PagesPerBlock:      device.pagesPerBlock,
		MaxCompletedErases: device.maxCompletedErases,
	}
	if device.evacuationActive {
		observation.Active = true
		observation.ActiveBlock = device.evacuationBlock
	}
	return observation, nil
}

// ObserveBlock returns the physical pages and erase progress for one block.
// Page bytes and the page list are deeply detached. This raw observation is
// available for active evacuation and incomplete erase states; callers decide
// separately whether those contents are readable or admissible.
func (device Device) ObserveBlock(block int64) (nandblockerasecuts.Block, error) {
	snapshot := device.table.Snapshot()
	first, end, err := device.validatedBlockRange(snapshot, block)
	if err != nil {
		return nandblockerasecuts.Block{}, err
	}
	return device.observeBlockRange(block, first, end), nil
}

// MediaObservation is the ordered physical NAND image of a configured
// block-enabled relocation device. It intentionally excludes logical owners.
type MediaObservation struct {
	LogicalPages       int64
	PagesPerBlock      int64
	PageBytes          int
	MaxCompletedErases uint64
	Blocks             []nandblockerasecuts.Block
}

// ObserveMedia returns a deeply detached physical image in ascending block
// order. As with ObserveBlock, active and incomplete blocks remain observable.
func (device Device) ObserveMedia() (MediaObservation, error) {
	snapshot := device.table.Snapshot()
	if !device.configuredWithSnapshot(snapshot) {
		return MediaObservation{}, ftlpagemap.ErrInvalidGeometry
	}
	if device.pagesPerBlock == 0 {
		return MediaObservation{}, ErrEraseDisabled
	}
	observation := MediaObservation{
		LogicalPages:       int64(len(snapshot.LogicalToPhysical)),
		PagesPerBlock:      device.pagesPerBlock,
		PageBytes:          device.pageSize,
		MaxCompletedErases: device.maxCompletedErases,
		Blocks:             make([]nandblockerasecuts.Block, len(device.blocks)),
	}
	for block := range device.blocks {
		first := int64(block) * device.pagesPerBlock
		observation.Blocks[block] = device.observeBlockRange(int64(block), first, first+device.pagesPerBlock)
	}
	return observation, nil
}

// observeBlockRange copies one already-validated extent. Both public physical
// observations share this byte and marker-preserving capture path.
func (device Device) observeBlockRange(block, first, end int64) nandblockerasecuts.Block {
	progress := device.blocks[int(block)]
	observed := nandblockerasecuts.Block{
		Pages:           make([]nandblockerasecuts.Page, int(end-first)),
		CompletedErases: progress.CompletedErases,
		IncompleteErase: progress.IncompleteErase,
	}
	for physical := first; physical < end; physical++ {
		page := device.pages[int(physical)]
		observed.Pages[int(physical-first)] = nandblockerasecuts.Page{
			Bytes:      append([]byte(nil), page.Bytes...),
			Programmed: page.Programmed,
		}
	}
	return observed
}

// LivePagesInBlock reports the number of currently mapped pages in one
// physical erase block. The page map is authoritative for liveness; page
// bytes and program markers are deliberately not part of this observation.
// It is a read-only query and is valid for active, incomplete, empty, full,
// and erase-budget-exhausted blocks.
func (device Device) LivePagesInBlock(block int64) (int64, error) {
	snapshot := device.table.Snapshot()
	first, end, err := device.validatedBlockRange(snapshot, block)
	if err != nil {
		return 0, err
	}
	var live int64
	for physical := first; physical < end; physical++ {
		if snapshot.PhysicalStatus[int(physical)] == ftlpagemap.Live {
			live++
		}
	}
	return live, nil
}

// OwnersInBlock returns the current logical owners in one physical erase
// block, in ascending logical-page order. Stale and spent pages are not
// owners, and this read-only observation remains valid during block work.
func (device Device) OwnersInBlock(block int64) ([]PageOwner, error) {
	snapshot := device.table.Snapshot()
	first, end, err := device.validatedBlockRange(snapshot, block)
	if err != nil {
		return nil, err
	}

	owners := make([]PageOwner, 0)
	for logical, physical := range snapshot.LogicalToPhysical {
		if physical >= first && physical < end && snapshot.PhysicalStatus[int(physical)] == ftlpagemap.Live {
			owners = append(owners, PageOwner{Logical: int64(logical), Physical: physical})
		}
	}
	return owners, nil
}

// validatedBlockRange applies the common status, erase-profile, and signed
// block validation order to one page-map snapshot.
func (device Device) validatedBlockRange(snapshot ftlpagemap.Snapshot, block int64) (int64, int64, error) {
	if !device.configuredWithSnapshot(snapshot) {
		return 0, 0, ftlpagemap.ErrInvalidGeometry
	}
	if device.pagesPerBlock == 0 {
		return 0, 0, ErrEraseDisabled
	}
	if block < 0 || block >= int64(len(device.blocks)) {
		return 0, 0, ErrInvalidBlockIndex
	}
	first := block * device.pagesPerBlock
	return first, first + device.pagesPerBlock, nil
}

// EraseBlock applies one DB04d block cut, quarantines partial progress, and
// releases the complete map extent only after the media count advances.
func (device Device) EraseBlock(blockIndex int64, cut EraseCut) (Device, EraseReceipt, error) {
	return device.eraseBlock(blockIndex, cut, false)
}

func (device Device) eraseBlock(blockIndex int64, cut EraseCut, allowActive bool) (Device, EraseReceipt, error) {
	snapshot := device.table.Snapshot()
	first, end, err := device.validatedBlockRange(snapshot, blockIndex)
	if err != nil {
		return device, EraseReceipt{}, err
	}
	if device.evacuationActive && blockIndex == device.evacuationBlock && !allowActive {
		return device, EraseReceipt{}, ErrEvacuationActive
	}
	if err := validateEraseRoles(snapshot.PhysicalStatus, first, end); err != nil {
		return device, EraseReceipt{}, err
	}
	block := nandblockerasecuts.Block{
		Pages:           make([]nandblockerasecuts.Page, int(device.pagesPerBlock)),
		CompletedErases: device.blocks[int(blockIndex)].CompletedErases,
		IncompleteErase: device.blocks[int(blockIndex)].IncompleteErase,
	}
	for offset := range block.Pages {
		page := device.pages[int(first)+offset]
		block.Pages[offset] = nandblockerasecuts.Page{Bytes: page.Bytes, Programmed: page.Programmed}
	}
	outcome, err := nandblockerasecuts.Apply(device.pageSize, device.maxCompletedErases, block, cut)
	if err != nil {
		return device, EraseReceipt{}, err
	}
	blocks := append([]BlockSnapshot(nil), device.blocks...)
	blocks[int(blockIndex)] = BlockSnapshot{CompletedErases: outcome.Block.CompletedErases, IncompleteErase: outcome.Block.IncompleteErase}
	// Device pages are immutable value state. Copy the page table, then use
	// Apply's owned output for this block; untouched page buffers may be shared.
	pages := append([]nandpageprogramcuts.PageState(nil), device.pages...)
	for offset, page := range outcome.Block.Pages {
		pages[int(first)+offset] = nandpageprogramcuts.PageState{Bytes: page.Bytes, Programmed: page.Programmed}
	}
	table := device.table
	released := outcome.Block.CompletedErases > block.CompletedErases
	if released {
		var releaseErr error
		table, releaseErr = table.ReleaseErased(first, device.pagesPerBlock)
		if releaseErr != nil {
			return device, EraseReceipt{}, releaseErr
		}
	}
	next := Device{
		table: table, pages: pages, pageSize: device.pageSize,
		ownerIdentities: device.ownerIdentities,
		pagesPerBlock:   device.pagesPerBlock, maxCompletedErases: device.maxCompletedErases, blocks: blocks,
		evacuationActive: device.evacuationActive, evacuationBlock: device.evacuationBlock,
	}
	receipt := EraseReceipt{Block: blockIndex, Released: released, Acknowledged: outcome.Acknowledged}
	return next, receipt, nil
}

// Snapshot returns detached map arrays, reservation data and media bytes.
func (device Device) Snapshot() Snapshot {
	if !device.configured() {
		return Snapshot{}
	}
	return Snapshot{Map: device.table.Snapshot(), Pages: clonePages(device.pages)}
}

func clonePages(pages []nandpageprogramcuts.PageState) []nandpageprogramcuts.PageState {
	result := make([]nandpageprogramcuts.PageState, len(pages))
	for index, page := range pages {
		result[index] = clonePage(page)
	}
	return result
}

func clonePage(page nandpageprogramcuts.PageState) nandpageprogramcuts.PageState {
	return nandpageprogramcuts.PageState{
		Bytes:      append([]byte(nil), page.Bytes...),
		Programmed: page.Programmed,
	}
}

func erasedPage(width int) []byte {
	page := make([]byte, width)
	for index := range page {
		page[index] = 0xff
	}
	return page
}
