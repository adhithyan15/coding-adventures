package recmerge

import (
	"hash/fnv"
	"sort"
)

// causeSet holds the causes of a void in bounded memory: the exact number of
// DISTINCT causes (as 64-bit hashes, 8 bytes each rather than the text) and
// only the smallest MaxCauses of them as strings.  "The smallest k of a
// stream" does not depend on arrival order, so the result is deterministic.
type causeSet struct {
	seen map[uint64]struct{}
	top  []string
}

func newCauseSet() *causeSet { return &causeSet{seen: map[uint64]struct{}{}} }

func (c *causeSet) add(s string) {
	h := fnv.New64a()
	_, _ = h.Write([]byte(s))
	if _, dup := c.seen[h.Sum64()]; dup {
		return
	}
	c.seen[h.Sum64()] = struct{}{}
	i := sort.SearchStrings(c.top, s)
	if i >= MaxCauses {
		return
	}
	c.top = append(c.top, "")
	copy(c.top[i+1:], c.top[i:])
	c.top[i] = s
	if len(c.top) > MaxCauses {
		c.top = c.top[:MaxCauses]
	}
}

func (c *causeSet) count() int { return len(c.seen) }

func (c *causeSet) list() []string { return append([]string{}, c.top...) }
