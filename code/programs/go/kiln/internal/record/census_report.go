package record

// `kiln record census`: reading a record directory back (KILN04, "`kiln record
// census`")
// ==========================================================================
//
// A recording leaves, per action, a directory of files the CHILDREN wrote:
//
//	traces/<sha256(action)>/N.trace     strace's output for step N
//	traces/<sha256(action)>/N.json      the step's sidecar (kiln-record-step/2)
//	traces/<sha256(action)>/record.json     the classifier's verdict, or
//	traces/<sha256(action)>/record.failed   its failure marker
//	traces/<sha256(action)>/census.json     the host paths the classifier saw
//
// This file walks them all, AFTER the build, and boils them down to a bounded
// accumulator (censusAcc) that census_render.go turns into the summary.  It is
// a diagnostic: it is never run during a recording, nothing reads its output for
// a verdict, and it never writes inside the record directory.
//
// Trust.  Every file is read as hostile (see census_stats.go): opened without
// following a symlink and without blocking on a FIFO (openNoFollow), required to
// be a regular file, read under a size bound, decoded strictly, and skipped with
// a COUNTED problem code when it is not what it should be.  Nothing here may
// panic on a malformed input, and nothing here believes a census.json: it is a
// number to be counted, never a verdict.
//
// The one thing that is not a file read is the per-trace parse, which is the
// point of the whole command: for each step trace it times ONE grammar-path
// parse (the call the classify child makes) and runs ONE cross-checked parse (the
// grammar path against the old scanner), so a sample of REAL builds feeds the
// G1/G2 and soak numbers of section 4.9.

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"math/bits"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// DefaultCrossCheckMaxBytes is the largest trace the census cross-checks: the
// cross-check buffers the whole trace (and parses it twice), so a trace near the
// 1 GiB cap would take several GiB.  A larger one is still timed (the child's own
// parse) but its cross-check is skipped and counted.  The workflow should also set
// GOMEMLIMIT for the census process.
const DefaultCrossCheckMaxBytes int64 = 256 << 20

// CensusSummarySchema is the schema of the summary.
const CensusSummarySchema = "kiln-census-summary/1"

// The summary's bounds, and the bounds on what is read to make it.
const (
	MaxSummaryJSONBytes = 512 << 10
	MaxSummaryMDBytes   = 256 << 10
	// CensusTopN is how many full paths (and prefixes) per kind the summary keeps.
	CensusTopN = 200

	maxCensusFailedBytes   = 64 << 10 // a record.failed is read at most this big
	maxCensusRecordBytes   = 32 << 20 // a record.json is read at most this big
	maxCensusEntries       = 65536    // entries in one census.json (the writer's own bound is 16384)
	maxCensusKinds         = 128      // distinct kinds kept; the rest are "(other)"
	maxCensusDistinct      = 100_000  // distinct (kind, path) pairs kept, over everything
	maxCensusPathBytes     = 32 << 20 // total bytes of the distinct paths kept
	maxCensusPrefixes      = 50_000   // distinct prefixes kept per kind
	maxCensusLanguages     = 64       // distinct languages kept; the rest are "other"
	maxCensusSteps         = 4096     // step files looked at per action
	maxCensusActions       = 200_000  // action directories looked at
	maxCensusPathRunes     = 512      // a path is cut to this many runes
	maxCensusDetails       = 20       // cross-check details kept
	maxCensusDetailRunes   = 200
	maxCensusHeaderBytes   = 1 << 20 // collect's header line
	maxCensusRecordsBytes  = 4 << 30 // how far the records file is read to count lines
	censusPrefixDepth      = 4
	maxCensusCPUInfo       = 256 << 10
	censusLargestActions   = 10
	censusActionNameRunes  = 200
	censusRecordsReadChunk = 1 << 20
)

// censusKindRE is what a kind must look like.  The writer's kinds are a fixed
// vocabulary ("host-read", "trace-incomplete:proc-path", ...); a file that
// carries anything else is not one of ours.
var censusKindRE = regexp.MustCompile(`^[a-z0-9:_-]{1,96}$`)

// censusLangRE is what a language name must look like.
var censusLangRE = regexp.MustCompile(`^[a-z0-9][a-z0-9_+.-]{0,31}$`)

// ---------------------------------------------------------------------------
// The summary
// ---------------------------------------------------------------------------

// CensusSummary is the whole report (JSON schema kiln-census-summary/1).
type CensusSummary struct {
	Schema string `json:"schema"`
	// InfrastructureFailure is "" or why the census could not do its job (no
	// traces; a pre/ that does not verify, so nothing could be parsed).
	InfrastructureFailure string            `json:"infrastructure_failure"`
	Host                  HostSummary       `json:"host"`
	Problems              map[string]int    `json:"problems"`
	Actions               ActionCounts      `json:"actions"`
	Explain               ExplainSummary    `json:"explain"`
	Reasons               []KindSummary     `json:"reasons"`
	TraceSizes            TraceSizes        `json:"trace_sizes"`
	Timing                TimingSummary     `json:"timing"`
	PerAction             PerActionSummary  `json:"per_action"`
	CrossCheck            CrossCheckSummary `json:"cross_check"`
	Languages             []LangSummary     `json:"languages"`
	Collect               *CollectSummary   `json:"collect"`
	Bounds                BoundsSummary     `json:"bounds"`
}

// HostSummary says where the recording was made: from the verified binding and
// host facts of pre/, and the CPU model of THIS machine.
type HostSummary struct {
	ImageID       string `json:"image_id"`
	Kernel        string `json:"kernel"`
	StraceVersion string `json:"strace_version"`
	ShardIndex    int    `json:"shard_index"`
	ShardCount    int    `json:"shard_count"`
	CPUModel      string `json:"cpu_model"`
}

// ActionCounts counts actions by what was found for them.  The census states
// are exclusive and add up to Total.  NONE of them means "no host reads" except
// CensusClean: an empty census beside a record whose parse half FAILED is a
// classifier that never ran, and a missing census is a classifier that errored.
type ActionCounts struct {
	Total                  int `json:"total"`
	WithRecord             int `json:"with_record"`
	RecordParseOK          int `json:"record_parse_ok"`
	RecordWritesComplete   int `json:"record_writes_complete"`
	RecordFailed           int `json:"record_failed"`
	NoVerdict              int `json:"no_verdict"`
	CensusMissing          int `json:"census_missing"`
	CensusInvalid          int `json:"census_invalid"`
	CensusEmptyParseFailed int `json:"census_empty_parse_failed"`
	CensusEmptyNoRecord    int `json:"census_empty_no_record"`
	CensusClean            int `json:"census_clean"`
	CensusNonEmpty         int `json:"census_nonempty"`
}

// KindSummary is the histogram of one reason kind.
type KindSummary struct {
	Kind          string      `json:"kind"`
	Occurrences   int64       `json:"occurrences"`
	Overflow      int64       `json:"overflow"`
	Actions       int         `json:"actions"`
	DistinctPaths int         `json:"distinct_paths"`
	DroppedPaths  int64       `json:"dropped_paths"`
	Prefixes      []PrefixRow `json:"prefixes"`
	TopPaths      []PathRow   `json:"top_paths"`
}

// PrefixRow is one path prefix of depth at most 4.
type PrefixRow struct {
	Prefix string `json:"prefix"`
	Paths  int    `json:"paths"`
	Count  int64  `json:"count"`
}

// PathRow is one full path and how often it was raised.
type PathRow struct {
	Path  string `json:"path"`
	Count int64  `json:"count"`
}

// TraceSizes are the size distributions of the step traces: all of them, those
// of steps that exited 0, and their line counts.
type TraceSizes struct {
	All    SizeDist `json:"all"`
	RootOK SizeDist `json:"root_ok"`
	Lines  Dist     `json:"lines"`
}

// TimingSummary is G2's raw material: the step wall, the grammar parse time and
// their ratio, with the counts over each limit.  Report-only.
type TimingSummary struct {
	StepWallMS     Dist `json:"step_wall_ms"`
	ParseMS        Dist `json:"parse_ms"`
	ParseShare     Dist `json:"parse_share"`
	ShareOver10Pct int  `json:"share_over_10pct"`
	ParseOver5s    int  `json:"parse_over_5s"`
	G2Failing      int  `json:"g2_failing"`
	WallsDropped   int  `json:"walls_dropped"`
	ParseRefused   int  `json:"parse_refused"`
	// ParseMBPerS is the parse throughput (trace megabytes per second of parse).
	ParseMBPerS Dist `json:"parse_mb_per_s"`
	// BySize is the timing per log2 trace-size bucket: G2 is a statement about a
	// trace SIZE, so a blended percentile hides it.
	BySize []SizeTiming `json:"by_size"`
	// G2AtP99 is the G2 verdict over the traces at or above the census p99 size,
	// where G2 is defined.
	G2AtP99 G2P99 `json:"g2_at_p99"`
	// FirstParse is the first timed parse alone: it includes the process's lazy
	// initialisation (grammar tables), which each classify child pays again, so
	// it is the cost to add back per action.
	FirstParse FirstParse `json:"first_parse"`
}

// SizeTiming is one size bucket's timing.
type SizeTiming struct {
	Min        int64 `json:"min"`
	N          int   `json:"n"`
	ParseMS    Dist  `json:"parse_ms"`
	ParseShare Dist  `json:"parse_share"`
}

// G2P99 is G2 over the traces at or above the census p99 size.
type G2P99 struct {
	ThresholdBytes float64 `json:"threshold_bytes"`
	Traces         int     `json:"traces"`
	Over10Pct      int     `json:"share_over_10pct"`
	Over5s         int     `json:"parse_over_5s"`
	Failing        int     `json:"failing"`
}

// FirstParse is the cold-start parse.
type FirstParse struct {
	MS    float64 `json:"ms"`
	Bytes int64   `json:"bytes"`
	// Accepted says whether that parse was accepted; the first timed parse is
	// taken whatever its outcome, because the cold start is paid either way.
	Accepted bool `json:"accepted"`
}

// PerActionSummary is the total trace bytes of each action against the cap
// (DefaultMaxTraceTotal).
type PerActionSummary struct {
	CapBytes int64         `json:"cap_bytes"`
	Bytes    Dist          `json:"bytes"`
	Over50   int           `json:"at_least_50pct_of_cap"`
	Over90   int           `json:"at_least_90pct_of_cap"`
	OverCap  int           `json:"at_least_cap"`
	Largest  []ActionBytes `json:"largest"`
}

// ActionBytes is one action's total trace bytes.
type ActionBytes struct {
	Action     string  `json:"action"`
	Bytes      int64   `json:"bytes"`
	PercentCap float64 `json:"percent_of_cap"`
}

// CrossCheckSummary is the D34 line, from the census's own parses.
type CrossCheckSummary struct {
	Traces       int      `json:"traces"`
	Bytes        int64    `json:"bytes"`
	Accepted     int      `json:"accepted"`
	Refused      int      `json:"refused"`
	Skipped      int      `json:"skipped_too_large"`
	SkippedBytes int64    `json:"skipped_bytes"`
	Mismatches   int      `json:"mismatches"`
	Allowed      int      `json:"allowed"`
	Details      []string `json:"details"`
}

// LangSummary is one language's recordable share.
type LangSummary struct {
	Language   string  `json:"language"`
	Actions    int     `json:"actions"`
	Recordable int     `json:"recordable"`
	Clean      int     `json:"clean"`
	Share      float64 `json:"recordable_share"`
}

// CollectSummary is what collect's records.jsonl header said.
type CollectSummary struct {
	ShardIndex                int      `json:"shard_index"`
	ShardCount                int      `json:"shard_count"`
	Void                      []string `json:"void"`
	PostInventoryUnexplained  int      `json:"post_inventory_unexplained"`
	TrackedChangedUnexplained int      `json:"tracked_changed_unexplained"`
	Records                   int64    `json:"records"`
}

// BoundsSummary says what was cut to keep the report bounded.
type BoundsSummary struct {
	JSONMaxBytes       int   `json:"json_max_bytes"`
	MDMaxBytes         int   `json:"md_max_bytes"`
	TopPerKind         int   `json:"top_per_kind"`
	JSONListsCut       bool  `json:"json_lists_cut"`
	MDTopPerKind       int   `json:"md_top_per_kind"`
	MDTruncated        bool  `json:"md_truncated"`
	PathsTruncated     int64 `json:"paths_truncated"`
	PathsDropped       int64 `json:"paths_dropped"`
	PrefixesDropped    int64 `json:"prefixes_dropped"`
	KindsCollapsed     int   `json:"kinds_collapsed"`
	LanguagesCollapsed int   `json:"languages_collapsed"`
	ActionsSkipped     int   `json:"actions_skipped"`
}

// ---------------------------------------------------------------------------
// The accumulator
// ---------------------------------------------------------------------------

type prefixAcc struct {
	paths int
	count int64
}

type kindAcc struct {
	occurrences, overflow, dropped int64
	actions                        int
	paths                          map[string]int64
	prefixes                       map[string]*prefixAcc
}

type langAcc struct{ actions, recordable, clean int }

// parseSample is one accepted timed parse: the trace's size, the parse time and
// its share of the step's wall (-1 when the wall was not positive).
type parseSample struct {
	size      int64
	ms, share float64
}

type namedBytes struct {
	name  string
	bytes int64
}

// censusAcc is what the walk accumulates.  Its memory is bounded by the caps
// above (distinct paths, prefixes, kinds, languages) plus a few numbers per
// step trace.
type censusAcc struct {
	red       *redactor
	problems  map[string]int
	actions   ActionCounts
	explain   ExplainSummary
	host      HostSummary
	samples   []parseSample
	firstSet  bool
	first     FirstParse
	pathBytes int64
	kinds     map[string]*kindAcc
	distinct  int

	pathsTruncated, pathsDropped, prefixesDropped int64
	kindsCollapsed, languagesCollapsed, skipped   int

	sizesAll, sizesOK []int64
	lines             []float64
	walls, parses     []float64
	shares            []float64
	wallsDropped      int
	over10, over5s    int
	g2                int
	parseRefused      int

	actionBytes []namedBytes
	cross       CrossCheckSummary
	langs       map[string]*langAcc
	collect     *CollectSummary
	infra       string
}

func newCensusAcc(red *redactor) *censusAcc {
	return &censusAcc{red: red, explain: newExplain(), problems: map[string]int{}, kinds: map[string]*kindAcc{}, langs: map[string]*langAcc{}}
}

// problem counts one problem under a fixed code.
func (a *censusAcc) problem(code string) { a.problems[code]++ }

// ---------------------------------------------------------------------------
// The walk
// ---------------------------------------------------------------------------

// CensusOptions are `kiln record census`'s inputs.
type CensusOptions struct {
	// Dir is the record directory (absolute, canonical, as VerifyPre needs it).
	Dir string
	// Records, when set, is collect's records.jsonl: its header and its line
	// count join the report.
	Records string
	// CrossCheck is the cross-checked parse; nil means trace.ParseCrossChecked.
	// This file is the ONE production file that names it
	// (TestParseCrossCheckedOnlyInCensus).
	CrossCheck CrossCheckFunc
	// CrossCheckMaxBytes overrides DefaultCrossCheckMaxBytes (0: the default).
	CrossCheckMaxBytes int64
	// CPUInfo is the file the CPU model is read from (empty: /proc/cpuinfo).
	CPUInfo string
	// Redact are the directory prefixes to label in any text (the caller passes
	// $HOME, the temporary directories and the workspace; the record directory is
	// added here).
	Redact []Redaction
}

// looksLikeRecordDir says dir has a real `pre/` and a real `traces/`.
func looksLikeRecordDir(dir string) error {
	for _, d := range []string{"pre", "traces"} {
		st, err := os.Lstat(filepath.Join(dir, d))
		if err != nil || st.Mode()&os.ModeSymlink != 0 || !st.IsDir() {
			return fmt.Errorf("%s/ is not a directory: this does not look like a record directory", d)
		}
	}
	return nil
}

// collectCensus walks the record directory.  An error is a REFUSAL (not a
// record directory); every other trouble is a counted problem, or the
// accumulator's infra text.
func collectCensus(o CensusOptions) (*censusAcc, error) {
	if err := looksLikeRecordDir(o.Dir); err != nil {
		return nil, err
	}
	red := newRedactor(append(append([]Redaction(nil), o.Redact...), Redaction{From: o.Dir, To: "$RECORD_DIR"}))
	acc := newCensusAcc(red)
	run := &censusRun{o: o, acc: acc, crossCheck: o.CrossCheck}
	if run.crossCheck == nil {
		run.crossCheck = trace.ParseCrossChecked
	}
	pre, err := VerifyPre(o.Dir)
	if err != nil {
		text, _ := safeText(err.Error(), 300, red)
		acc.problem("pre_unverified")
		acc.infra = "pre/ does not verify, so no trace was parsed: " + text
	} else {
		run.pre = pre
		acc.hostFrom(pre, o.CPUInfo)
	}
	ents, err := os.ReadDir(filepath.Join(o.Dir, "traces"))
	if err != nil {
		acc.problem("traces_unreadable")
	}
	for _, e := range ents {
		if acc.actions.Total >= maxCensusActions {
			acc.skipped++
			continue
		}
		if e.Name() == ".seq" || e.Name() == ".serial" || e.Name() == ".collected" {
			continue // the recorder's own sequence counter, lock and collected marker
		}
		if !hexDigest(e.Name()) || !e.Type().IsDir() {
			acc.problem("stray_traces_entry")
			continue
		}
		run.action(e.Name())
	}
	if o.Records != "" {
		acc.collect = readCollectSummary(o.Records, acc)
	}
	if acc.infra == "" && len(acc.sizesAll) == 0 {
		acc.infra = "no step trace was found to parse"
	} else if acc.infra == "" && acc.cross.Traces == 0 {
		acc.infra = "no trace was cross-checked (every one was too large, unreadable or refused a panic)"
	}
	return acc, nil
}

// censusRun is the walk's state.
type censusRun struct {
	o          CensusOptions
	acc        *censusAcc
	pre        *Pre
	crossCheck CrossCheckFunc
}

// stepIndex parses "<n>.<ext>" with n canonical (no sign, no leading zero).
func stepIndex(name, ext string) (int, bool) {
	s, ok := strings.CutSuffix(name, "."+ext)
	if !ok || s == "" || len(s) > 6 {
		return 0, false
	}
	n, err := strconv.Atoi(s)
	if err != nil || n < 0 || strconv.Itoa(n) != s {
		return 0, false
	}
	return n, true
}

// sha256Hex is the directory name an action maps to.
func sha256Hex(s string) string {
	sum := sha256.Sum256([]byte(s))
	return hex.EncodeToString(sum[:])
}

// censusState is what census.json turned out to be.
type censusState int

const (
	censusMissing censusState = iota
	censusInvalid
	censusEmpty
	censusNonEmpty
)

// action processes one action directory (the directory's name is the SHA-256 of
// the action's name; the name itself is learned from the files, then checked
// against the directory).
func (r *censusRun) action(hexName string) {
	a := r.acc
	adir := filepath.Join(r.o.Dir, "traces", hexName)
	ents, err := os.ReadDir(adir)
	if err != nil {
		a.problem("action_dir_unreadable")
		return
	}
	a.actions.Total++
	var sidecars []int
	traces := map[int]bool{}
	for _, e := range ents {
		if n, ok := stepIndex(e.Name(), "json"); ok {
			if len(sidecars) >= maxCensusSteps {
				a.problem("steps_over_cap")
				continue
			}
			sidecars = append(sidecars, n)
		} else if n, ok := stepIndex(e.Name(), "trace"); ok {
			if len(traces) < maxCensusSteps {
				traces[n] = true
			}
		}
	}
	sort.Ints(sidecars)

	// The sidecars first: they name the action.
	name := ""
	scs := map[int]StepSidecar{}
	for _, j := range sidecars {
		data, err := readBoundedFile(filepath.Join(adir, strconv.Itoa(j)+".json"), maxSidecarBytes)
		if err != nil {
			a.problem("sidecar_unreadable")
			continue
		}
		sc, err := DecodeStepSidecar(data)
		if err != nil {
			a.problem("sidecar_invalid")
			continue
		}
		if sc.Step != j || sha256Hex(sc.Action) != hexName {
			a.problem("sidecar_action_mismatch")
			continue
		}
		scs[j] = sc
		name = sc.Action
		a.sidecarFacts(sc)
	}

	// The verdict.  record.failed WINS over a record.json, as in collect: the
	// parent writes it over a valid record when its deadline fires.
	var rec *ClassifiedRecord
	if data, err := readBoundedFile(filepath.Join(adir, "record.json"), maxCensusRecordBytes); err == nil {
		if c, err := DecodeClassified(data); err != nil {
			a.problem("record_invalid")
		} else if sha256Hex(c.Action) != hexName {
			a.problem("record_action_mismatch")
		} else {
			if name == "" {
				name = c.Action
			}
			if r.pre != nil && (c.BindingID != r.pre.BindingID || c.SealedSHA256 != r.pre.SealedSHA256) {
				// Another binding or another sealed directory: not this recording's.
				a.problem("record_binding_mismatch")
			} else {
				rec = &c
			}
		}
	} else if !errors.Is(err, fs.ErrNotExist) {
		a.problem("record_unreadable")
	}
	failed := false
	if st, err := os.Lstat(filepath.Join(adir, "record.failed")); err == nil && st.Mode().IsRegular() {
		failed = true
		code := "other"
		if text, err := readBoundedFile(filepath.Join(adir, "record.failed"), maxCensusFailedBytes); err == nil {
			code = failedCode(string(text))
		}
		a.explain.RecordFailed[code]++
		if rec != nil {
			a.problem("record_and_failed_both")
			rec = nil
		}
	}
	switch {
	case rec != nil:
		a.actions.WithRecord++
		if rec.ParseOK {
			a.actions.RecordParseOK++
		}
		if rec.WritesComplete {
			a.actions.RecordWritesComplete++
		}
		a.explainRecord(rec)
	case failed:
		// A failure marker, counted on its own.
	default:
		a.actions.NoVerdict++
	}
	if failed {
		a.actions.RecordFailed++
	}

	// The census.
	state, doc := r.readCensus(adir, name)
	recordable := rec != nil && rec.ParseOK && rec.WritesComplete
	switch state {
	case censusMissing:
		a.actions.CensusMissing++
	case censusInvalid:
		a.actions.CensusInvalid++
	case censusEmpty:
		switch {
		case rec != nil && !rec.ParseOK:
			a.actions.CensusEmptyParseFailed++
		case rec != nil:
			a.actions.CensusClean++
		default:
			a.actions.CensusEmptyNoRecord++
		}
	case censusNonEmpty:
		a.actions.CensusNonEmpty++
		a.addCensus(doc)
	}

	// The language.
	lang := "other"
	if name != "" {
		lang = r.language(name)
	}
	la := a.langs[lang]
	if la == nil {
		if len(a.langs) >= maxCensusLanguages {
			a.languagesCollapsed++
			lang = "other"
			if la = a.langs[lang]; la == nil {
				la = &langAcc{}
				a.langs[lang] = la
			}
		} else {
			la = &langAcc{}
			a.langs[lang] = la
		}
	}
	la.actions++
	if recordable {
		la.recordable++
		if state == censusEmpty {
			la.clean++
		}
	}

	// The traces.
	for j := range traces {
		if _, ok := scs[j]; !ok {
			a.problem("trace_without_sidecar")
		}
	}
	steps := make([]int, 0, len(scs))
	for j := range scs {
		steps = append(steps, j)
	}
	sort.Ints(steps)
	var total int64
	for _, j := range steps {
		sc := scs[j]
		if !sc.Started {
			continue
		}
		if !traces[j] {
			a.problem("trace_missing")
			continue
		}
		total += r.step(adir, name, j, sc)
	}
	if total > 0 {
		shown, _ := safeText(name, censusActionNameRunes, a.red)
		a.actionBytes = append(a.actionBytes, namedBytes{name: shown, bytes: total})
	}
}

// language is the action's language: the plan's (verified pre/), else the one
// in "code/packages/<lang>/..." or "code/programs/<lang>/...".  Anything that
// does not look like a language name is "other".
func (r *censusRun) language(name string) string {
	cand := ""
	if r.pre != nil {
		if p, ok := r.pre.Action(name); ok {
			cand = p.Language
		}
	}
	if cand == "" {
		parts := strings.Split(name, "/")
		if len(parts) >= 3 && parts[0] == "code" && (parts[1] == "packages" || parts[1] == "programs") {
			cand = parts[2]
		}
	}
	if !censusLangRE.MatchString(cand) {
		return "other"
	}
	return cand
}

// readCensus reads one action's census.json under every rule of the file's
// trust level.  doc is valid only for censusNonEmpty.
func (r *censusRun) readCensus(adir, name string) (censusState, *CensusDoc) {
	a := r.acc
	data, err := readBoundedFile(filepath.Join(adir, CensusFileName), MaxCensusBytes)
	if err != nil {
		if errors.Is(err, fs.ErrNotExist) {
			return censusMissing, nil
		}
		a.problem("census_unreadable") // a symlink, a FIFO, a directory, too big, an I/O error
		return censusInvalid, nil
	}
	var doc CensusDoc
	if err := legacy.PolicyDecode(data, &doc); err != nil {
		a.problem("census_decode")
		return censusInvalid, nil
	}
	switch {
	case doc.Schema != CensusSchema:
		a.problem("census_schema")
		return censusInvalid, nil
	case name != "" && doc.Action != name:
		a.problem("census_action_mismatch")
		return censusInvalid, nil
	case doc.Total < 0 || doc.OverflowTotal < 0 || len(doc.Entries) > maxCensusEntries || len(doc.Overflow) > maxCensusKinds:
		a.problem("census_bounds")
		return censusInvalid, nil
	}
	for k, n := range doc.Overflow {
		if !censusKindRE.MatchString(k) || n < 0 {
			a.problem("census_bad_entry")
			return censusInvalid, nil
		}
	}
	for _, e := range doc.Entries {
		if !censusKindRE.MatchString(e.Kind) || e.Count < 0 {
			a.problem("census_bad_entry")
			return censusInvalid, nil
		}
	}
	if doc.Total == 0 && doc.OverflowTotal == 0 && len(doc.Entries) == 0 {
		return censusEmpty, nil
	}
	return censusNonEmpty, &doc
}

// kindFor returns the accumulator of a kind, collapsing a kind beyond the cap.
func (a *censusAcc) kindFor(kind string) *kindAcc {
	k := a.kinds[kind]
	if k != nil {
		return k
	}
	if len(a.kinds) >= maxCensusKinds {
		a.kindsCollapsed++
		kind = "(other)"
		if k = a.kinds[kind]; k != nil {
			return k
		}
	}
	k = &kindAcc{paths: map[string]int64{}, prefixes: map[string]*prefixAcc{}}
	a.kinds[kind] = k
	return k
}

// addCensus folds one valid census into the histograms.
func (a *censusAcc) addCensus(doc *CensusDoc) {
	seen := map[*kindAcc]bool{}
	for kind, n := range doc.Overflow {
		k := a.kindFor(kind)
		k.overflow += n
		seen[k] = true
	}
	for _, e := range doc.Entries {
		k := a.kindFor(e.Kind)
		seen[k] = true
		k.occurrences += e.Count
		p, cut := safeText(e.Path, maxCensusPathRunes, a.red)
		if cut {
			a.pathsTruncated++
		}
		if _, known := k.paths[p]; !known {
			if a.distinct >= maxCensusDistinct || a.pathBytes+int64(len(p)) > maxCensusPathBytes {
				a.pathsDropped++
				k.dropped += e.Count
				continue
			}
			a.distinct++
			a.pathBytes += int64(len(p))
		}
		first := false
		if _, known := k.paths[p]; !known {
			first = true
		}
		k.paths[p] += e.Count
		pr := pathPrefix(p, censusPrefixDepth)
		pa := k.prefixes[pr]
		if pa == nil {
			if len(k.prefixes) >= maxCensusPrefixes {
				a.prefixesDropped++
				continue
			}
			pa = &prefixAcc{}
			k.prefixes[pr] = pa
		}
		pa.count += e.Count
		if first {
			pa.paths++
		}
	}
	for k := range seen {
		k.actions++
	}
}

// countLines reads f to its end (at most limit+1 bytes) and counts newlines.
func countLines(f io.Reader, limit int64) (int64, error) {
	buf := make([]byte, 256<<10)
	var lines, read int64
	for {
		n, err := f.Read(buf)
		lines += int64(bytes.Count(buf[:n], []byte{'\n'}))
		read += int64(n)
		if err == io.EOF {
			return lines, nil
		}
		if err != nil {
			return lines, err
		}
		if read > limit {
			return lines, nil
		}
	}
}

// CrossCheckFunc is the cross-checked parse (trace.ParseCrossChecked's shape).
// CensusOptions.CrossCheck injects it, so a test can stand in a mismatch or an
// allowed divergence, which no real trace is known to produce, without a
// package-level variable.
type CrossCheckFunc = func(io.Reader, string, trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, int)

// helperParseOptions are the options the classify child parses a step trace
// with (parseHelperTrace builds them from the same function): helper-root mode,
// the sealed Kiln binary as the helper, the classify limits.
func helperParseOptions(kilnExe string, limits trace.Limits) trace.Options {
	return trace.Options{Limits: limits, HelperRoot: true, HelperExe: kilnExe}
}

// step measures one step's trace and returns its size in bytes (0 when it was
// not read).  It is the heart of the command; the order matters:
//
//  1. open (no follow), regular file, at most the classify limit;
//  2. count bytes and lines (not timed);
//  3. the wall from the sidecar;
//  4. ONE timed parse through parseStep, GC ON, as in production;
//  5. ONE cross-checked parse, whose mismatch is a value.
//
// Only one trace is in memory at a time: each parse's result is dropped before
// the next trace is opened.
func (r *censusRun) step(adir, name string, j int, sc StepSidecar) int64 {
	a := r.acc
	path := filepath.Join(adir, strconv.Itoa(j)+".trace")
	limits := ClassifyLimits()
	f, err := openNoFollow(path)
	if err != nil {
		a.problem("trace_unreadable")
		return 0
	}
	st, err := f.Stat()
	if err != nil || !st.Mode().IsRegular() {
		f.Close()
		a.problem("trace_not_regular")
		return 0
	}
	size := st.Size()
	if size > limits.MaxTotalBytes {
		f.Close()
		a.problem("trace_over_limit")
		return 0
	}
	lines, err := countLines(f, size)
	f.Close()
	if err != nil {
		a.problem("trace_unreadable")
		return 0
	}
	if size != sc.TraceBytes {
		a.problem("trace_size_differs_from_sidecar")
	}
	a.sizesAll = append(a.sizesAll, size)
	if sc.RootStatus == 0 {
		a.sizesOK = append(a.sizesOK, size)
	}
	a.lines = append(a.lines, float64(lines))

	// The wall.  A wall that is not positive (a stamp that does not parse, a
	// clock that stepped back, a zero) is dropped and counted, never divided by.
	var wallMS float64
	t0, err1 := time.Parse(time.RFC3339Nano, sc.StartedAt)
	t1, err2 := time.Parse(time.RFC3339Nano, sc.EndedAt)
	if d := t1.Sub(t0); err1 != nil || err2 != nil || d <= 0 {
		a.wallsDropped++
	} else {
		wallMS = d.Seconds() * 1000
		a.walls = append(a.walls, wallMS)
	}

	if r.pre == nil {
		return size
	}
	if _, ok := r.pre.Action(name); !ok {
		a.problem("action_not_in_plan")
		return size
	}
	pkgDir := r.pre.Config.PackageDir(name)

	// The timed parse.  Not trace.Measure*: those switch the GC off, which is
	// process-global and test-only.  The one garbage collection BEFORE the clock
	// starts is outside the timing: it keeps the previous trace's garbage (this
	// process is long-lived, the child is not) off this parse's account.
	runtime.GC()
	var why string
	var elapsed time.Duration
	if !a.guard(func() {
		start := time.Now()
		_, why = parseStep(r.pre, name, j, sc, pkgDir, limits)
		elapsed = time.Since(start)
	}) {
		return size
	}
	if !a.firstSet {
		a.firstSet, a.first = true, FirstParse{MS: round3(elapsed.Seconds() * 1000), Bytes: size, Accepted: why == ""}
	}
	if why != "" {
		a.parseRefused++
		a.explain.ParseRefusals[parseReasonCode(why)]++
	} else {
		ms := elapsed.Seconds() * 1000
		a.parses = append(a.parses, ms)
		sample := parseSample{size: size, ms: ms, share: -1}
		over5 := ms > 5000
		if over5 {
			a.over5s++
		}
		over10 := false
		if wallMS > 0 {
			share := ms / wallMS
			sample.share = share
			a.shares = append(a.shares, share)
			if share > 0.10 {
				a.over10++
				over10 = true
			}
		}
		a.samples = append(a.samples, sample)
		if over5 || over10 {
			a.g2++
		}
	}

	// A trace over the threshold is timed but not cross-checked: the cross-check
	// buffers it whole and parses it twice.
	limit := r.o.CrossCheckMaxBytes
	if limit <= 0 {
		limit = DefaultCrossCheckMaxBytes
	}
	if size > limit {
		a.cross.Skipped++
		a.cross.SkippedBytes += size
		return size
	}

	// The cross-checked parse (a second file read; one trace in memory).
	cf, err := openNoFollow(path)
	if err != nil {
		a.problem("trace_unreadable")
		return size
	}
	var perr error
	var m *trace.CrossCheckMismatch
	var allowed int
	ok := a.guard(func() {
		_, perr, m, allowed = r.crossCheck(cf, pkgDir, helperParseOptions(r.pre.Config.KilnExe, limits))
	})
	cf.Close()
	if !ok {
		return size
	}
	cc := &a.cross
	cc.Traces++
	cc.Bytes += size
	switch {
	case m != nil:
		cc.Mismatches++
		a.detail(fmt.Sprintf("%s step %d: mismatch: %s", hexName12(adir), j, m.Detail))
	case perr != nil:
		cc.Refused++
	default:
		cc.Accepted++
	}
	if allowed > 0 {
		cc.Allowed += allowed
		a.detail(fmt.Sprintf("%s step %d: %d allowed stricter-only divergence(s)", hexName12(adir), j, allowed))
	}
	return size
}

// hexName12 is the first 12 digits of the action directory's name: enough to
// find it, and carrying no text from any file.
func hexName12(adir string) string {
	b := filepath.Base(adir)
	if len(b) > 12 {
		b = b[:12]
	}
	return b
}

// detail keeps one sanitised, cut cross-check detail (at most maxCensusDetails).
func (a *censusAcc) detail(s string) {
	if len(a.cross.Details) >= maxCensusDetails {
		return
	}
	t, _ := safeText(s, maxCensusDetailRunes, a.red)
	a.cross.Details = append(a.cross.Details, t)
}

// readCollectSummary reads collect's records.jsonl: the header (line one,
// strictly decoded) and the number of record lines.  It is read as untrusted
// too: no-follow, regular, the header at most 1 MiB, the whole read at most 4
// GiB.  A failure is a counted problem and a nil result, never fatal.
func readCollectSummary(path string, a *censusAcc) *CollectSummary {
	f, err := openNoFollow(path)
	if err != nil {
		a.problem("records_unreadable")
		return nil
	}
	defer f.Close()
	if st, err := f.Stat(); err != nil || !st.Mode().IsRegular() {
		a.problem("records_unreadable")
		return nil
	}
	head := make([]byte, maxCensusHeaderBytes)
	n, err := io.ReadFull(f, head)
	if err != nil && !errors.Is(err, io.ErrUnexpectedEOF) && !errors.Is(err, io.EOF) {
		a.problem("records_unreadable")
		return nil
	}
	head = head[:n]
	nl := bytes.IndexByte(head, '\n')
	if nl < 0 {
		a.problem("records_header")
		return nil
	}
	h, err := recfmt.DecodeHeader(head[:nl])
	if err != nil {
		a.problem("records_header")
		return nil
	}
	cs := &CollectSummary{ShardIndex: h.ShardIndex, ShardCount: h.ShardCount,
		PostInventoryUnexplained: h.PostInventoryUnexplained, TrackedChangedUnexplained: h.TrackedChangedUnexplained,
		Void: []string{}}
	for _, v := range h.Void {
		if len(cs.Void) >= 16 {
			break
		}
		t, _ := safeText(v, 64, a.red)
		cs.Void = append(cs.Void, t)
	}
	// Records: the newlines after the header, in what was already read and in
	// the rest of the file.
	cs.Records = int64(bytes.Count(head[nl+1:], []byte{'\n'}))
	read := int64(len(head))
	buf := make([]byte, censusRecordsReadChunk)
	for read < maxCensusRecordsBytes {
		k, err := f.Read(buf)
		cs.Records += int64(bytes.Count(buf[:k], []byte{'\n'}))
		read += int64(k)
		if err != nil {
			break
		}
	}
	return cs
}

// ---------------------------------------------------------------------------
// From accumulator to summary
// ---------------------------------------------------------------------------

// summary renders the accumulator; top is the per-kind list length (at most
// CensusTopN).
func (a *censusAcc) summary(top int) *CensusSummary {
	s := &CensusSummary{
		Schema: CensusSummarySchema, InfrastructureFailure: a.infra, Host: a.host,
		Problems: map[string]int{}, Actions: a.actions, Explain: a.explain, Collect: a.collect,
		Reasons: []KindSummary{}, Languages: []LangSummary{},
	}
	for k, v := range a.problems {
		s.Problems[k] = v
	}
	kinds := make([]string, 0, len(a.kinds))
	for k := range a.kinds {
		kinds = append(kinds, k)
	}
	sort.Strings(kinds)
	for _, kind := range kinds {
		k := a.kinds[kind]
		ks := KindSummary{Kind: kind, Occurrences: k.occurrences, Overflow: k.overflow, Actions: k.actions,
			DistinctPaths: len(k.paths), DroppedPaths: k.dropped, Prefixes: []PrefixRow{}, TopPaths: []PathRow{}}
		for p, n := range k.paths {
			ks.TopPaths = append(ks.TopPaths, PathRow{Path: p, Count: n})
		}
		sort.Slice(ks.TopPaths, func(i, j int) bool {
			if ks.TopPaths[i].Count != ks.TopPaths[j].Count {
				return ks.TopPaths[i].Count > ks.TopPaths[j].Count
			}
			return ks.TopPaths[i].Path < ks.TopPaths[j].Path
		})
		if len(ks.TopPaths) > top {
			ks.TopPaths = ks.TopPaths[:top]
		}
		for p, pa := range k.prefixes {
			ks.Prefixes = append(ks.Prefixes, PrefixRow{Prefix: p, Paths: pa.paths, Count: pa.count})
		}
		sort.Slice(ks.Prefixes, func(i, j int) bool {
			if ks.Prefixes[i].Count != ks.Prefixes[j].Count {
				return ks.Prefixes[i].Count > ks.Prefixes[j].Count
			}
			return ks.Prefixes[i].Prefix < ks.Prefixes[j].Prefix
		})
		if len(ks.Prefixes) > top {
			ks.Prefixes = ks.Prefixes[:top]
		}
		s.Reasons = append(s.Reasons, ks)
	}
	s.TraceSizes = TraceSizes{All: sizeDistOf(a.sizesAll), RootOK: sizeDistOf(a.sizesOK), Lines: distOf(a.lines)}
	shares := make([]float64, len(a.shares))
	for i, v := range a.shares {
		shares[i] = v
	}
	s.Timing = TimingSummary{
		StepWallMS: distOf(a.walls), ParseMS: distOf(a.parses), ParseShare: distOf(shares),
		ShareOver10Pct: a.over10, ParseOver5s: a.over5s, G2Failing: a.g2,
		WallsDropped: a.wallsDropped, ParseRefused: a.parseRefused, FirstParse: a.first,
	}
	a.timingBySize(&s.Timing, s.TraceSizes.All.P99)
	pa := PerActionSummary{CapBytes: DefaultMaxTraceTotal, Largest: []ActionBytes{}}
	bytesF := make([]float64, 0, len(a.actionBytes))
	for _, ab := range a.actionBytes {
		bytesF = append(bytesF, float64(ab.bytes))
		if ab.bytes*2 >= DefaultMaxTraceTotal {
			pa.Over50++
		}
		if ab.bytes*10 >= DefaultMaxTraceTotal*9 {
			pa.Over90++
		}
		if ab.bytes >= DefaultMaxTraceTotal {
			pa.OverCap++
		}
	}
	pa.Bytes = distOf(bytesF)
	sorted := append([]namedBytes(nil), a.actionBytes...)
	sort.Slice(sorted, func(i, j int) bool {
		if sorted[i].bytes != sorted[j].bytes {
			return sorted[i].bytes > sorted[j].bytes
		}
		return sorted[i].name < sorted[j].name
	})
	for i := 0; i < len(sorted) && i < censusLargestActions; i++ {
		pa.Largest = append(pa.Largest, ActionBytes{Action: sorted[i].name, Bytes: sorted[i].bytes,
			PercentCap: round3(float64(sorted[i].bytes) * 100 / float64(DefaultMaxTraceTotal))})
	}
	s.PerAction = pa
	s.CrossCheck = a.cross
	if s.CrossCheck.Details == nil {
		s.CrossCheck.Details = []string{}
	}
	names := make([]string, 0, len(a.langs))
	for l := range a.langs {
		names = append(names, l)
	}
	sort.Strings(names)
	for _, l := range names {
		la := a.langs[l]
		ls := LangSummary{Language: l, Actions: la.actions, Recordable: la.recordable, Clean: la.clean}
		if la.actions > 0 {
			ls.Share = round3(float64(la.recordable) / float64(la.actions))
		}
		s.Languages = append(s.Languages, ls)
	}
	s.Bounds = BoundsSummary{
		JSONMaxBytes: MaxSummaryJSONBytes, MDMaxBytes: MaxSummaryMDBytes, TopPerKind: top,
		PathsTruncated: a.pathsTruncated, PathsDropped: a.pathsDropped, PrefixesDropped: a.prefixesDropped,
		KindsCollapsed: a.kindsCollapsed, LanguagesCollapsed: a.languagesCollapsed, ActionsSkipped: a.skipped,
	}
	return s
}

// sidecarFacts counts one decoded sidecar's facts.
func (a *censusAcc) sidecarFacts(sc StepSidecar) {
	f := &a.explain.Sidecar
	f.Steps++
	if sc.Truncated {
		f.Truncated[sc.TruncatedReason]++ // Validate allows only the three fixed reasons
	}
	if !sc.Clean {
		f.NotClean++
	}
	for _, u := range sc.Unclean {
		if c, ok := matchCode(u, uncleanCodes); ok && c == u {
			f.Unclean[u]++
		} else {
			f.Unclean["other"]++
		}
	}
	if sc.Exit125 {
		f.Exit125++
	}
	if sc.Started && sc.RootStatus != 0 {
		f.RootNonZero++
	}
}

// explainRecord counts the path-free reasons of a believed record.json: the
// kind of each classifier reason, and each parse reason's code, once per action.
func (a *censusAcc) explainRecord(rec *ClassifiedRecord) {
	seen := map[string]bool{}
	if rec.Record != nil {
		for _, r := range rec.Record.Reasons {
			if c := reasonKindCode(r); !seen[c] {
				seen[c] = true
				a.explain.RecordReasonKinds[c]++
			}
		}
	}
	seen = map[string]bool{}
	for _, r := range rec.ParseReasons {
		if c := parseReasonCode(r); !seen[c] {
			seen[c] = true
			a.explain.RecordParseReasons[c]++
		}
	}
}

// guard runs f and turns a panic in it into a counted problem: one bad trace
// must not take the whole census down.  It reports whether f finished.
func (a *censusAcc) guard(f func()) (ok bool) {
	defer func() {
		if recover() != nil {
			a.problem("parse_panic")
			ok = false
		}
	}()
	f()
	return true
}

// timingBySize fills the per-size-bucket rows, the throughput distribution and
// the G2 verdict at the census p99 size.
func (a *censusAcc) timingBySize(t *TimingSummary, p99 float64) {
	type group struct{ ms, share []float64 }
	groups := map[int64]*group{}
	var mbps []float64
	t.G2AtP99.ThresholdBytes = p99
	for _, sm := range a.samples {
		var min int64
		if k := bits.Len64(uint64(sm.size)); k > 0 {
			min = 1 << (k - 1)
		}
		g := groups[min]
		if g == nil {
			g = &group{}
			groups[min] = g
		}
		g.ms = append(g.ms, sm.ms)
		if sm.share >= 0 {
			g.share = append(g.share, sm.share)
		}
		if sm.ms > 0 {
			mbps = append(mbps, float64(sm.size)/1e6/(sm.ms/1000))
		}
		if p99 > 0 && float64(sm.size) >= p99 {
			g2 := &t.G2AtP99
			g2.Traces++
			o5, o10 := sm.ms > 5000, sm.share > 0.10
			if o5 {
				g2.Over5s++
			}
			if o10 {
				g2.Over10Pct++
			}
			if o5 || o10 {
				g2.Failing++
			}
		}
	}
	t.ParseMBPerS = distOf(mbps)
	mins := make([]int64, 0, len(groups))
	for m := range groups {
		mins = append(mins, m)
	}
	sort.Slice(mins, func(i, j int) bool { return mins[i] < mins[j] })
	t.BySize = []SizeTiming{}
	for _, m := range mins {
		g := groups[m]
		t.BySize = append(t.BySize, SizeTiming{Min: m, N: len(g.ms), ParseMS: distOf(g.ms), ParseShare: distOf(g.share)})
	}
}

// hostFrom fills the host block from the verified pre/ (binding and host facts)
// and the CPU model of this machine.  Every string is sanitised and cut.
func (a *censusAcc) hostFrom(pre *Pre, cpuinfo string) {
	cut := func(s string) string { t, _ := safeText(s, 200, a.red); return t }
	b := pre.Binding
	a.host = HostSummary{Kernel: cut(b.Kernel), StraceVersion: cut(b.StraceVersion), ShardIndex: b.ShardIndex, ShardCount: b.ShardCount}
	if data, err := pre.ReadSealed("host-facts.json"); err == nil {
		if hf, err := recfmt.DecodeHostFacts(data); err == nil {
			a.host.ImageID = cut(hf.Image)
		}
	}
	if cpuinfo == "" {
		cpuinfo = "/proc/cpuinfo"
	}
	a.host.CPUModel = cut(cpuModel(cpuinfo))
}

// cpuModel is the first "model name" of a cpuinfo file ("" if there is none).
func cpuModel(path string) string {
	f, err := os.Open(path)
	if err != nil {
		return ""
	}
	defer f.Close()
	data, err := io.ReadAll(io.LimitReader(f, maxCensusCPUInfo))
	if err != nil {
		return ""
	}
	for _, line := range strings.Split(string(data), "\n") {
		if k, v, ok := strings.Cut(line, ":"); ok && strings.TrimSpace(k) == "model name" {
			return strings.TrimSpace(v)
		}
	}
	return ""
}
