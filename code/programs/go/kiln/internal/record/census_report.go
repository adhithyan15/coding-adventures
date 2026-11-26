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

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/exechead"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
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

	maxCensusFailedBytes = 64 << 10 // a record.failed is read at most this big
	maxCensusRecordBytes = 32 << 20 // a record.json is read at most this big
	maxCensusEntries     = 65536    // entries in one census.json (the writer's own bound is 20480)
	maxCensusKinds       = 128      // distinct kinds kept; the rest are "(other)"
	maxCensusDistinct    = 100_000  // distinct (kind, path) pairs kept, over everything
	maxCensusPathBytes   = 32 << 20 // total bytes of the distinct paths kept
	maxCensusPrefixes    = 50_000   // distinct prefixes kept per kind
	maxCensusLanguages   = 64       // distinct languages kept; the rest are "other"
	maxCensusSteps       = 4096     // step files looked at per action
	maxCensusActions     = 200_000  // action directories looked at
	maxCensusPathRunes   = 512      // a path is cut to this many runes
	maxCensusDetails     = 20       // cross-check details kept
	// Line outlines (trace.Shape) have TWO budgets, so a flood of one refusal
	// (many traces refused as refused:clone3) cannot starve the allowed
	// stricter-only divergences, whose lines are the ones the default flip needs:
	// allowed:* outlines get 8 slots and 32 computations of their own; refusals 8
	// slots, 32 computations in all and 2 per distinct refusal code.  A trace is
	// streamed at most twice per computation; all computations together scan at
	// most maxCensusShapeBytes (2x each trace's size is charged).
	maxCensusOpsSum          = 1 << 40 // the sum of one census.json's op counts
	maxCensusShapes          = 8
	maxCensusShapeCallsAllow = 32
	maxCensusShapeCallsRefus = 32
	maxCensusShapeCallsCode  = 2
	maxCensusShapeBytes      = 4 << 30
	maxCensusAllowedRows     = 64 // distinct allowed-row names kept
	maxCensusDetailRunes     = 200
	maxCensusHeaderBytes     = 1 << 20 // collect's header line
	maxCensusRecordsBytes    = 4 << 30 // how far the records file is read to count lines
	censusPrefixDepth        = 4
	maxCensusCPUInfo         = 256 << 10
	censusLargestActions     = 10
	censusActionNameRunes    = 200
	censusRecordsReadChunk   = 1 << 20
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
	// StatePreloadedMarker is a plain fact: traces/.state-preloaded is present as
	// a regular file. The walk skips it (it is not an action), so this is what
	// keeps "skipped" from meaning "unseen". Reported with or without --records;
	// with a collect header the problem state_preloaded_marker_without_void checks
	// it against the header's void list.
	StatePreloadedMarker bool `json:"state_preloaded_marker"`
	// ExecHead is the D48 step 6 report: whether the shard ran with exec heads
	// and what the supervisor and the classifier made of them.
	ExecHead ExecHeadSummary `json:"exec_head"`
	// ParseRules says how often the KILN04 D50 parse rules fired in this
	// shard's actions (counts only; nothing here is a path).
	ParseRules ParseRulesSummary `json:"parse_rules"`
	Bounds     BoundsSummary     `json:"bounds"`
	// Shapes are at most maxCensusShapes redacted line outlines of refusals
	// and allowed divergences (see ShapeSummary).
	Shapes []ShapeSummary `json:"shapes"`
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
	// BuiltOK counts the actions whose build did not fail: a believed
	// record.json with no `failed` reason (when the parse half failed and the
	// record has no reasons, no started step exited non-zero).  An action with no
	// believed record (record.failed, no verdict) is in neither this nor
	// Emittable: nothing says its build was fine.
	BuiltOK int `json:"built_ok"`
	// Emittable counts the actions whose record would be emitted as it stands:
	// a believed record with parse ok, writes complete, ZERO reasons, a plan
	// entry with NO gate and a weak key (an unkeyed action has nothing to be
	// emitted under; a gated one the merge turns into precondition:<gate>), and
	// not in a void shard (ShardVoid).  It is an UPPER BOUND: it is per action
	// and pre-merge, so it cannot see reads-product or any cross-action rule.
	// It is stricter than RecordWritesComplete: an action can be recordable
	// (parse ok, writes complete) and still carry a host-read reason, and
	// neither "clean" nor "recordable" means "would emit".
	Emittable int `json:"emittable"`
	// Gated counts actions whose plan entry carries a gate (the merge never
	// emits them); Unkeyed those whose plan entry has no weak key (a refusal
	// instead).  Both are counted over every action with a plan entry, whatever
	// its record says.
	Gated   int `json:"gated"`
	Unkeyed int `json:"unkeyed"`
	// WritesUnknown counts the actions whose write set is unknown: no believed record (record.failed, or no verdict at all), or a believed
	// record with writes_complete false, plus record.failed.  A strict merge
	// voids the run for any of them.
	WritesUnknown int `json:"writes_unknown"`
	// ShardVoid says collect's header lists a void reason, so Emittable is 0
	// whatever the records say (unknown without --records, then false).
	ShardVoid bool `json:"shard_void"`
	// Truncated counts the actions whose write set is KNOWN but whose execution
	// was cut short, which the merge classes as unkTruncated (recmerge
	// merge.go, `unknown`, last switch, and M-1): a believed record with a
	// `failed` reason (the action's status is not built), a `daemon-left`
	// reason, or fewer steps run than the plan lists (steps-missing).  It
	// mirrors that function as far as the census's inputs allow: the census has
	// the classified record and the plan, not the shard record's status, so
	// `failed` stands for status != built and len(plan steps) for steps_total.
	// An action already in WritesUnknown (a hard unknown) is not counted again.
	// A strict merge voids the run for any of them; the relaxed one only loses
	// that action.  Before the recipe prefix rule (KILN04 D36) a failed action
	// that stopped mid-recipe was a hard unknown instead.
	Truncated int `json:"truncated"`
	// RelaxedWouldLose is the number of actions the relaxed merge would drop
	// (relaxLost, merge.go voidStrict): Truncated.  It only applies when the
	// relaxed merge does not void the run, see RelaxedMergeVoidsShard.
	RelaxedWouldLose int `json:"relaxed_would_lose"`
	// RelaxedMergeVoidsShard: WritesUnknown > 0 or ShardVoid.  A hard unknown
	// voids the run in both verdicts.
	RelaxedMergeVoidsShard bool `json:"relaxed_merge_voids_shard"`
	// StrictMergeVoidsShard: WritesUnknown > 0, Truncated > 0 or ShardVoid.
	StrictMergeVoidsShard bool `json:"strict_merge_voids_shard"`
}

// KindSummary is the histogram of one reason kind.
type KindSummary struct {
	// Ops, for host-read, counts the occurrences by the kind of look that raised
	// them (traced.OpClasses: listing, absent-probe, open, stat, lookup, exec,
	// ...).  Omitted for a kind that is not split.
	Ops           map[string]int64 `json:"ops,omitempty"`
	Kind          string           `json:"kind"`
	Occurrences   int64            `json:"occurrences"`
	Overflow      int64            `json:"overflow"`
	Actions       int              `json:"actions"`
	DistinctPaths int              `json:"distinct_paths"`
	DroppedPaths  int64            `json:"dropped_paths"`
	Prefixes      []PrefixRow      `json:"prefixes"`
	TopPaths      []PathRow        `json:"top_paths"`
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
// their ratio, with the counts over each limit.  G2 is a REPORTED TARGET, never
// a gate (KILN04 D49): nothing in the census exit status or CI reads these, and
// `G2Failing` counts traces over either old limit for the record only.
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
	// G2Target is the reported G2 target (KILN04 D49): parse share at p50 at
	// most 10%, with this run's image and parse throughput beside it.
	G2Target G2Target `json:"g2_target"`
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

// G2TargetShare is the G2 target: parse share of step wall time at p50.
const G2TargetShare = 0.10

// G2Target is the reported G2 target.  Reported, not a gate (KILN04 D49).
type G2Target struct {
	TargetShareP50 float64 `json:"target_share_p50"`
	ShareP50       float64 `json:"share_p50"`
	// Met is whether ShareP50 is at or under the target; false when no share
	// was measured (ShareN is 0).
	Met    bool `json:"met"`
	ShareN int  `json:"share_n"`
	// ImageID and ParseMBPerSP50 are the per-image parse throughput: one census
	// covers one runner image, so the image id names the figure.
	ImageID        string  `json:"image_id"`
	ParseMBPerSP50 float64 `json:"parse_mb_per_s_p50"`
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
	Traces       int   `json:"traces"`
	Bytes        int64 `json:"bytes"`
	Accepted     int   `json:"accepted"`
	Refused      int   `json:"refused"`
	Skipped      int   `json:"skipped_too_large"`
	SkippedBytes int64 `json:"skipped_bytes"`
	Mismatches   int   `json:"mismatches"`
	Allowed      int   `json:"allowed"`
	// AllowedRows is the histogram of Allowed by the NAME of the spec's
	// stricter-only row that allowed each (a fixed program string of
	// internal/trace/crosscheck.go, never trace text).
	AllowedRows map[string]int `json:"allowed_rows"`
	Details     []string       `json:"details"`
}

// ShapeSummary is the redacted outline of the line a refusal or an allowed
// divergence stopped at, with how often the same outline recurred.  Every
// token of the outline is a type name, a character class, a closed-table
// syscall name or ASCII punctuation (trace.Shape); never a byte of a path, a
// string or an annotation.
type ShapeSummary struct {
	// Origin is "refusal:<parse code>" (the fixed code of the refusal) or
	// "allowed:<row>" (the stricter-only row's fixed name).
	Origin string `json:"origin"`
	Count  int    `json:"count"`
	// Where is the first occurrence: the action's digest prefix, the step and
	// the line.  It carries no text from any file.
	Where string `json:"where"`
	// Lines are the pid's pending `<unfinished ...>` half (when there is one)
	// and then the line where the grammar stopped.
	Lines []ShapeLine `json:"lines"`
}

// ShapeLine is one line of a ShapeSummary.
type ShapeLine struct {
	Role      string `json:"role"`
	Line      int    `json:"line"`
	Tokens    string `json:"tokens"`
	Facts     string `json:"facts"`
	Truncated bool   `json:"truncated"`
}

// LangSummary is one language's recordable share.
type LangSummary struct {
	Language   string  `json:"language"`
	Actions    int     `json:"actions"`
	Recordable int     `json:"recordable"`
	Clean      int     `json:"clean"`
	Share      float64 `json:"recordable_share"`
	// BuiltOK and Emittable are as ActionCounts', for this language.
	BuiltOK       int `json:"built_ok"`
	Emittable     int `json:"emittable"`
	Gated         int `json:"gated"`
	Unkeyed       int `json:"unkeyed"`
	WritesUnknown int `json:"writes_unknown"`
	// StepWallS is this language's step wall time in seconds (n, p50, max), from
	// the step sidecars' stamps; see census_stepwall.go.
	StepWallS LangWall `json:"step_wall_s"`
	// StepWallFailedS is the same for the steps whose root status was not zero
	// (a subset of StepWallS).  A stalled fetch is a long step that then fails;
	// this separates it from a slow successful compile.
	StepWallFailedS LangWall `json:"step_wall_failed_s"`
	// Truncated and RelaxedWouldLose are as ActionCounts', for this language.
	Truncated        int `json:"truncated"`
	RelaxedWouldLose int `json:"relaxed_would_lose"`
	// The shares: of all actions, and of the actions whose build did not fail
	// (0 when there are none).  RecordableShare is the existing Share over all.
	RecordableShareOfBuiltOK float64 `json:"recordable_share_of_built_ok"`
	EmittableShare           float64 `json:"emittable_share"`
	EmittableShareOfBuiltOK  float64 `json:"emittable_share_of_built_ok"`
}

// CollectSummary is what collect's records.jsonl header said.
type CollectSummary struct {
	ShardIndex                int      `json:"shard_index"`
	ShardCount                int      `json:"shard_count"`
	Void                      []string `json:"void"`
	PostInventoryUnexplained  int      `json:"post_inventory_unexplained"`
	TrackedChangedUnexplained int      `json:"tracked_changed_unexplained"`
	Records                   int64    `json:"records"`
	// UnknownReasons counts this shard's records whose write set collect marked
	// unknown, by the fixed code of the reason (unknownReasonCode): always the
	// five keys serial-violated, interrupted, not-run, exechead-seal and other.
	UnknownReasons map[string]int `json:"unknown_reasons"`
}

// ParseRulesSummary counts the firings of the two KILN04 D50 parse rules, so the
// next census shows whether they matter.  RestartedSpawns: fork, vfork or clone
// calls that ended `= ? ERESTARTNOINTR` and were dropped (rule 2b).  BareOpens:
// successful read-only opens with no -y annotation that were accepted (rule 2c).
type ParseRulesSummary struct {
	RestartedSpawns int64 `json:"restarted_spawns"`
	BareOpens       int64 `json:"bare_opens"`
}

// ExecHeadSummary is what the census says about exec heads (KILN04 D48 step 6).
// Every key of every map is a member of a closed list written here (the
// thirteen exec-unreadable sub-codes, the supervisor's six unclean reasons and
// the five exec-head parse reasons), and every one is always present, zero
// included, so a gate reads "none" as a fact and not as an absent key.  Nothing
// here is a path.
type ExecHeadSummary struct {
	// Sealed says the shard ran with exec heads: the sealed binding says
	// exec_head (prepare --exec-head).  Everything below is zero when it is false
	// and nothing was supervised.
	Sealed bool `json:"sealed"`
	// KernelTrusted says the binding's kernel is on the trusted table (series
	// 7.0), where `exechead-kernel` must never fire.
	KernelTrusted bool `json:"kernel_trusted"`
	// SupervisedSteps counts the step sidecars that carry an exec-head nonce.
	SupervisedSteps int `json:"supervised_steps"`
	// SubCodes counts the occurrences of each `trace-incomplete:exec-unreadable:
	// exechead-*` sub-code in the actions' census files (the classifier's own
	// count, before its per-kind cap).
	SubCodes map[string]int `json:"sub_codes"`
	// Unclean counts the steps unclean for each of the supervisor's reasons
	// (handover, receiver, failed, overflow, stuck, stall).
	Unclean map[string]int `json:"unclean"`
	// ParseReasons counts the actions whose record.json refused an exec-head file
	// (missing, unclaimed, oversize, digest, malformed).
	ParseReasons map[string]int `json:"parse_reasons"`
	// Kills is the number of times the supervisor ended a step itself, summed over
	// steps, and KillSteps the steps it did that to; Notifications is the
	// supervisor's notification count summed over steps.
	Kills         int   `json:"kills"`
	KillSteps     int   `json:"kill_steps"`
	Notifications int64 `json:"notifications"`
	// GoEmittable is Actions.Emittable restricted to language go: the actions
	// whose `go test` binary under the fresh tmpfs this change exists to believe.
	GoEmittable int `json:"go_emittable"`
	// The vacuity counters.  A run can seal exec heads, change nothing and pass
	// every hard-zero criterion while nothing exercised the mechanism (census run
	// A: Rust-only shards, every action failed in 0.1 s, no step exec'd a binary
	// under the tmpfs, every sub-code 0).  A zero that nothing could have made
	// nonzero is not a result, so the census says how much could have:
	//   GoActions    Go actions in the shard;
	//   GoBuiltOK    Go actions whose build succeeded (as Actions.BuiltOK);
	//   TmpfsExecs   successful execs under the step tmpfs that reached the
	//                classifier's exec-head pairing, believed or refused with an
	//                exechead-* sub-code (traced.Census.TmpfsExecs).  The number
	//                that says the mechanism was exercised; it is at least the
	//                sum of the pairing's sub-codes (unless a bound saturated).
	GoActions  int   `json:"go_actions"`
	GoBuiltOK  int   `json:"go_built_ok"`
	TmpfsExecs int64 `json:"tmpfs_execs"`
	// SealMismatch counts the STARTED steps whose sidecar has an exec-head nonce
	// exactly when the seal says off (or none when it says on): the census's own
	// count of what collect voids a run for, so a gate does not depend on
	// collect's header being present.  It must be 0.
	SealMismatch int `json:"seal_mismatch"`
}

// execHeadSubCodes are the exec-unreadable sub-codes of D48 step 5, in the order
// the census prints them.
var execHeadSubCodes = []string{
	"exechead-pair", "exechead-kernel", "exechead-outside", "exechead-gone", "exechead-mismatch",
	"exechead-timeout", "exechead-unpinned", "exechead-fexecve", "exechead-same-image",
	"exechead-failed", "exechead-overload", "exechead-tainted", "exechead-notelf",
}

// execHeadUncleanCodes are the supervisor's unclean reasons; execHeadParseCodes
// the parse reasons of an exec-head file that cannot be trusted.
var (
	execHeadUncleanCodes = []string{
		exechead.UncleanHandover, exechead.UncleanStall, exechead.UncleanStuck,
		exechead.UncleanReceiver, exechead.UncleanOverflow, exechead.UncleanFailed,
	}
	execHeadParseCodes = []string{"exechead-missing", "exechead-digest", "exechead-malformed", "exechead-unclaimed", "exechead-oversize"}
)

// execHeadSubKindPrefix starts the census kinds of the sub-codes.
const execHeadSubKindPrefix = "trace-incomplete:exec-unreadable:"

// satAdd adds two non-negative counts, saturating at 1<<62 instead of wrapping.
func satAdd(a, b int64) int64 {
	const top = 1 << 62
	if a < 0 || b < 0 || a > top-b {
		return top
	}
	return a + b
}

// noteExecHeadKind counts n occurrences of a census kind that is an exec-head
// sub-code; any other kind is not its business.
func (a *censusAcc) noteExecHeadKind(kind string, n int64) {
	code, ok := strings.CutPrefix(kind, execHeadSubKindPrefix)
	if !ok {
		return
	}
	for _, c := range execHeadSubCodes {
		if c == code {
			a.execHeadSub[code] += n
			return
		}
	}
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
	// ShapesDropped counts the refusals and allowed divergences whose outline
	// was not kept: more than maxCensusShapes distinct outlines, or more than
	// maxCensusShapeCalls outline computations in this census.
	ShapesDropped int `json:"shapes_dropped"`
}

// ---------------------------------------------------------------------------
// The accumulator
// ---------------------------------------------------------------------------

type prefixAcc struct {
	paths int
	count int64
}

type kindAcc struct {
	ops                            map[string]int64
	occurrences, overflow, dropped int64
	actions                        int
	paths                          map[string]int64
	prefixes                       map[string]*prefixAcc
}

type langAcc struct {
	actions, recordable, clean, builtOK, emittable, recordableBuilt int
	gated, unkeyed, writesUnknown, truncated                        int
	walls                                                           []float64 // step walls, ms
	failedWalls                                                     []float64 // the walls of steps with a non-zero root status, ms
}

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
	markerSeen  bool // traces/.state-preloaded is a regular file
	infra       string

	// The exec-head report (ExecHeadSummary): the seal, and the sums the walk makes.
	execHead                     ExecHeadSummary
	execHeadSub                  map[string]int64
	execHeadSteps, execHeadKills int
	execHeadKillSteps            int
	execHeadNotified             int64
	execHeadSealMismatch         int
	parseRules                   ParseRulesSummary

	// Line shapes (trace.Shape): the distinct outlines kept, by key, the number
	// of outline computations spent and the occurrences not kept.
	shapes        []ShapeSummary
	shapeIdx      map[string]int
	shapeCalls    map[string]int  // computations by class ("allowed", "refusal")
	shapeKept     map[string]int  // slots used by class
	shapeCode     map[string]int  // computations by refusal origin
	shapeSeen     map[string]bool // (trace path, line) already shaped
	shapeBytes    int64           // bytes charged against maxCensusShapeBytes
	shapesDropped int
}

func newCensusAcc(red *redactor) *censusAcc {
	return &censusAcc{red: red, explain: newExplain(), problems: map[string]int{}, kinds: map[string]*kindAcc{}, langs: map[string]*langAcc{},
		execHeadSub: map[string]int64{},
		cross:       CrossCheckSummary{AllowedRows: map[string]int{}}, shapeIdx: map[string]int{},
		shapeCalls: map[string]int{}, shapeKept: map[string]int{}, shapeCode: map[string]int{}, shapeSeen: map[string]bool{}}
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
	// CrossCheck is the cross-checked parse; nil means trace.ParseCrossCheckedDetail.
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
	// The sealed facts first: they decide how the invoker's own directories are
	// labelled (relabelRunner), and the redactor must exist before any text is
	// made.
	pre, err := VerifyPre(o.Dir)
	stepHome, stepTmp := "", ""
	if err == nil {
		stepHome, stepTmp = sealedHomeTmp(pre)
	}
	red := newRedactor(append(relabelRunner(o.Redact, stepHome, stepTmp), Redaction{From: o.Dir, To: "$RECORD_DIR"}))
	acc := newCensusAcc(red)
	run := &censusRun{o: o, acc: acc, crossCheck: o.CrossCheck}
	if run.crossCheck == nil {
		run.crossCheck = trace.ParseCrossCheckedDetail
	}
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
		// A marker is only a marker as a regular file: a directory or a link
		// with that name is somebody else's entry and is counted as stray below.
		isMarker := isTracesMarker(e.Name()) && e.Type().IsRegular()
		if isMarker && e.Name() == StatePreloadedMarker {
			acc.markerSeen = true
		}
		if acc.actions.Total >= maxCensusActions {
			acc.skipped++
			continue
		}
		if isMarker {
			continue // the recorder's own sequence counter, lock, collected marker and state-preloaded marker
		}
		if !hexDigest(e.Name()) || !e.Type().IsDir() {
			acc.problem("stray_traces_entry")
			continue
		}
		run.action(e.Name())
	}
	if o.Records != "" {
		acc.collect = readCollectSummary(o.Records, acc)
		// collect writes the marker and names the void in one pass; a marker the
		// header does not account for means the two disagree.
		if acc.collect != nil && acc.markerSeen {
			named := false
			for _, v := range acc.collect.Void {
				if v == voidStatePreloaded {
					named = true
				}
			}
			if !named {
				acc.problem("state_preloaded_marker_without_void")
			}
		}
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
	// lang is the language of the action being walked (as collapsed by
	// languageAcc), for the per-language breakdown of parse refusals.
	lang string
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
	// The language comes first: the parse-reason breakdown is by language.
	lang, la := r.languageAcc(name)
	r.lang = lang
	switch {
	case rec != nil:
		a.actions.WithRecord++
		if rec.ParseOK {
			a.actions.RecordParseOK++
		}
		if rec.WritesComplete {
			a.actions.RecordWritesComplete++
		}
		a.explainRecord(rec, lang)
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

	la.actions++
	if recordable {
		la.recordable++
		if state == censusEmpty {
			la.clean++
		}
	}
	built := builtOK(rec, scs)
	a.noteGoAction(lang, built)
	if built {
		a.actions.BuiltOK++
		la.builtOK++
		if recordable {
			la.recordableBuilt++
		}
	}
	// The plan's verdict on the action: a gate, or no weak key, means the merge
	// never emits it, however good its record.  No plan entry (no verified pre/,
	// an action not in the plan) means we cannot say: not emittable.
	planned, inPlan := Action{}, false
	if r.pre != nil && name != "" {
		planned, inPlan = r.pre.Action(name)
	}
	gated, unkeyed, mergeable := planVerdict(planned, inPlan)
	if gated {
		a.actions.Gated++
		la.gated++
	}
	if unkeyed {
		a.actions.Unkeyed++
		la.unkeyed++
	}
	// No believed record covers record.failed AND an action with no verdict at
	// all: either way nothing says what it wrote.
	if rec == nil || !rec.WritesComplete {
		a.actions.WritesUnknown++
		la.writesUnknown++
	} else if truncatedRec(rec, planned, inPlan) {
		a.actions.Truncated++
		la.truncated++
	}
	if mergeable && emittable(rec) {
		a.actions.Emittable++
		la.emittable++
		if lang == "go" {
			a.execHead.GoEmittable++
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

// languageAcc resolves the action's language and returns it with its
// accumulator, collapsing to "other" once maxCensusLanguages are held.
func (r *censusRun) languageAcc(name string) (string, *langAcc) {
	a := r.acc
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
	return lang, la
}

// builtOK says the action's build did not fail: a believed record with no
// `failed` reason.  A record whose parse half failed has no reasons at all (the
// classifier never ran), so for it the sidecars decide: no started step exited
// non-zero.  An action with no believed record is not built_ok: nothing says
// its build was fine.
func builtOK(rec *ClassifiedRecord, scs map[int]StepSidecar) bool {
	if rec == nil {
		return false
	}
	if rec.Record != nil {
		for _, r := range rec.Record.Reasons {
			if reasonKindCode(r) == "failed" {
				return false
			}
		}
		return true
	}
	for _, sc := range scs {
		if sc.Started && sc.RootStatus != 0 {
			return false
		}
	}
	return true
}

// planVerdict reads the plan's side of "would the merge emit this action": a
// gate (the merge turns it into precondition:<gate>) or no weak key (nothing to
// emit under) means never.  mergeable is false when there is no plan entry, too:
// nothing can be claimed for an action the verified pre/ does not know.
func planVerdict(planned Action, inPlan bool) (gated, unkeyed, mergeable bool) {
	if !inPlan {
		return false, false, false
	}
	gated, unkeyed = planned.Gate != "", planned.Weak == ""
	return gated, unkeyed, !gated && !unkeyed
}

// emittable says the RECORD is emittable: parse ok, writes complete and not one
// reason.  The caller adds the plan's side (no gate, a weak key), and summary
// the shard's (collect's void list, once the header is read).
// truncatedRec mirrors the last switch of recmerge's unknown() (merge.go): a
// believed record whose write set is known but whose execution was cut short.
// `failed` is the stand-in for "status is not built" (the classifier raises it
// for any step that exited non-zero, was killed or has no exit line),
// `daemon-left` is the same reason, and a plan with more steps than ran is the
// merge's steps-missing.  Hard unknowns (writes_complete false, no record) are
// the caller's, and are not truncated here.
func truncatedRec(rec *ClassifiedRecord, planned Action, inPlan bool) bool {
	if rec == nil || !rec.ParseOK || rec.Record == nil {
		return false
	}
	for _, r := range rec.Record.Reasons {
		switch reasonKindCode(r) {
		case "failed", "daemon-left":
			return true
		}
	}
	return inPlan && rec.StepsRun < len(planned.Steps)
}

func emittable(rec *ClassifiedRecord) bool {
	return rec != nil && rec.ParseOK && rec.WritesComplete && rec.Record != nil && len(rec.Record.Reasons) == 0
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
	if len(doc.Ops) > maxCensusKinds {
		a.problem("census_bounds")
		return censusInvalid, nil
	}
	// The counts are summed across up to maxCensusActions files into int64s, so
	// one file's op counts are held to a sum no wrap-around can follow
	// (2^40 each, 2^18 files: 2^58).
	var opsSum int64
	for k, m := range doc.Ops {
		if !censusKindRE.MatchString(k) || len(m) > len(traced.OpClasses) {
			a.problem("census_bad_entry")
			return censusInvalid, nil
		}
		for class, n := range m {
			if !traced.IsOpClass(class) || n < 0 || n > maxCensusOpsSum {
				a.problem("census_bad_entry")
				return censusInvalid, nil
			}
			if opsSum += n; opsSum > maxCensusOpsSum {
				a.problem("census_bad_entry")
				return censusInvalid, nil
			}
		}
	}
	for _, n := range []int64{doc.ParseRestartedSpawns, doc.ParseBareOpens} {
		if n < 0 || n > traced.MaxParseRuleCount {
			a.problem("census_bad_entry")
			return censusInvalid, nil
		}
	}
	if doc.ExecHeadTmpfsExecs < 0 || doc.ExecHeadTmpfsExecs > traced.MaxTmpfsExecs {
		a.problem("census_bad_entry")
		return censusInvalid, nil
	}
	// The counter is DIAGNOSTIC.  The classifier bumps it before it raises any
	// exechead-* sub-code, so a file's count can never be below the sum of its own
	// exechead-* occurrences (entries AND the overflow the classifier or the
	// encoder shed): a smaller one contradicts itself and the file is refused.  That
	// is all that can be checked.  A forged value on a clean file (no sub-codes) is
	// undetectable here; it can only make a vacuous shard look exercised, and it
	// never hides a hard-zero criterion, which reads other fields.  The cap is a
	// saturation point, so a count held at it is not held to the sum.
	var ehSum int64
	for _, e := range doc.Entries {
		if isExecHeadSubKind(e.Kind) {
			ehSum = satAdd(ehSum, e.Count)
		}
	}
	for k, n := range doc.Overflow {
		if isExecHeadSubKind(k) {
			ehSum = satAdd(ehSum, n)
		}
	}
	if doc.ExecHeadTmpfsExecs < ehSum && doc.ExecHeadTmpfsExecs < traced.MaxTmpfsExecs {
		a.problem("census_bad_entry")
		return censusInvalid, nil
	}
	// Counted here, for every valid file, EMPTY ONES TOO: a believed exec raises
	// no reason, so an action whose every exec was believed has an empty census
	// and must still count as having exercised the mechanism.
	a.execHead.TmpfsExecs = satAdd(a.execHead.TmpfsExecs, doc.ExecHeadTmpfsExecs)
	a.parseRules.RestartedSpawns = satAdd(a.parseRules.RestartedSpawns, doc.ParseRestartedSpawns)
	a.parseRules.BareOpens = satAdd(a.parseRules.BareOpens, doc.ParseBareOpens)
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

// isExecHeadSubKind says kind is one of the exechead-* exec-unreadable census kinds
// (the ones the classifier raises from the exec-head pairing).
func isExecHeadSubKind(kind string) bool {
	code, ok := strings.CutPrefix(kind, execHeadSubKindPrefix)
	return ok && strings.HasPrefix(code, "exechead-")
}

// noteGoAction counts one action for the vacuity counters go_actions and
// go_built_ok; any other language counts nothing.
func (a *censusAcc) noteGoAction(lang string, built bool) {
	if lang != "go" {
		return
	}
	a.execHead.GoActions++
	if built {
		a.execHead.GoBuiltOK++
	}
}

// addCensus folds one valid census into the histograms.
func (a *censusAcc) addCensus(doc *CensusDoc) {
	seen := map[*kindAcc]bool{}
	for kind, m := range doc.Ops {
		k := a.kindFor(kind)
		if k.ops == nil {
			k.ops = map[string]int64{}
		}
		for class, n := range m {
			k.ops[class] += n // class is from the closed list (readCensus checked)
		}
	}
	for kind, n := range doc.Overflow {
		a.noteExecHeadKind(kind, n)
		k := a.kindFor(kind)
		k.overflow += n
		seen[k] = true
	}
	for _, e := range doc.Entries {
		a.noteExecHeadKind(e.Kind, e.Count)
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

// CrossCheckFunc is the cross-checked parse (trace.ParseCrossCheckedDetail's shape).
// CensusOptions.CrossCheck injects it, so a test can stand in a mismatch or an
// allowed divergence, which no real trace is known to produce, without a
// package-level variable.
type CrossCheckFunc = func(io.Reader, string, trace.Options) (*trace.Trace, error, *trace.CrossCheckMismatch, trace.CrossCheckDetail)

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
		// The same wall, filed under the action's language (r.lang is set by
		// the action loop before its traces are walked).
		if la := a.langs[r.lang]; la != nil {
			la.walls = append(la.walls, wallMS)
			if sc.RootStatus != 0 {
				la.failedWalls = append(la.failedWalls, wallMS)
			}
		}
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
		code := parseReasonCode(why)
		a.explain.ParseRefusals[code]++
		a.explain.ParseRefusalsByLanguage.add(r.lang, code)
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
	var detail trace.CrossCheckDetail
	ok := a.guard(func() {
		_, perr, m, detail = r.crossCheck(cf, pkgDir, helperParseOptions(r.pre.Config.KilnExe, limits))
	})
	cf.Close()
	if !ok {
		return size
	}
	cc := &a.cross
	refusalOrigin, refusalLine := "", 0
	cc.Traces++
	cc.Bytes += size
	switch {
	case m != nil:
		cc.Mismatches++
		a.detail(fmt.Sprintf("%s step %d: mismatch: %s", hexName12(adir), j, m.Detail))
	case perr != nil:
		cc.Refused++
		// Which call: the closed-table name the parse error carries (see
		// refusalSyscall), by reason and language.  Census only.
		var ce *trace.Error
		errors.As(perr, &ce)
		code := parseReasonCode(perr.Error())
		sys := refusalSyscall(code, ce)
		a.explain.ParseRefusalSyscalls.add(code, sys)
		a.explain.ParseRefusalSyscallsByLanguage.add(r.lang, code, sys)
		// Where the grammar stopped, as a redacted outline: the fixed code of the
		// refusal says why, the outline says what the line looked like.
		var te *trace.Error
		if errors.As(perr, &te) && te.Line > 0 {
			// te.Line is the error's own line: for an unresumed-call that is the
			// call's START (the unfinished half), not where the input ended.
			refusalOrigin, refusalLine = "refusal:"+parseReasonCode(perr.Error()), te.Line
		}
	default:
		cc.Accepted++
	}
	if detail.Allowed > 0 {
		cc.Allowed += detail.Allowed
		r.noteAllowed(adir, path, j, size, detail)
	}
	// The refusal's outline comes AFTER the allowed divergences': when one line
	// is both, the (trace, line) dedupe must leave it labelled allowed:<row>,
	// which is the line the default flip needs, not refusal:<code>.
	if refusalLine > 0 {
		r.noteShape(adir, path, j, size, refusalOrigin, refusalLine)
	}
	return size
}

// noteAllowed reports one trace's allowed stricter-only divergences: the
// histogram by row name, one detail per distinct row (its first line), and an
// outline of the line of each hit.  The row names are fixed program strings
// (trace.CrossCheckDetail); the cut and the cap below only bound what a
// misbehaving injected CrossCheck could return.
func (r *censusRun) noteAllowed(adir, path string, j int, size int64, d trace.CrossCheckDetail) {
	a := r.acc
	for row, n := range d.Rows {
		name, _ := safeText(row, 120, a.red)
		if _, known := a.cross.AllowedRows[name]; !known && len(a.cross.AllowedRows) >= maxCensusAllowedRows {
			name = "(other)"
		}
		a.cross.AllowedRows[name] += n
	}
	seen := map[string]bool{}
	for _, h := range d.Hits {
		name, _ := safeText(h.Row, 120, a.red)
		if !seen[name] {
			seen[name] = true
			a.detail(fmt.Sprintf("%s step %d: allowed stricter-only divergence, row: %s (%d here, first at line %d)", hexName12(adir), j, name, d.Rows[h.Row], h.Line))
		}
		r.noteShape(adir, path, j, size, "allowed:"+name, h.Line)
	}
}

// noteShape computes the redacted outline of a line (trace.Shape) and folds it
// into the census's few kept outlines.  The work is bounded three ways (see the
// budgets at maxCensusShapes): by class (allowed:* and refusal:* never share a
// budget), per refusal code, and in bytes scanned.  A (trace, line) already
// shaped is not shaped again.  What was not kept is counted
// (bounds.shapes_dropped).
func (r *censusRun) noteShape(adir, path string, j int, size int64, origin string, line int) {
	a := r.acc
	if line <= 0 {
		return
	}
	seenKey := path + "|" + strconv.Itoa(line)
	if a.shapeSeen[seenKey] {
		return
	}
	class, callCap := "refusal", maxCensusShapeCallsRefus
	if strings.HasPrefix(origin, "allowed:") {
		class, callCap = "allowed", maxCensusShapeCallsAllow
	}
	cost := 2 * size // at most two streaming passes
	if a.shapeCalls[class] >= callCap || a.shapeBytes+cost > maxCensusShapeBytes ||
		(class == "refusal" && a.shapeCode[origin] >= maxCensusShapeCallsCode) {
		a.shapesDropped++
		return
	}
	a.shapeSeen[seenKey] = true
	a.shapeCalls[class]++
	a.shapeCode[origin]++
	a.shapeBytes += cost
	var shapes []trace.LineShape
	if !a.guard(func() {
		shapes = trace.Shape(func() (io.ReadCloser, error) {
			f, err := openNoFollow(path)
			if err != nil {
				return nil, err
			}
			return f, nil
		}, line)
	}) || len(shapes) == 0 {
		return
	}
	key := origin
	lines := make([]ShapeLine, 0, len(shapes))
	for _, s := range shapes {
		key += "\n" + s.Role + "\n" + s.Tokens + "\n" + s.Facts
		lines = append(lines, ShapeLine{Role: s.Role, Line: s.Line, Tokens: s.Tokens, Facts: s.Facts, Truncated: s.Truncated})
	}
	if i, ok := a.shapeIdx[key]; ok {
		a.shapes[i].Count++
		return
	}
	if a.shapeKept[class] >= maxCensusShapes {
		a.shapesDropped++
		return
	}
	a.shapeKept[class]++
	a.shapeIdx[key] = len(a.shapes)
	a.shapes = append(a.shapes, ShapeSummary{
		Origin: origin, Count: 1, Lines: lines,
		Where: fmt.Sprintf("%s step %d line %d", hexName12(adir), j, line),
	})
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
	// Records: the lines after the header, in what was already read and in the
	// rest of the file, and the fixed code of each unknown-write-set reason.
	cs.UnknownReasons = map[string]int{}
	for _, c := range unknownCodes {
		cs.UnknownReasons[c] = 0
	}
	cs.Records = scanRecordLines(io.MultiReader(bytes.NewReader(head[nl+1:]), f), maxCensusRecordsBytes-int64(len(head)), cs.UnknownReasons)
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
		Problems: map[string]int{}, Actions: a.actions, Explain: a.explain, Collect: a.collect, StatePreloadedMarker: a.markerSeen, ExecHead: a.execHeadSummary(), ParseRules: a.parseRules,
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
		ks := KindSummary{Ops: k.ops, Kind: kind, Occurrences: k.occurrences, Overflow: k.overflow, Actions: k.actions,
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
	s.Timing.G2Target = G2Target{
		TargetShareP50: G2TargetShare, ShareP50: s.Timing.ParseShare.P50,
		Met: s.Timing.ParseShare.N > 0 && s.Timing.ParseShare.P50 <= G2TargetShare, ShareN: s.Timing.ParseShare.N,
		ImageID: a.host.ImageID, ParseMBPerSP50: s.Timing.ParseMBPerS.P50,
	}
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
	s.CrossCheck.AllowedRows = map[string]int{}
	for k, v := range a.cross.AllowedRows {
		s.CrossCheck.AllowedRows[k] = v
	}
	names := make([]string, 0, len(a.langs))
	for l := range a.langs {
		names = append(names, l)
	}
	sort.Strings(names)
	// A collect header that names a void reason voids the whole shard: nothing
	// in it is emitted, whatever each record says.
	gated := a.collect != nil && len(a.collect.Void) > 0
	s.Actions.ShardVoid = gated
	if gated {
		s.Actions.Emittable = 0
	}
	s.Actions.RelaxedWouldLose = s.Actions.Truncated
	s.Actions.RelaxedMergeVoidsShard = gated || s.Actions.WritesUnknown > 0
	s.Actions.StrictMergeVoidsShard = gated || s.Actions.WritesUnknown > 0 || s.Actions.Truncated > 0
	ratio := func(n, d int) float64 {
		if d == 0 {
			return 0
		}
		return round3(float64(n) / float64(d))
	}
	for _, l := range names {
		la := a.langs[l]
		emit := la.emittable
		if gated {
			emit = 0
		}
		ls := LangSummary{Language: l, Actions: la.actions, Recordable: la.recordable, Clean: la.clean,
			BuiltOK: la.builtOK, Emittable: emit, Gated: la.gated, Unkeyed: la.unkeyed, WritesUnknown: la.writesUnknown,
			Truncated: la.truncated, RelaxedWouldLose: la.truncated,
			StepWallS:                langWallOf(la.walls),
			StepWallFailedS:          langWallOf(la.failedWalls),
			Share:                    ratio(la.recordable, la.actions),
			RecordableShareOfBuiltOK: ratio(la.recordableBuilt, la.builtOK),
			EmittableShare:           ratio(emit, la.actions),
			EmittableShareOfBuiltOK:  ratio(emit, la.builtOK),
		}
		s.Languages = append(s.Languages, ls)
	}
	s.Shapes = append([]ShapeSummary{}, a.shapes...)
	s.Bounds = BoundsSummary{
		JSONMaxBytes: MaxSummaryJSONBytes, MDMaxBytes: MaxSummaryMDBytes, TopPerKind: top,
		PathsTruncated: a.pathsTruncated, PathsDropped: a.pathsDropped, PrefixesDropped: a.prefixesDropped,
		KindsCollapsed: a.kindsCollapsed, LanguagesCollapsed: a.languagesCollapsed, ActionsSkipped: a.skipped,
		ShapesDropped: a.shapesDropped,
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
	if sc.Started && !sc.StraceStderrEmpty {
		c := sc.StraceStderrCode
		if c == "" {
			c = StraceErrUnrecorded
		}
		f.StraceStderrCodes[c]++ // Validate allows only the closed list
		if sc.RootStatus != 0 {
			f.StraceStderrFailedSteps[c]++ // the same closed list, so equally bounded
		}
		if sc.StraceExit >= 0 {
			f.StraceExits[StraceExitLabel(sc.StraceExit)]++
		}
	}
	if sc.Started && sc.StraceSignal != 0 {
		f.StraceSignals[StraceSignalLabel(sc.StraceSignal)]++
	}
	if sc.Exit125 {
		f.Exit125++
	}
	if sc.Started && (sc.ExecHeadNonce != "") != a.execHead.Sealed {
		a.execHeadSealMismatch++
	}
	if sc.ExecHeadNonce != "" {
		a.execHeadSteps++
		// Saturating sums: each count is at most MaxExecHeadCount (Validate), but
		// enough sidecars of that would still wrap.
		a.execHeadNotified = satAdd(a.execHeadNotified, int64(sc.ExecHeadNotified))
		a.execHeadKills = int(satAdd(int64(a.execHeadKills), int64(sc.ExecHeadKills)))
		if sc.ExecHeadKills > 0 {
			a.execHeadKillSteps++
		}
	}
	if sc.Started && sc.RootStatus != 0 {
		f.RootNonZero++
	}
}

// explainRecord counts the path-free reasons of a believed record.json: the
// kind of each classifier reason, and each parse reason's code, once per action.
func (a *censusAcc) explainRecord(rec *ClassifiedRecord, lang string) {
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
			a.explain.RecordParseReasonsByLanguage.add(lang, c)
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

// sealedHomeTmp are the HOME and TMPDIR the STEP ran with, from the sealed host
// facts of pre/ (the classifier labels paths under them $HOME and $TMPDIR).  ""
// when they cannot be read.
func sealedHomeTmp(pre *Pre) (home, tmp string) {
	data, err := pre.ReadSealed("host-facts.json")
	if err != nil {
		return "", ""
	}
	hf, err := recfmt.DecodeHostFacts(data)
	if err != nil {
		return "", ""
	}
	return hf.Home, hf.Tmpdir
}

// relabelRunner keeps `$HOME` and `$TMPDIR` meaning ONE thing in a report: the
// step's.  The classifier labelled the step's fresh HOME `$HOME` and its TMPDIR
// `$TMPDIR` in every path it wrote (traced redact), but `kiln record census`
// runs with the RUNNER's real HOME and temporary directory and used to label
// those the same way, so one string named two different directories.  A
// redaction whose directory is the sealed step's own keeps its label (it is the
// same directory); any other `$HOME` / `$TMPDIR` is the invoker's and becomes
// `$RUNNER_HOME` / `$RUNNER_TMP`.  When the sealed facts are unknown (stepHome
// and stepTmp are "") every one is the invoker's.  Other labels are untouched.
func relabelRunner(in []Redaction, stepHome, stepTmp string) []Redaction {
	out := make([]Redaction, len(in))
	// A trailing slash is not a different directory (newRedactor drops it too).
	stepHome, stepTmp = strings.TrimRight(stepHome, "/"), strings.TrimRight(stepTmp, "/")
	for i, r := range in {
		out[i] = r
		from := strings.TrimRight(r.From, "/")
		switch {
		case r.To == "$HOME" && (stepHome == "" || from != stepHome):
			out[i].To = "$RUNNER_HOME"
		case r.To == "$TMPDIR" && (stepTmp == "" || from != stepTmp):
			out[i].To = "$RUNNER_TMP"
		}
	}
	return out
}

// execHeadSummary assembles the exec-head block: the walk's sums, and the
// per-code tables with every closed-list key present.
func (a *censusAcc) execHeadSummary() ExecHeadSummary {
	e := a.execHead
	e.SupervisedSteps, e.Kills, e.KillSteps, e.Notifications = a.execHeadSteps, a.execHeadKills, a.execHeadKillSteps, a.execHeadNotified
	e.SealMismatch = a.execHeadSealMismatch
	e.SubCodes, e.Unclean, e.ParseReasons = map[string]int{}, map[string]int{}, map[string]int{}
	for _, c := range execHeadSubCodes {
		e.SubCodes[c] = int(min(a.execHeadSub[c], 1<<31-1))
	}
	for _, c := range execHeadUncleanCodes {
		e.Unclean[c] = a.explain.Sidecar.Unclean[c]
	}
	for _, c := range execHeadParseCodes {
		e.ParseReasons[c] = a.explain.RecordParseReasons[c]
	}
	return e
}

// hostFrom fills the host block from the verified pre/ (binding and host facts)
// and the CPU model of this machine.  Every string is sanitised and cut.
func (a *censusAcc) hostFrom(pre *Pre, cpuinfo string) {
	cut := func(s string) string { t, _ := safeText(s, 200, a.red); return t }
	b := pre.Binding
	a.execHead.Sealed, a.execHead.KernelTrusted = b.ExecHead, traced.KernelTrusted(b.Kernel)
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
