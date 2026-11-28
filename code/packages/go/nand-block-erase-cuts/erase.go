package nandblockerasecuts

import "errors"

const (
	maxPagesPerBlock = 256
	maxPageSize      = 65_536
	maxSafeErase     = uint64(9_007_199_254_740_991)
)

// Each sentinel is a stable typed refusal. Callers can use errors.Is without
// depending on how the block transition is represented internally.
var (
	ErrInvalidPageSize   = errors.New("page size outside supported range")
	ErrInvalidBlock      = errors.New("invalid NAND block state")
	ErrInvalidEraseCount = errors.New("invalid completed erase count or ceiling")
	ErrInvalidCut        = errors.New("invalid NAND erase cut")
	ErrEraseLimit        = errors.New("block erase limit reached")
)

// Page separates the bytes on a page from the program marker. An erased page
// has a clear marker and contains only 0xff bytes.
type Page struct {
	Bytes      []byte
	Programmed bool
}

// Block is one independently modeled erase unit. The pages, counter, and
// incomplete flag are copied into every successful outcome.
type Block struct {
	Pages           []Page
	CompletedErases uint64
	IncompleteErase bool
}

// CutKind names the points at which an erase command can be interrupted.
type CutKind uint8

const (
	BeforeAccept CutKind = iota
	AfterPrefix
	AfterMedia
	Complete
)

// Cut selects work progress. PrefixPages is meaningful only for AfterPrefix.
type Cut struct {
	Kind        CutKind
	PrefixPages int
}

// Outcome owns a detached post-command block and reports caller acknowledgement.
type Outcome struct {
	Block        Block
	Acknowledged bool
}

// Apply computes one interrupted or completed erase command for one block.
// The caller retains ownership of block and must keep it stable during the call.
// A refusal returns the zero Outcome and one stable domain error.
func Apply(pageSize int, ceiling uint64, block Block, cut Cut) (Outcome, error) {
	if pageSize < 1 || pageSize > maxPageSize {
		return Outcome{}, ErrInvalidPageSize
	}
	if !validBlock(pageSize, block) {
		return Outcome{}, ErrInvalidBlock
	}
	if ceiling > maxSafeErase || block.CompletedErases > ceiling {
		return Outcome{}, ErrInvalidEraseCount
	}
	if !validCut(cut, len(block.Pages)) {
		return Outcome{}, ErrInvalidCut
	}
	if block.CompletedErases >= ceiling {
		return Outcome{}, ErrEraseLimit
	}

	result := cloneBlock(block)
	acknowledged := false
	switch cut.Kind {
	case BeforeAccept:
		// Power failed before acceptance, so even an older incomplete marker stays.
	case AfterPrefix:
		for index := 0; index < cut.PrefixPages; index++ {
			fillErased(result.Pages[index].Bytes)
			result.Pages[index].Programmed = false
		}
		result.IncompleteErase = true
	case AfterMedia, Complete:
		eraseEveryPage(&result)
		result.CompletedErases++
		result.IncompleteErase = false
		acknowledged = cut.Kind == Complete
	}

	return Outcome{Block: result, Acknowledged: acknowledged}, nil
}

func validBlock(pageSize int, block Block) bool {
	if len(block.Pages) < 1 || len(block.Pages) > maxPagesPerBlock {
		return false
	}
	for _, page := range block.Pages {
		if len(page.Bytes) != pageSize {
			return false
		}
		if !page.Programmed && !isErased(page.Bytes) {
			return false
		}
	}
	return true
}

func validCut(cut Cut, pageCount int) bool {
	switch cut.Kind {
	case BeforeAccept, AfterMedia, Complete:
		return cut.PrefixPages == 0
	case AfterPrefix:
		return cut.PrefixPages >= 0 && cut.PrefixPages < pageCount
	default:
		return false
	}
}

func cloneBlock(block Block) Block {
	clone := Block{
		Pages:           make([]Page, len(block.Pages)),
		CompletedErases: block.CompletedErases,
		IncompleteErase: block.IncompleteErase,
	}
	for index, page := range block.Pages {
		clone.Pages[index] = Page{
			Bytes:      append([]byte(nil), page.Bytes...),
			Programmed: page.Programmed,
		}
	}
	return clone
}

func eraseEveryPage(block *Block) {
	for index := range block.Pages {
		fillErased(block.Pages[index].Bytes)
		block.Pages[index].Programmed = false
	}
}

func fillErased(page []byte) {
	for index := range page {
		page[index] = 0xff
	}
}

func isErased(page []byte) bool {
	for _, value := range page {
		if value != 0xff {
			return false
		}
	}
	return true
}
