package record

// The census side file (KILN04, "Census side file")
// =================================================
//
// `traces/<sha256(action)>/census.json` is a DIAGNOSTIC the classify child
// leaves beside record.json: every path-carrying reason the classifier raised,
// counted BEFORE the 16-per-kind cap hid most of them (traced.Census).  A later
// workflow reads it to histogram the host paths that void recordings.
//
// Its rules, each one a way it could otherwise hurt the build:
//
//	always on       the child always attaches the observer
//	best-effort     any failure to make or write the file is dropped: the
//	                verdict and the child's exit status never depend on it
//	never believed  nothing reads it for a verdict; it is in no shard and no
//	                hashed or sealed data; `collect` and `step` skip its name
//	atomic          written to a temporary name in the same directory, then
//	                rename(2)d over census.json, so a reader sees it whole or
//	                not at all
//	bounded         at most traced.MaxCensusPerKind paths per kind, and the
//	                encoded file at most MaxCensusBytes (entries are shed
//	                round-robin from every kind's tail into that kind's
//	                "overflow" until it fits)
//	no fsync        the rename alone gives whole-or-nothing; a slow sync could
//	                push the child past the parent's deadline and cost the
//	                record.json already linked

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"os"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// CensusSchema is the schema of census.json.
const CensusSchema = "kiln-record-census/1"

// MaxCensusBytes bounds the encoded file.
const MaxCensusBytes = 512 << 10

// CensusFileName is the file's name in an action's trace directory.
const CensusFileName = "census.json"

// censusTmpPrefix starts the temporary name a census is written under; it is
// followed by 16 lowercase hex characters (8 random bytes).
const censusTmpPrefix = CensusFileName + ".tmp-"

// isCensusTmp says name is exactly such a temporary name.
func isCensusTmp(name string) bool {
	rest, ok := strings.CutPrefix(name, censusTmpPrefix)
	if !ok || len(rest) != 16 {
		return false
	}
	for _, r := range rest {
		if !(r >= '0' && r <= '9' || r >= 'a' && r <= 'f') {
			return false
		}
	}
	return true
}

// CensusFile is the path of an action's census.
func CensusFile(dir, action string) string { return StepDir(dir, action) + "/" + CensusFileName }

// CensusDoc is the file's content.  Overflow is per kind (what a bound
// dropped, in occurrences), OverflowTotal its sum.
type CensusDoc struct {
	Schema        string               `json:"schema"`
	Action        string               `json:"action"`
	Total         int64                `json:"total"`
	OverflowTotal int64                `json:"overflow_total"`
	Overflow      map[string]int64     `json:"overflow"`
	Entries       []traced.CensusEntry `json:"entries"`
	// Ops counts, per kind, the occurrences by op class (traced.OpClasses): what
	// kind of look raised each host-read.  Additive: absent when empty.
	Ops map[string]map[string]int64 `json:"ops,omitempty"`
	// ExecHeadTmpfsExecs is traced.Census.TmpfsExecs: the successful execs under
	// the step tmpfs that reached the exec-head pairing.  Additive: absent when 0.
	ExecHeadTmpfsExecs int64 `json:"exec_head_tmpfs_execs,omitempty"`
	// ParseRestartedSpawns and ParseBareOpens are traced.Census.RestartedSpawns
	// and BareOpens: how often the KILN04 D50 parse rules fired.  Additive:
	// absent when 0.
	ParseRestartedSpawns int64 `json:"parse_restarted_spawns,omitempty"`
	ParseBareOpens       int64 `json:"parse_bare_opens,omitempty"`
}

// EncodeCensus renders c, shedding entries until the result is at most
// MaxCensusBytes.  Shedding is FAIR: each round takes a slice off the tail of
// every kind that still has more than one entry, so a kind with a thousand
// paths cannot make a kind with three disappear.  What is shed is counted in
// that kind's overflow.
func EncodeCensus(action string, c *traced.Census) ([]byte, error) {
	doc := CensusDoc{Schema: CensusSchema, Action: action, Total: c.Total, Overflow: map[string]int64{}, ExecHeadTmpfsExecs: c.TmpfsExecs,
		ParseRestartedSpawns: c.RestartedSpawns, ParseBareOpens: c.BareOpens}
	for k, n := range c.Overflow {
		if n > 0 {
			doc.Overflow[k] = n
		}
	}
	if len(c.Ops) > 0 {
		doc.Ops = map[string]map[string]int64{}
		for k, m := range c.Ops {
			doc.Ops[k] = map[string]int64{}
			for class, n := range m {
				doc.Ops[k][class] = n
			}
		}
	}
	byKind := map[string][]traced.CensusEntry{}
	for _, e := range c.Entries() { // sorted by kind, then path
		byKind[e.Kind] = append(byKind[e.Kind], e)
	}
	kinds := make([]string, 0, len(byKind))
	for k := range byKind {
		kinds = append(kinds, k)
	}
	sort.Strings(kinds)
	for {
		doc.Entries = []traced.CensusEntry{}
		for _, k := range kinds {
			doc.Entries = append(doc.Entries, byKind[k]...)
		}
		doc.OverflowTotal = 0
		for _, n := range doc.Overflow {
			doc.OverflowTotal += n
		}
		data, err := json.Marshal(doc)
		if err != nil {
			return nil, err
		}
		if len(data) <= MaxCensusBytes {
			return data, nil
		}
		shed := false
		for _, k := range kinds {
			es := byKind[k]
			if len(es) <= 1 {
				continue
			}
			cut := len(es) / 8
			if cut < 1 {
				cut = 1
			}
			if cut > len(es)-1 {
				cut = len(es) - 1
			}
			for _, e := range es[len(es)-cut:] {
				doc.Overflow[k] += e.Count
			}
			byKind[k] = es[:len(es)-cut]
			shed = true
		}
		if !shed {
			return data, nil // one entry per kind: nothing fairer to drop
		}
	}
}

// censusWrite writes the file; a variable so a test can make it fail.
var censusWrite = writeCensusFile

// writeCensusFile writes data to final atomically: temp file, rename.
func writeCensusFile(final string, data []byte) error {
	var rnd [8]byte
	if _, err := rand.Read(rnd[:]); err != nil {
		return err
	}
	tmp := final + ".tmp-" + hex.EncodeToString(rnd[:])
	f, err := os.OpenFile(tmp, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0o600)
	if err != nil {
		return err
	}
	_, err = f.Write(data)
	// No fsync, on purpose: the census is written while the parent's classify
	// deadline runs, and a slow sync could cost the valid record.json that was
	// linked just before.  rename(2) alone gives whole-or-nothing to a reader
	// of the live directory; after a crash the file may be empty or absent,
	// which a diagnostic on an ephemeral runner can bear.
	if cerr := f.Close(); err == nil {
		err = cerr
	}
	if err == nil {
		err = os.Rename(tmp, final)
	}
	if err != nil {
		os.Remove(tmp)
	}
	return err
}

// writeCensusBestEffort encodes and writes the census, and swallows every
// failure, a panic included.  It reports nothing: that is the point.
func writeCensusBestEffort(dir, action string, c *traced.Census) {
	defer func() { _ = recover() }()
	if c == nil {
		return
	}
	data, err := EncodeCensus(action, c)
	if err != nil {
		return
	}
	_ = censusWrite(CensusFile(dir, action), data)
}
