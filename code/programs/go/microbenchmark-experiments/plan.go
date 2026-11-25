package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"math"

	mb "github.com/adhithyan15/coding-adventures/code/packages/go/microbenchmark"
)

// Plan fixes every comparison before collection. Small explicit limits bound
// memory, total cases and checksum arithmetic independently of machine speed.
type Plan struct {
	Version       int       `json:"version"`
	Config        mb.Config `json:"config"`
	MemoryBytes   []int     `json:"memory_bytes"`
	GrowthLengths []int     `json:"growth_lengths"`
	Seeds         []int64   `json:"seeds"`
}

// Only two schema objects occur in a plan. Let the standard decoder validate
// syntax; visit their raw members before duplicate names can disappear in maps.
func planObject(data []byte, keys []string) (map[string]json.RawMessage, error) {
	d := json.NewDecoder(bytes.NewReader(data))
	first, err := d.Token()
	if err != nil || first != json.Delim('{') {
		return nil, fmt.Errorf("expected plan object")
	}
	allowed := map[string]bool{}
	for _, k := range keys {
		allowed[k] = true
	}
	result := map[string]json.RawMessage{}
	for d.More() {
		token, err := d.Token()
		if err != nil {
			return nil, err
		}
		key, ok := token.(string)
		if !ok || !allowed[key] {
			return nil, fmt.Errorf("unknown plan key %q", key)
		}
		if _, ok := result[key]; ok {
			return nil, fmt.Errorf("duplicate plan key %q", key)
		}
		var raw json.RawMessage
		if err := d.Decode(&raw); err != nil {
			return nil, err
		}
		if bytes.Equal(bytes.TrimSpace(raw), []byte("null")) {
			return nil, fmt.Errorf("null plan field %s", key)
		}
		result[key] = raw
	}
	if _, err := d.Token(); err != nil {
		return nil, err
	}
	if _, err := d.Token(); err != io.EOF {
		return nil, fmt.Errorf("trailing plan data")
	}
	for _, k := range keys {
		if _, ok := result[k]; !ok {
			return nil, fmt.Errorf("missing plan field %s", k)
		}
	}
	return result, nil
}

func parsePlan(data []byte) (Plan, error) {
	var p Plan
	if len(data) > 65536 {
		return p, fmt.Errorf("plan exceeds 64 KiB")
	}
	root, err := planObject(data, []string{"version", "config", "memory_bytes", "growth_lengths", "seeds"})
	if err != nil {
		return p, err
	}
	if _, err := planObject(root["config"], []string{"pairs", "confidence_percent", "minimum_factor", "seed", "target_ns", "max_batch"}); err != nil {
		return p, err
	}
	if err := json.Unmarshal(data, &p); err != nil {
		return p, err
	}
	c := p.Config
	if p.Version != 1 || c.Pairs < 1 || c.Pairs > 10000 || c.ConfidencePercent < 1 || c.ConfidencePercent > 99 || math.IsNaN(c.MinimumFactor) || math.IsInf(c.MinimumFactor, 0) || c.MinimumFactor <= 1 || c.TargetNS <= 0 || c.MaxBatch < 1 || c.MaxBatch > 1048576 {
		return p, fmt.Errorf("invalid plan version or configuration")
	}
	if len(p.MemoryBytes) > 32 || len(p.GrowthLengths) > 32 || len(p.Seeds) < 1 || len(p.Seeds) > 16 || len(p.Seeds)*(2*len(p.MemoryBytes)+len(p.GrowthLengths)) < 1 || len(p.Seeds)*(2*len(p.MemoryBytes)+len(p.GrowthLengths)) > 128 {
		return p, fmt.Errorf("plan must contain 1 through 128 cases")
	}
	for _, group := range []struct {
		values   []int
		min, max int
	}{{p.MemoryBytes, 16, 32 << 20}, {p.GrowthLengths, 1, 32768}} {
		seen := map[int]bool{}
		for _, n := range group.values {
			if n < group.min || n > group.max || seen[n] {
				return p, fmt.Errorf("invalid or repeated workload size %d", n)
			}
			seen[n] = true
		}
	}
	seen := map[int64]bool{}
	for _, seed := range p.Seeds {
		if seen[seed] {
			return p, fmt.Errorf("duplicate seed")
		}
		seen[seed] = true
	}
	return p, nil
}
