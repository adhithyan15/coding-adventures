package ftlpagemap

import "errors"

const (
	minLogicalPages  = 1
	maxLogicalPages  = 64
	minPhysicalPages = 2
	maxPhysicalPages = 128
	noMapping        = -1
)

// Refusal errors are stable classes for callers and model adapters.
var (
	ErrInvalidGeometry     = errors.New("invalid geometry")
	ErrInvalidLogicalPage  = errors.New("invalid logical page")
	ErrInvalidPhysicalPage = errors.New("invalid physical page")
	ErrCurrentOwnerChanged = errors.New("current logical owner changed")
	ErrReservationPending  = errors.New("reservation already pending")
	ErrNoFreePage          = errors.New("no free physical page")
	ErrNoReservation       = errors.New("no reservation")
	ErrInvalidDisposition  = errors.New("invalid finish disposition")
	ErrInvalidFence        = errors.New("invalid allocation fence")
	ErrInvalidExtent       = errors.New("invalid erased extent")
	ErrIneligibleExtent    = errors.New("erased extent contains a live or reserved page")
)

// Disposition tells Finish what evidence the caller has for the reserved page.
type Disposition uint8

const (
	MediaReady Disposition = iota
	AbortFresh
	AbortSpent
)

// PhysicalStatus classifies one physical page in the translation table.
type PhysicalStatus uint8

const (
	Free PhysicalStatus = iota
	Reserved
	Live
	Stale
)

// Reservation identifies the logical destination and exclusively reserved slot.
type Reservation struct {
	Logical  int64
	Physical int64
}

// Snapshot is a detached, inspectable view. Missing mappings use -1.
type Snapshot struct {
	LogicalToPhysical []int64
	PhysicalStatus    []PhysicalStatus
	Reservation       *Reservation
}

// Table owns all private arrays. Its zero value is deliberately unconfigured.
type Table struct {
	logicalCount  int64
	physicalCount int64
	logicalMap    []int64
	physical      []PhysicalStatus
	pending       *Reservation
}

// New constructs an empty table after checking the supported geometry.
func New(logicalCount, physicalCount int64) (Table, error) {
	if logicalCount < minLogicalPages || logicalCount > maxLogicalPages ||
		physicalCount < minPhysicalPages || physicalCount > maxPhysicalPages ||
		physicalCount <= logicalCount {
		return Table{}, ErrInvalidGeometry
	}
	logicalMap := make([]int64, int(logicalCount))
	for index := range logicalMap {
		logicalMap[index] = noMapping
	}
	return Table{
		logicalCount:  logicalCount,
		physicalCount: physicalCount,
		logicalMap:    logicalMap,
		physical:      make([]PhysicalStatus, int(physicalCount)),
	}, nil
}

func (table Table) configured() bool {
	return table.logicalCount >= minLogicalPages && table.logicalCount <= maxLogicalPages &&
		table.physicalCount >= minPhysicalPages && table.physicalCount <= maxPhysicalPages &&
		table.physicalCount > table.logicalCount &&
		len(table.logicalMap) == int(table.logicalCount) && len(table.physical) == int(table.physicalCount)
}

// Lookup returns the current live location. A pending relocation does not
// publish its destination, so a prior mapping remains visible until Finish.
func (table Table) Lookup(logical int64) (physical int64, present bool, err error) {
	if !table.configured() {
		return 0, false, ErrInvalidGeometry
	}
	if logical < 0 || logical >= table.logicalCount {
		return 0, false, ErrInvalidLogicalPage
	}
	physical = table.logicalMap[int(logical)]
	if physical == noMapping {
		return 0, false, nil
	}
	return physical, true, nil
}

// InvalidateCurrent removes one exact mapping and marks its old slot stale.
// The slot stays unavailable until ReleaseErased receives completed-media
// evidence from the caller.
func (table Table) InvalidateCurrent(logical, physical int64) (Table, error) {
	if !table.configured() {
		return table, ErrInvalidGeometry
	}
	if logical < 0 || logical >= table.logicalCount {
		return table, ErrInvalidLogicalPage
	}
	if physical < 0 || physical >= table.physicalCount {
		return table, ErrInvalidPhysicalPage
	}
	if table.pending != nil {
		return table, ErrReservationPending
	}
	if table.logicalMap[int(logical)] != physical {
		return table, ErrCurrentOwnerChanged
	}

	result := table.clone()
	result.logicalMap[int(logical)] = noMapping
	result.physical[int(physical)] = Stale
	return result, nil
}

// Begin reserves the lowest free page. On refusal it returns the original
// value and a zero Reservation, preserving caller-visible state.
func (table Table) Begin(logical int64) (Table, Reservation, error) {
	return table.BeginExcluding(logical, nil)
}

// BeginExcluding reserves the lowest free slot not excluded by the caller's
// current allocation fence. A zero-length fence is equivalent to Begin.
func (table Table) BeginExcluding(logical int64, excluded []bool) (Table, Reservation, error) {
	if !table.configured() {
		return table, Reservation{}, ErrInvalidGeometry
	}
	if logical < 0 || logical >= table.logicalCount {
		return table, Reservation{}, ErrInvalidLogicalPage
	}
	if table.pending != nil {
		return table, Reservation{}, ErrReservationPending
	}
	if len(excluded) != 0 && len(excluded) != len(table.physical) {
		return table, Reservation{}, ErrInvalidFence
	}
	physical := -1
	for index, status := range table.physical {
		if status == Free && (len(excluded) == 0 || !excluded[index]) {
			physical = index
			break
		}
	}
	if physical < 0 {
		return table, Reservation{}, ErrNoFreePage
	}

	result := table.clone()
	reservation := Reservation{Logical: logical, Physical: int64(physical)}
	result.physical[physical] = Reserved
	result.pending = &reservation
	return result, reservation, nil
}

// ReleaseErased makes a completed media extent reusable when every slot is
// already free or stale. Media alignment and completion belong to the caller.
func (table Table) ReleaseErased(firstPhysical, pageCount int64) (Table, error) {
	if !table.configured() {
		return table, ErrInvalidGeometry
	}
	if firstPhysical < 0 || firstPhysical >= table.physicalCount || pageCount <= 0 ||
		pageCount > table.physicalCount-firstPhysical {
		return table, ErrInvalidExtent
	}
	end := firstPhysical + pageCount
	for index := firstPhysical; index < end; index++ {
		status := table.physical[int(index)]
		if status != Free && status != Stale {
			return table, ErrIneligibleExtent
		}
	}
	result := table.clone()
	for index := firstPhysical; index < end; index++ {
		result.physical[int(index)] = Free
	}
	return result, nil
}

// Finish resolves the pending slot. A ready write publishes the new mapping
// and invalidates the previous live slot atomically. Abort dispositions keep
// the previous mapping untouched.
func (table Table) Finish(disposition Disposition) (Table, error) {
	if !table.configured() {
		return table, ErrInvalidGeometry
	}
	if table.pending == nil {
		return table, ErrNoReservation
	}
	if disposition > AbortSpent {
		return table, ErrInvalidDisposition
	}

	result := table.clone()
	reservation := *result.pending
	result.pending = nil
	slot := int(reservation.Physical)
	switch disposition {
	case MediaReady:
		previous := result.logicalMap[int(reservation.Logical)]
		if previous != noMapping {
			result.physical[int(previous)] = Stale
		}
		result.logicalMap[int(reservation.Logical)] = reservation.Physical
		result.physical[slot] = Live
	case AbortFresh:
		result.physical[slot] = Free
	case AbortSpent:
		result.physical[slot] = Stale
	}
	return result, nil
}

// Snapshot returns fresh arrays and a copied reservation pointer, so editing
// the snapshot never changes the table.
func (table Table) Snapshot() Snapshot {
	if !table.configured() {
		return Snapshot{}
	}
	snapshot := Snapshot{
		LogicalToPhysical: append([]int64(nil), table.logicalMap...),
		PhysicalStatus:    append([]PhysicalStatus(nil), table.physical...),
	}
	if table.pending != nil {
		reservation := *table.pending
		snapshot.Reservation = &reservation
	}
	return snapshot
}

func (table Table) clone() Table {
	result := Table{
		logicalCount:  table.logicalCount,
		physicalCount: table.physicalCount,
		logicalMap:    append([]int64(nil), table.logicalMap...),
		physical:      append([]PhysicalStatus(nil), table.physical...),
	}
	if table.pending != nil {
		reservation := *table.pending
		result.pending = &reservation
	}
	return result
}
