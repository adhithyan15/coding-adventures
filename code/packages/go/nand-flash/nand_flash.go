// Package nandflash models the logical behavior of full-page, single-level-cell
// NAND media. It is an in-memory teaching model, not a device controller.
package nandflash

import (
	"encoding/hex"
	"errors"
)

const (
	maxBlocks          = 1024
	maxPagesPerBlock   = 256
	maxPageSizeBytes   = 65536
	maxStorageBytes    = 16 * 1024 * 1024
	maxSafeEraseCycles = uint64(9007199254740991)
)

// Each sentinel is both the Go error identity and the portable DB01 message.
// Callers can use errors.Is without depending on implementation details.
var (
	ErrInvalidArgument = errors.New("invalid API argument")
	ErrGeometry        = errors.New("geometry values are outside supported bounds")
	ErrBlock           = errors.New("block index out of range")
	ErrPage            = errors.New("page index out of range")
	ErrPayload         = errors.New("page data must be bytes")
	ErrPayloadSize     = errors.New("page data must match page_size_bytes")
	ErrProgrammed      = errors.New("page has already been programmed since erase")
	ErrEraseLimit      = errors.New("block erase limit reached")
)

// Geometry is copied into a Device when New succeeds. Its positive fields are
// read-only thereafter; Geometry() returns another value copy.
type Geometry struct {
	Blocks        int
	PagesPerBlock int
	PageSizeBytes int
}

// Config carries the geometry and optional per-block erase budget. New copies
// the pointed-to budget and never retains caller-owned configuration memory.
type Config struct {
	Geometry       Geometry
	MaxEraseCycles *uint64
}

// Device owns flattened page bytes, a program-once marker per page, and a
// successful-erase counter per block. Callers must serialize operations on one
// Device; independent devices share no state. Device must not be copied.
type Device struct {
	geometry       Geometry
	pages          []byte
	programmed     []bool
	eraseCounts    []uint64
	hasEraseLimit  bool
	maxEraseCycles uint64
}

// Snapshot is a detached JSON-compatible view. Page data is lowercase hex so
// JSON has an unambiguous portable spelling for each byte.
type Snapshot struct {
	Pages       [][]string `json:"pages"`
	EraseCounts []uint64   `json:"erase_counts"`
	Programmed  [][]bool   `json:"programmed"`
}

// New validates every dimension before multiplication. The first product is
// bounded by 1024*256, while the division check proves the storage product is
// at most 16 MiB before it is formed.
func New(config Config) (*Device, error) {
	g := config.Geometry
	if g.Blocks < 1 || g.Blocks > maxBlocks ||
		g.PagesPerBlock < 1 || g.PagesPerBlock > maxPagesPerBlock ||
		g.PageSizeBytes < 1 || g.PageSizeBytes > maxPageSizeBytes {
		return nil, ErrGeometry
	}

	pageCount := g.Blocks * g.PagesPerBlock
	if pageCount > maxStorageBytes/g.PageSizeBytes {
		return nil, ErrGeometry
	}
	storageBytes := pageCount * g.PageSizeBytes
	hasEraseLimit := config.MaxEraseCycles != nil
	maxEraseCycles := uint64(0)
	if hasEraseLimit {
		maxEraseCycles = *config.MaxEraseCycles
		if maxEraseCycles > maxSafeEraseCycles {
			return nil, ErrGeometry
		}
	}

	d := &Device{
		geometry:       g,
		pages:          make([]byte, storageBytes),
		programmed:     make([]bool, pageCount),
		eraseCounts:    make([]uint64, g.Blocks),
		hasEraseLimit:  hasEraseLimit,
		maxEraseCycles: maxEraseCycles,
	}
	for index := range d.pages {
		d.pages[index] = 0xFF
	}
	return d, nil
}

// valid rejects nil and zero-value devices before any address or buffer check.
func (d *Device) valid() bool {
	return d != nil && d.pages != nil
}

// Geometry returns a value copy, never a mutable view into the device.
func (d *Device) Geometry() (Geometry, error) {
	if !d.valid() {
		return Geometry{}, ErrInvalidArgument
	}
	return d.geometry, nil
}

// pageIndex checks block before page and linearizes only values already known
// to fit the constructor-validated geometry.
func (d *Device) pageIndex(block, page int) (int, error) {
	if !d.valid() {
		return 0, ErrInvalidArgument
	}
	if block < 0 || block >= d.geometry.Blocks {
		return 0, ErrBlock
	}
	if page < 0 || page >= d.geometry.PagesPerBlock {
		return 0, ErrPage
	}
	return block*d.geometry.PagesPerBlock + page, nil
}

// Read returns a new slice for every successful call, so changing a result
// cannot rewrite the device's page bytes.
func (d *Device) Read(block, page int) ([]byte, error) {
	index, err := d.pageIndex(block, page)
	if err != nil {
		return nil, err
	}
	start := index * d.geometry.PageSizeBytes
	end := start + d.geometry.PageSizeBytes
	result := make([]byte, d.geometry.PageSizeBytes)
	copy(result, d.pages[start:end])
	return result, nil
}

// Program snapshots caller bytes after validating the address, nilness and
// exact page length. The marker is checked last, so refusal precedence is
// stable and the copied bytes are exactly the bytes stored on success.
func (d *Device) Program(block, page int, data []byte) error {
	index, err := d.pageIndex(block, page)
	if err != nil {
		return err
	}
	if data == nil {
		return ErrPayload
	}
	if len(data) != d.geometry.PageSizeBytes {
		return ErrPayloadSize
	}
	if d.programmed[index] {
		return ErrProgrammed
	}
	start := index * d.geometry.PageSizeBytes
	copy(d.pages[start:start+d.geometry.PageSizeBytes], data)
	d.programmed[index] = true
	return nil
}

// EraseBlock resets every page and marker in one block, then increments that
// block's counter. The checks finish before mutation, so a refused erase is
// observationally atomic.
func (d *Device) EraseBlock(block int) error {
	if !d.valid() {
		return ErrInvalidArgument
	}
	if block < 0 || block >= d.geometry.Blocks {
		return ErrBlock
	}
	if (d.hasEraseLimit && d.eraseCounts[block] >= d.maxEraseCycles) ||
		d.eraseCounts[block] >= maxSafeEraseCycles {
		return ErrEraseLimit
	}
	firstPage := block * d.geometry.PagesPerBlock
	firstByte := firstPage * d.geometry.PageSizeBytes
	blockBytes := d.geometry.PagesPerBlock * d.geometry.PageSizeBytes
	for index := firstByte; index < firstByte+blockBytes; index++ {
		d.pages[index] = 0xFF
	}
	for index := firstPage; index < firstPage+d.geometry.PagesPerBlock; index++ {
		d.programmed[index] = false
	}
	d.eraseCounts[block]++
	return nil
}

// EraseCount returns one counter by value; no internal slice is exposed.
func (d *Device) EraseCount(block int) (uint64, error) {
	if !d.valid() {
		return 0, ErrInvalidArgument
	}
	if block < 0 || block >= d.geometry.Blocks {
		return 0, ErrBlock
	}
	return d.eraseCounts[block], nil
}

// IsProgrammed reports the page marker without exposing the marker slice.
func (d *Device) IsProgrammed(block, page int) (bool, error) {
	index, err := d.pageIndex(block, page)
	if err != nil {
		return false, err
	}
	return d.programmed[index], nil
}

// Snapshot allocates every nested collection anew and encodes each complete
// page. A snapshot can therefore be changed independently of future snapshots.
func (d *Device) Snapshot() (Snapshot, error) {
	if !d.valid() {
		return Snapshot{}, ErrInvalidArgument
	}
	snapshot := Snapshot{
		Pages:       make([][]string, d.geometry.Blocks),
		EraseCounts: append([]uint64(nil), d.eraseCounts...),
		Programmed:  make([][]bool, d.geometry.Blocks),
	}
	for block := 0; block < d.geometry.Blocks; block++ {
		snapshot.Pages[block] = make([]string, d.geometry.PagesPerBlock)
		snapshot.Programmed[block] = make([]bool, d.geometry.PagesPerBlock)
		for page := 0; page < d.geometry.PagesPerBlock; page++ {
			index := block*d.geometry.PagesPerBlock + page
			start := index * d.geometry.PageSizeBytes
			end := start + d.geometry.PageSizeBytes
			snapshot.Pages[block][page] = hex.EncodeToString(d.pages[start:end])
			snapshot.Programmed[block][page] = d.programmed[index]
		}
	}
	return snapshot, nil
}
