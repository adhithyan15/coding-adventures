package riscvsimulator

const (
	SyscallWriteByte = 1
	SyscallReadByte  = 2
	SyscallExit      = 10
)

// HostIO provides tiny host services for source-language end-to-end tests.
//
// It is intentionally byte-oriented because the first users are Brainfuck and
// other small compiler pipelines that only need stdin/stdout plus exit status.
type HostIO struct {
	Input  []byte
	Output []byte

	inputOffset int
	Exited      bool
	ExitCode    uint32
}

func NewHostIO(input []byte) *HostIO {
	copied := append([]byte(nil), input...)
	return &HostIO{Input: copied}
}

func (h *HostIO) ReadByte() byte {
	if h == nil || h.inputOffset >= len(h.Input) {
		return 0
	}
	value := h.Input[h.inputOffset]
	h.inputOffset++
	return value
}

func (h *HostIO) WriteByte(value byte) {
	if h == nil {
		return
	}
	h.Output = append(h.Output, value)
}

func (h *HostIO) OutputString() string {
	if h == nil {
		return ""
	}
	return string(h.Output)
}
