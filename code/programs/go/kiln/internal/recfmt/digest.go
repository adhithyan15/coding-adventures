package recfmt

import (
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"sort"
)

// StepsDigest is the digest of a package's BUILD step lines (KILN04 section
// 7.1, G6): SHA-256 of
//
//	u64be(n) || (u64be(len(line)) || line)*
//
// The length prefixes make it injective: ["ab","c"] and ["a","bc"] differ, and
// so do ["a"] and ["a",""].  A shard computes it over the lines its TRACE shows
// were executed; the merge computes it over the lines its own build tool says
// this OS runs; B6 compares them.
func StepsDigest(lines []string) string {
	h := sha256.New()
	var n [8]byte
	binary.BigEndian.PutUint64(n[:], uint64(len(lines)))
	h.Write(n[:])
	for _, l := range lines {
		binary.BigEndian.PutUint64(n[:], uint64(len(l)))
		h.Write(n[:])
		h.Write([]byte(l))
	}
	return hex.EncodeToString(h.Sum(nil))
}

// ShardPlan is one shard of the build plan, as Kiln projects it: the packages
// the shard RUNS (its assigned packages and their prerequisites, which is why
// an action appears in several shards) and the ones it is ASSIGNED.  Both are
// package rel_paths.
type ShardPlan struct {
	Index    int
	Packages []string
	Assigned []string
}

// PartitionDigest is the binding's partition_sha256: SHA-256 of the canonical
// JSON
//
//	[[index, [packages, sorted], [assigned, sorted]], ...]
//
// ordered by index.  It is derived from the plan rather than taken from the
// plan file, so two machines whose plan files differ in an irrelevant byte
// still agree.  A plan that names a package twice in one list is refused: it
// cannot describe a partition.
func PartitionDigest(plan []ShardPlan) (string, error) {
	ps := append([]ShardPlan(nil), plan...)
	sort.Slice(ps, func(a, b int) bool { return ps[a].Index < ps[b].Index })
	var buf []byte
	buf = append(buf, '[')
	for i, p := range ps {
		if i > 0 {
			buf = append(buf, ',')
		}
		if i > 0 && ps[i-1].Index == p.Index {
			return "", fmt.Errorf("partition: shard %d appears twice", p.Index)
		}
		pk, as := sortedCopy(p.Packages), sortedCopy(p.Assigned)
		if err := sortedUnique("partition packages", pk); err != nil {
			return "", err
		}
		if err := sortedUnique("partition assigned", as); err != nil {
			return "", err
		}
		part, err := marshal([]any{p.Index, pk, as})
		if err != nil {
			return "", err
		}
		buf = append(buf, part...)
	}
	buf = append(buf, ']')
	d := sha256.Sum256(buf)
	return hex.EncodeToString(d[:]), nil
}
