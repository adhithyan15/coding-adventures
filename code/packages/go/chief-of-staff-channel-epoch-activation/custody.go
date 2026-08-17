package epochactivation

import (
	"crypto/subtle"

	channelcrypto "github.com/adhithyan15/coding-adventures/code/packages/go/chief-of-staff-channel-crypto"
)

// CustodySelection is the three-valued result of an atomic custody claim.
//
// Three values, not two, and the distinction is the heart of D18T. "Selected"
// and "idempotent" are both successes but mean different things: the first says
// *you* won the slot, the second says you are retrying something already won
// with byte-identical inputs. "Conflict" means somebody else owns it and you
// must not proceed -- notably, you may not look at what they stored.
type CustodySelection string

const (
	// CustodySelected means the slot was empty and this bundle now owns it.
	CustodySelected CustodySelection = "selected"
	// CustodyIdempotent means an identical bundle already owns the slot.
	CustodyIdempotent CustodySelection = "idempotent"
	// CustodyConflict means a different bundle owns the slot. Fail closed.
	CustodyConflict CustodySelection = "conflict"
)

// CustodyError is a secret-free failure from the injected custody backend.
type CustodyError struct{}

func (*CustodyError) Error() string { return string(ErrCustody) }

// Code reports the stable D18T error code.
func (*CustodyError) Code() ErrorCode { return ErrCustody }

// EpochKeyHandle is an opaque, redacted reference to one retained epoch key.
//
// It carries no key bytes and no reversible locator -- only the channel and
// epoch, both already public. Resolving a handle to an actual CMK is the sole
// privilege of the originator encryption boundary, via WithKey.
type EpochKeyHandle struct {
	channelID []byte
	epoch     uint64
}

// NewEpochKeyHandle builds a handle over an owned copy of the channel ID.
func NewEpochKeyHandle(channelID []byte, epoch uint64) EpochKeyHandle {
	return EpochKeyHandle{channelID: append([]byte(nil), channelID...), epoch: epoch}
}

// ChannelID returns a defensive copy.
func (h EpochKeyHandle) ChannelID() []byte { return append([]byte(nil), h.channelID...) }

// Epoch reports the epoch this handle refers to.
func (h EpochKeyHandle) Epoch() uint64 { return h.epoch }

// String redacts. Handles end up in logs and debug output; a handle that
// printed its channel and epoch would be harmless today and a liability the
// moment somebody adds a field to this struct.
func (h EpochKeyHandle) String() string { return "EpochKeyHandle([REDACTED])" }

// GoString redacts under %#v as well.
func (h EpochKeyHandle) GoString() string { return h.String() }

// PublicPreparation is the exact secret-free recovery bundle retained beside a
// prepared CMK. After any crash, this is enough to replay every public write
// without regenerating a single byte.
type PublicPreparation struct {
	channelID []byte
	baseEpoch uint64
	newEpoch  uint64
	planBytes []byte
	grants    [][]byte
}

// NewPublicPreparation takes ownership by copying every input.
func NewPublicPreparation(channelID []byte, baseEpoch, newEpoch uint64, planBytes []byte, grants [][]byte) PublicPreparation {
	copiedGrants := make([][]byte, len(grants))
	for index, grant := range grants {
		copiedGrants[index] = append([]byte(nil), grant...)
	}
	return PublicPreparation{
		channelID: append([]byte(nil), channelID...),
		baseEpoch: baseEpoch,
		newEpoch:  newEpoch,
		planBytes: append([]byte(nil), planBytes...),
		grants:    copiedGrants,
	}
}

// ChannelID returns a defensive copy.
func (p PublicPreparation) ChannelID() []byte { return append([]byte(nil), p.channelID...) }

// BaseEpoch reports the epoch being rotated away from.
func (p PublicPreparation) BaseEpoch() uint64 { return p.baseEpoch }

// NewEpoch reports the candidate epoch.
func (p PublicPreparation) NewEpoch() uint64 { return p.newEpoch }

// PlanBytes returns a defensive copy of the exact D18T record.
func (p PublicPreparation) PlanBytes() []byte { return append([]byte(nil), p.planBytes...) }

// Grants returns defensive copies of the exact D18G records in D18Q order.
func (p PublicPreparation) Grants() [][]byte {
	out := make([][]byte, len(p.grants))
	for index, grant := range p.grants {
		out[index] = append([]byte(nil), grant...)
	}
	return out
}

// Clone deep-copies the bundle.
func (p PublicPreparation) Clone() PublicPreparation {
	return NewPublicPreparation(p.channelID, p.baseEpoch, p.newEpoch, p.planBytes, p.grants)
}

// Equal compares every public byte.
func (p PublicPreparation) Equal(other PublicPreparation) bool {
	if !bytesEqual(p.channelID, other.channelID) ||
		p.baseEpoch != other.baseEpoch ||
		p.newEpoch != other.newEpoch ||
		!bytesEqual(p.planBytes, other.planBytes) ||
		len(p.grants) != len(other.grants) {
		return false
	}
	for index := range p.grants {
		if !bytesEqual(p.grants[index], other.grants[index]) {
			return false
		}
	}
	return true
}

// PreparedEpoch is one indivisible candidate offered to custody: the public
// recovery bundle *and* the secret CMK, together.
//
// "Indivisible" is the whole point. Custody must never store the plan without
// the key or the key without the plan -- either half alone leaves a channel
// that cannot recover. That is why this is a single type with a single
// custody entry point rather than two calls a caller could interleave.
type PreparedEpoch struct {
	public PublicPreparation
	cmk    *channelcrypto.ChannelMasterKey
}

// NewPreparedEpoch clones both halves so the caller may destroy its own copies.
func NewPreparedEpoch(public PublicPreparation, cmk *channelcrypto.ChannelMasterKey) (*PreparedEpoch, error) {
	cloned, err := cloneCMK(cmk)
	if err != nil {
		return nil, err
	}
	return &PreparedEpoch{public: public.Clone(), cmk: cloned}, nil
}

// PublicPreparation returns the secret-free half.
func (p *PreparedEpoch) PublicPreparation() PublicPreparation { return p.public.Clone() }

// CloneCMK yields an independently destroyable copy of the secret.
func (p *PreparedEpoch) CloneCMK() (*channelcrypto.ChannelMasterKey, error) { return cloneCMK(p.cmk) }

// Destroy releases the owned secret.
func (p *PreparedEpoch) Destroy() {
	if p.cmk != nil {
		p.cmk.Destroy()
	}
}

// String redacts.
func (p *PreparedEpoch) String() string { return "PreparedEpoch([REDACTED])" }

// GoString redacts under %#v as well.
func (p *PreparedEpoch) GoString() string { return p.String() }

// OriginatorKeyCustody is the injected atomic secret boundary.
//
// A production implementation MUST survive process and machine restart. The
// Durable method is how an implementation declares that honestly; the
// production constructor refuses anything that answers false, so a test double
// cannot be wired into a real channel by accident.
type OriginatorKeyCustody interface {
	// Durable reports whether this custody survives restart.
	Durable() bool

	// ImportActiveIfAbsent atomically owns a copy of exactly 32 CMK octets for
	// an already-active epoch. Used only at channel creation and at version 1
	// migration -- never to invent a key.
	ImportActiveIfAbsent(channelID []byte, epoch uint64, cmk *channelcrypto.ChannelMasterKey) (CustodySelection, error)

	// ResolveHandle returns an opaque handle, or nil when the epoch is absent.
	// It never returns key bytes.
	ResolveHandle(channelID []byte, epoch uint64) (*EpochKeyHandle, error)

	// PrepareIfAbsent atomically claims (channel, new_epoch) for one complete
	// bundle. This is the single selection point of the whole protocol.
	PrepareIfAbsent(prepared *PreparedEpoch) (CustodySelection, error)

	// LoadPreparation returns the durable recovery bundle, or nil when absent.
	LoadPreparation(channelID []byte, newEpoch uint64) (*PublicPreparation, error)

	// WithKey resolves a handle to a transient CMK for exactly one operation.
	WithKey(handle EpochKeyHandle, operation func(*channelcrypto.ChannelMasterKey) error) error

	// DestroyChannel applies the configured logical erasure policy.
	DestroyChannel(channelID []byte) error
}

// InMemoryKeyCustody is deterministic, explicitly non-durable custody for
// conformance tests. Durable() returns false, so NewStore refuses it and only
// NewStoreForTesting will accept it.
type InMemoryKeyCustody struct {
	keys         map[custodySlot]*channelcrypto.ChannelMasterKey
	preparations map[custodySlot]PublicPreparation
}

type custodySlot struct {
	channel string
	epoch   uint64
}

// NewInMemoryKeyCustody builds empty non-durable custody.
func NewInMemoryKeyCustody() *InMemoryKeyCustody {
	return &InMemoryKeyCustody{
		keys:         map[custodySlot]*channelcrypto.ChannelMasterKey{},
		preparations: map[custodySlot]PublicPreparation{},
	}
}

// Durable reports false. Tests that need a production-shaped custody embed
// this type and override the method.
func (*InMemoryKeyCustody) Durable() bool { return false }

// ImportActiveIfAbsent claims an already-active epoch key.
func (c *InMemoryKeyCustody) ImportActiveIfAbsent(channelID []byte, epoch uint64, cmk *channelcrypto.ChannelMasterKey) (CustodySelection, error) {
	slot := custodySlot{string(channelID), epoch}
	current, exists := c.keys[slot]
	if !exists {
		cloned, err := cloneCMK(cmk)
		if err != nil {
			return CustodyConflict, err
		}
		c.keys[slot] = cloned
		return CustodySelected, nil
	}
	same, err := sameCMK(current, cmk)
	if err != nil {
		return CustodyConflict, err
	}
	if same {
		return CustodyIdempotent, nil
	}
	// Deliberately does not reveal *how* the stored secret differs.
	return CustodyConflict, nil
}

// ResolveHandle reports whether an epoch key is retained.
func (c *InMemoryKeyCustody) ResolveHandle(channelID []byte, epoch uint64) (*EpochKeyHandle, error) {
	if _, exists := c.keys[custodySlot{string(channelID), epoch}]; !exists {
		return nil, nil
	}
	handle := NewEpochKeyHandle(channelID, epoch)
	return &handle, nil
}

// PrepareIfAbsent atomically claims the epoch slot for one complete bundle.
//
// Both halves are checked before either is written, and a partially-present
// slot (key without bundle, or bundle without key) is a conflict rather than
// something to repair -- a half-written slot means an invariant already broke,
// and guessing at the missing half is exactly the fallback D18T forbids.
func (c *InMemoryKeyCustody) PrepareIfAbsent(prepared *PreparedEpoch) (CustodySelection, error) {
	public := prepared.public
	slot := custodySlot{string(public.channelID), public.newEpoch}
	currentPublic, hasPublic := c.preparations[slot]
	currentCMK, hasCMK := c.keys[slot]

	if !hasPublic && !hasCMK {
		cloned, err := prepared.CloneCMK()
		if err != nil {
			return CustodyConflict, err
		}
		c.preparations[slot] = public.Clone()
		c.keys[slot] = cloned
		return CustodySelected, nil
	}
	if !hasPublic || !hasCMK || !currentPublic.Equal(public) {
		return CustodyConflict, nil
	}
	candidate, err := prepared.CloneCMK()
	if err != nil {
		return CustodyConflict, err
	}
	defer candidate.Destroy()
	same, err := sameCMK(currentCMK, candidate)
	if err != nil {
		return CustodyConflict, err
	}
	if same {
		return CustodyIdempotent, nil
	}
	return CustodyConflict, nil
}

// LoadPreparation returns a copy of the durable recovery bundle.
func (c *InMemoryKeyCustody) LoadPreparation(channelID []byte, newEpoch uint64) (*PublicPreparation, error) {
	preparation, exists := c.preparations[custodySlot{string(channelID), newEpoch}]
	if !exists {
		return nil, nil
	}
	cloned := preparation.Clone()
	return &cloned, nil
}

// WithKey lends a transient CMK for exactly one operation and destroys it after.
func (c *InMemoryKeyCustody) WithKey(handle EpochKeyHandle, operation func(*channelcrypto.ChannelMasterKey) error) error {
	cmk, exists := c.keys[custodySlot{string(handle.channelID), handle.epoch}]
	if !exists {
		return &CustodyError{}
	}
	transient, err := cloneCMK(cmk)
	if err != nil {
		return err
	}
	defer transient.Destroy()
	return operation(transient)
}

// DestroyChannel erases every retained secret for one channel. Public history
// is untouched -- that is the store's business, and D18T keeps it append-only.
func (c *InMemoryKeyCustody) DestroyChannel(channelID []byte) error {
	channel := string(channelID)
	for slot, cmk := range c.keys {
		if slot.channel == channel {
			cmk.Destroy()
			delete(c.keys, slot)
		}
	}
	for slot := range c.preparations {
		if slot.channel == channel {
			delete(c.preparations, slot)
		}
	}
	return nil
}

// RetainedKeyCount reports how many epoch keys are held, for tests.
func (c *InMemoryKeyCustody) RetainedKeyCount() int { return len(c.keys) }

func cloneCMK(cmk *channelcrypto.ChannelMasterKey) (*channelcrypto.ChannelMasterKey, error) {
	if cmk == nil {
		return nil, &CustodyError{}
	}
	raw, err := cmk.Bytes()
	if err != nil {
		return nil, &CustodyError{}
	}
	defer wipe(raw)
	cloned, err := channelcrypto.ChannelMasterKeyFromBytes(raw)
	if err != nil {
		return nil, &CustodyError{}
	}
	return cloned, nil
}

// sameCMK compares in constant time. The comparison is between two secrets, so
// a length-or-content early exit would leak information about the stored key
// to a caller who controls the candidate.
func sameCMK(left, right *channelcrypto.ChannelMasterKey) (bool, error) {
	leftBytes, err := left.Bytes()
	if err != nil {
		return false, &CustodyError{}
	}
	defer wipe(leftBytes)
	rightBytes, err := right.Bytes()
	if err != nil {
		return false, &CustodyError{}
	}
	defer wipe(rightBytes)
	return subtle.ConstantTimeCompare(leftBytes, rightBytes) == 1, nil
}

func wipe(value []byte) {
	for index := range value {
		value[index] = 0
	}
}
