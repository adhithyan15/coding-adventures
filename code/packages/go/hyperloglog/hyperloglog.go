package hyperloglog

import (
	"crypto/sha1"
	"encoding/binary"
	"math"
	"math/bits"
)

const defaultPrecision = 10

type HyperLogLog struct {
	precision uint8
	registers []uint8
}

func New() *HyperLogLog {
	return NewWithPrecision(defaultPrecision)
}

func NewWithPrecision(precision uint8) *HyperLogLog {
	if precision < 4 || precision > 16 {
		precision = defaultPrecision
	}
	return &HyperLogLog{
		precision: precision,
		registers: make([]uint8, 1<<precision),
	}
}

func (h *HyperLogLog) Clone() *HyperLogLog {
	if h == nil {
		return New()
	}
	clone := &HyperLogLog{
		precision: h.precision,
		registers: append([]uint8(nil), h.registers...),
	}
	return clone
}

func (h *HyperLogLog) Add(value []byte) bool {
	if h == nil {
		return false
	}
	hash := hash64(value)
	index := int(hash >> (64 - h.precision))
	w := hash << h.precision
	rank := uint8(bits.LeadingZeros64(w) + 1)
	if rank > h.registers[index] {
		h.registers[index] = rank
		return true
	}
	return false
}

func (h *HyperLogLog) Merge(other *HyperLogLog) {
	if h == nil || other == nil {
		return
	}
	if h.precision != other.precision {
		panic("hyperloglog: precision mismatch")
	}
	for i := range h.registers {
		if other.registers[i] > h.registers[i] {
			h.registers[i] = other.registers[i]
		}
	}
}

func (h *HyperLogLog) Count() uint64 {
	if h == nil {
		return 0
	}
	m := float64(len(h.registers))
	if m == 0 {
		return 0
	}

	var sum float64
	zeros := 0
	for _, register := range h.registers {
		sum += math.Exp2(-float64(register))
		if register == 0 {
			zeros++
		}
	}

	alpha := h.alpha()
	estimate := alpha * m * m / sum

	if estimate <= 2.5*m && zeros > 0 {
		estimate = m * math.Log(m/float64(zeros))
	}

	if estimate < 0 {
		return 0
	}
	return uint64(estimate + 0.5)
}

func (h *HyperLogLog) Equal(other *HyperLogLog) bool {
	if h == nil || other == nil {
		return h == other
	}
	if h.precision != other.precision || len(h.registers) != len(other.registers) {
		return false
	}
	for i := range h.registers {
		if h.registers[i] != other.registers[i] {
			return false
		}
	}
	return true
}

func (h *HyperLogLog) alpha() float64 {
	m := len(h.registers)
	switch m {
	case 16:
		return 0.673
	case 32:
		return 0.697
	case 64:
		return 0.709
	default:
		return 0.7213 / (1 + 1.079/float64(m))
	}
}

func hash64(value []byte) uint64 {
	sum := sha1.Sum(value)
	return binary.BigEndian.Uint64(sum[:8])
}
