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
//	bounded         at most traced.MaxCensusEntries pairs, and the encoded
//	                file at most MaxCensusBytes (entries are shed from the tail
//	                into "overflow" until it fits)

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"os"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// CensusSchema is the schema of census.json.
const CensusSchema = "kiln-record-census/1"

// MaxCensusBytes bounds the encoded file.
const MaxCensusBytes = 512 << 10

// CensusFileName is the file's name in an action's trace directory.
const CensusFileName = "census.json"

// censusTmpPrefix starts the temporary name a census is written under.
const censusTmpPrefix = CensusFileName + ".tmp-"

// CensusFile is the path of an action's census.
func CensusFile(dir, action string) string { return StepDir(dir, action) + "/" + CensusFileName }

// CensusDoc is the file's content.
type CensusDoc struct {
	Schema   string               `json:"schema"`
	Action   string               `json:"action"`
	Total    int64                `json:"total"`
	Overflow int64                `json:"overflow"`
	Entries  []traced.CensusEntry `json:"entries"`
}

// EncodeCensus renders c, shedding entries from the tail (their counts go to
// overflow) until the result is at most MaxCensusBytes.
func EncodeCensus(action string, c *traced.Census) ([]byte, error) {
	doc := CensusDoc{Schema: CensusSchema, Action: action, Total: c.Total, Overflow: c.Overflow, Entries: c.Entries()}
	if doc.Entries == nil {
		doc.Entries = []traced.CensusEntry{}
	}
	for {
		data, err := json.Marshal(doc)
		if err != nil {
			return nil, err
		}
		if len(data) <= MaxCensusBytes || len(doc.Entries) == 0 {
			return data, nil
		}
		// Shed in chunks, so a huge file costs few re-encodes.
		cut := len(doc.Entries) / 8
		if cut < 1 {
			cut = 1
		}
		for _, e := range doc.Entries[len(doc.Entries)-cut:] {
			doc.Overflow += e.Count
		}
		doc.Entries = doc.Entries[:len(doc.Entries)-cut]
	}
}

// censusWrite writes the file; a variable so a test can make it fail.
var censusWrite = writeCensusFile

// writeCensusFile writes data to final atomically: temp file, fsync, rename.
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
	if err == nil {
		err = f.Sync()
	}
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
