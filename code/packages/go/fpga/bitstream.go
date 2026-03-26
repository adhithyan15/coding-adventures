package fpga

// =========================================================================
// Bitstream — FPGA Configuration Data
// =========================================================================
//
// In a real FPGA, a bitstream is a binary blob that programs every
// configurable element: LUT truth tables, flip-flop enables, carry chain
// enables, routing switch states, I/O pad modes, and Block RAM contents.
//
// The bitstream is loaded at power-up (or during runtime for partial
// reconfiguration) and writes to the SRAM cells that control the fabric.
//
// Instead of a binary format, we use JSON-like maps for readability
// and education. The configuration specifies:
//
//  1. CLBs: Which LUTs get which truth tables, FF enables, carry enables
//  2. Routing: Which switch matrix ports are connected
//  3. I/O: Pin names, modes, and mappings

import (
	"encoding/json"
)

// SliceConfig holds the configuration for one slice.
type SliceConfig struct {
	LutA         []int `json:"lut_a"`
	LutB         []int `json:"lut_b"`
	FFAEnabled   bool  `json:"ff_a"`
	FFBEnabled   bool  `json:"ff_b"`
	CarryEnabled bool  `json:"carry"`
}

// CLBConfig holds the configuration for one CLB (2 slices).
type CLBConfig struct {
	Slice0 SliceConfig `json:"slice0"`
	Slice1 SliceConfig `json:"slice1"`
}

// RouteConfig holds a single routing connection.
type RouteConfig struct {
	Source      string `json:"src"`
	Destination string `json:"dst"`
}

// IOConfig holds the configuration for one I/O block.
type IOConfig struct {
	Mode string `json:"mode"`
}

// Bitstream is the FPGA configuration data — the "program" for the fabric.
type Bitstream struct {
	CLBs    map[string]CLBConfig       `json:"clbs"`
	Routing map[string][]RouteConfig   `json:"routing"`
	IO      map[string]IOConfig        `json:"io"`
	LutK    int                        `json:"lut_k"`
}

// FromJSON loads a bitstream from a JSON file.
func FromJSON(path string) (*Bitstream, error) {
	return StartNew[*Bitstream]("fpga.FromJSON", nil,
		func(op *Operation[*Bitstream], rf *ResultFactory[*Bitstream]) *OperationResult[*Bitstream] {
			data, err := op.File.ReadFile(path)
			if err != nil {
				return rf.Fail(nil, err)
			}
			bs, err := FromJSONBytes(data)
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, bs)
		}).GetResult()
}

// FromJSONBytes parses a bitstream from JSON bytes.
func FromJSONBytes(data []byte) (*Bitstream, error) {
	var raw struct {
		CLBs    map[string]json.RawMessage `json:"clbs"`
		Routing map[string][]RouteConfig   `json:"routing"`
		IO      map[string]IOConfig        `json:"io"`
		LutK    int                        `json:"lut_k"`
	}

	if err := json.Unmarshal(data, &raw); err != nil {
		return nil, err
	}

	lutK := raw.LutK
	if lutK == 0 {
		lutK = 4
	}

	bs := &Bitstream{
		CLBs:    make(map[string]CLBConfig),
		Routing: raw.Routing,
		IO:      raw.IO,
		LutK:    lutK,
	}

	if bs.Routing == nil {
		bs.Routing = make(map[string][]RouteConfig)
	}
	if bs.IO == nil {
		bs.IO = make(map[string]IOConfig)
	}

	defaultTT := make([]int, 1<<lutK)

	for name, rawCLB := range raw.CLBs {
		var clbData struct {
			Slice0 *SliceConfig `json:"slice0"`
			Slice1 *SliceConfig `json:"slice1"`
		}
		if err := json.Unmarshal(rawCLB, &clbData); err != nil {
			return nil, err
		}

		s0 := SliceConfig{LutA: defaultTT, LutB: defaultTT}
		if clbData.Slice0 != nil {
			s0 = *clbData.Slice0
			if s0.LutA == nil {
				s0.LutA = defaultTT
			}
			if s0.LutB == nil {
				s0.LutB = defaultTT
			}
		}

		s1 := SliceConfig{LutA: defaultTT, LutB: defaultTT}
		if clbData.Slice1 != nil {
			s1 = *clbData.Slice1
			if s1.LutA == nil {
				s1.LutA = defaultTT
			}
			if s1.LutB == nil {
				s1.LutB = defaultTT
			}
		}

		bs.CLBs[name] = CLBConfig{Slice0: s0, Slice1: s1}
	}

	return bs, nil
}

// FromMap creates a Bitstream from a structured map.
//
// This is a convenience for creating bitstreams programmatically in Go
// without going through JSON serialization.
func FromMap(clbs map[string]CLBConfig, routing map[string][]RouteConfig, io map[string]IOConfig, lutK int) *Bitstream {
	if lutK == 0 {
		lutK = 4
	}
	if clbs == nil {
		clbs = make(map[string]CLBConfig)
	}
	if routing == nil {
		routing = make(map[string][]RouteConfig)
	}
	if io == nil {
		io = make(map[string]IOConfig)
	}

	return &Bitstream{
		CLBs:    clbs,
		Routing: routing,
		IO:      io,
		LutK:    lutK,
	}
}
