// Package recfmt defines the data formats that move between a KILN04 stage 4
// recording shard and the merge job (code/specs/KILN04-traced-read-sets.md,
// section 7 and 7.1), and decodes them strictly.
//
// # Where it sits
//
//	shard job (stage 4, PR 2)                      merge job (stage 4, PR 3)
//	  writes pre/binding.json       \               reads them back with
//	         pre/host-facts.json     > recfmt  -->  recfmt.DecodeShard
//	         pre/inventory.json     /               and hands the result to
//	         records.jsonl                          recmerge.Merge (this PR)
//
// This package is PURE: it never opens a file.  The caller supplies bytes or
// an io.Reader, so every refusal below is tested with literal strings.
//
// # Why the decoder is so strict
//
// A shard is untrusted input.  The merge's soundness argument ("no action's
// observations overlap another action's writes") is only as good as the
// observation lists it was given, so a record that LOOKS valid but has been
// shortened must not be accepted.  Three kinds of laxness would allow that,
// and each is closed here:
//
//   - a MISSING key.  Go's decoder leaves an absent field at its zero value,
//     so {"writes": ...} without "untracked_observations" would silently mean
//     "observed nothing".  Every key that is not tagged omitempty is required,
//     and a list may not be null.
//   - a DUPLICATE or case-variant key.  Go's decoder lets the last duplicate
//     win and matches names case-insensitively.  legacy.PolicyDecode refuses
//     both, so what a reviewer reads in a shard file is what is parsed.
//   - an UNSORTED, DUPLICATED or MALFORMED path.  The merge treats the lists
//     as sets, but a record whose lists are not in the classifier's canonical
//     form was not produced by the classifier.  Every path list must hold
//     valid KILN02 paths (key.ValidatePath: no leading or trailing slash, no
//     "." or ".." segment, no empty segment, valid UTF-8), sorted bytewise and
//     without repeats.
//
// A path such as "a/b/" or "a//b" is refused for the same reason the key
// encoder refuses it: two spellings of one path would let an overlap check
// miss.  Names are case-sensitive ("A" and "a" are two paths), as they are on
// the Linux runners that record.
package recfmt

import (
	"fmt"
	"sort"
	"strings"
	"unicode"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// Schema names, one per file.  A version bump is a new name.
const (
	SchemaBinding   = "kiln-record-binding/1"
	SchemaHostFacts = "kiln-record-host-facts/1"
	SchemaInventory = "kiln-record-inventory/1"
	SchemaShard     = "kiln-record-shard/1"
)

// Caps.  A value over a cap is a format error (the whole shard is refused),
// never a truncation: a truncated list is a smaller set of observations.
const (
	// MaxList bounds every path list of a record, the same figure the
	// classifier uses (a list over it is already a reason there).
	MaxList = 1 << 16
	// MaxInventory bounds the pre-BUILD inventory.
	MaxInventory = 1 << 21
	// MaxPath bounds one path of a record (a PATH_MAX is 4096).
	MaxPath = 4096
	// MaxDepth bounds the segments of one path.  No real checkout nests
	// 128 deep; a path of 1,500 one-letter segments only exists to make every
	// ancestor probe of the merge hash a long prefix.
	MaxDepth = 128
	// MaxText bounds a free-text field.
	MaxText = 4096
	// MaxSteps bounds the per-step summaries of one action.
	MaxSteps = 1 << 12
	// MaxShards bounds shard_count.
	MaxShards = 1 << 12
	// MaxEnv bounds the scrubbed environment of the host facts.
	MaxEnv = 1 << 12
	// MaxReasons bounds the reasons of one record.
	MaxReasons = 1 << 12
	// MaxReasonsPerKind is the classifier's per-kind cap (traced.CapReasons).
	MaxReasonsPerKind = 16
	// MaxReasonKinds bounds the DISTINCT reason kinds of one record.  The
	// classifier emits about a dozen (trace-incomplete, host-read, network,
	// daemon, daemon-left, failed, writes-tracked, state-write, trace-visible,
	// vcs-read, ...); 64 leaves a wide margin and stops a record from
	// carrying hundreds of kinds at 16 reasons each (a merge cost multiplier).
	MaxReasonKinds = 64
)

// Binding is pre/binding.json: what one shard claims about itself before any
// BUILD ran (KILN04 section 7, "Shard binding").  Its identity (binding_id) is
// the SHA-256 of its bytes, so every other file can name it.
type Binding struct {
	Schema     string `json:"schema"`
	Repository string `json:"repository"`
	Run        string `json:"run"`
	Attempt    int    `json:"attempt"`
	Revision   string `json:"revision"`
	ShardIndex int    `json:"shard_index"`
	ShardCount int    `json:"shard_count"`
	// The digests the merge job regenerates (graph, steps, partition) or
	// recomputes from the files beside the binding (host facts, inventory).
	GraphSHA256         string `json:"graph_sha256"`
	StepsSHA256         string `json:"steps_sha256"`
	PartitionSHA256     string `json:"partition_sha256"`
	HostFactsSHA256     string `json:"host_facts_sha256"`
	InventorySHA256     string `json:"inventory_sha256"`
	SandboxConfigSHA256 string `json:"sandbox_config_sha256"`
	KilnSHA256          string `json:"kiln_sha256"`
	BuildToolSHA256     string `json:"build_tool_sha256"`
	Recorder            int    `json:"recorder"`
	StraceVersion       string `json:"strace_version"`
	Kernel              string `json:"kernel"`
	JobStart            int64  `json:"job_start"`
	PreparedAt          string `json:"prepared_at"`
}

// HostFacts is pre/host-facts.json: the machine facts the weak key hashes and
// no other job can observe (KILN04 section 7.1, G4).  The merge job rebuilds
// each shard's weak keys from these plus its own Source.
//
// It deliberately satisfies legacy.Imager (ImageID), so it can be handed to
// legacy.Options as the Host.
type HostFacts struct {
	Schema string `json:"schema"`
	// Image is the runner image id, written image_id.
	Image  string `json:"image_id"`
	Root   string `json:"root"`
	Home   string `json:"home"`
	Tmpdir string `json:"tmpdir"`
	// ExecutorArgs are the executor's flags (legacy.CanonicalExecutorArgs).
	ExecutorArgs []string `json:"executor_args"`
	// Env is the scrubbed environment as "NAME=value" entries, selected by
	// EnvPolicy.EnvFor plus HOME and TMPDIR, never PATH (PATH is constructed).
	Env []string `json:"env"`
}

// ImageID makes HostFacts a legacy.Imager.
func (f HostFacts) ImageID() string { return f.Image }

// Inventory is pre/inventory.json: every path on the disk before the first
// BUILD that the tracked tree does not hold, a directory without a tracked
// file collapsed to one entry (the granularity of a write root).
type Inventory struct {
	Schema string   `json:"schema"`
	Paths  []string `json:"paths"`
}

// ShardHeader is the first line of records.jsonl.
type ShardHeader struct {
	Schema     string `json:"schema"`
	BindingID  string `json:"binding_id"`
	ShardIndex int    `json:"shard_index"`
	ShardCount int    `json:"shard_count"`
	// Void lists the shard-level problems the shard itself found:
	// "tracked-changed", "state-preloaded", "survivor", "records-too-large".
	Void                     []string `json:"void"`
	PostInventoryUnexplained int      `json:"post_inventory_unexplained"`
	// TrackedChangedUnexplained counts the tracked paths the shard's backstop
	// found changed that no record's tracked_writes covers (KILN04 7.1, M-2).
	// A tracked file changed by something no trace saw is an unknown write.
	TrackedChangedUnexplained int `json:"tracked_changed_unexplained"`
}

// Step is the per-step summary a record carries; the merge only counts them.
type Step struct {
	WallMS     int64 `json:"wall_ms"`
	TraceBytes int64 `json:"trace_bytes"`
	Clean      bool  `json:"clean"`
}

// Status values of a record.  Anything but StatusBuilt means the action did
// not pass its BUILD on this shard.
const StatusBuilt = "built"

// ShardRecord is one line of records.jsonl after the header: one action as one
// shard ran it.  An action appears in several shards (shards are
// prerequisite-closed, KILN04 section 7.1 G3); each appearance is a record.
type ShardRecord struct {
	BindingID  string    `json:"binding_id"`
	Action     string    `json:"action"`
	Language   string    `json:"language"`
	Status     string    `json:"status"`
	StepsRun   int       `json:"steps_run"`
	StepsTotal int       `json:"steps_total"`
	Steps      []Step    `json:"steps"`
	StepsSHA   string    `json:"steps_sha256"`
	Weak       string    `json:"weak"`
	Refusal    string    `json:"refusal"`
	Gate       string    `json:"gate"`
	Unknown    string    `json:"unknown_writes"`
	Record     RawRecord `json:"record"`
	// Complete is the optional "writes_complete" switch of KILN04 section 7.1
	// (G1).  The classifier computes it (traced.RawRecord.WritesComplete, a
	// Go-only field); the shard writer MUST derive it with CompleteOf, and
	// EncodeRecord refuses a record where it is missing or disagrees
	// (CheckComplete).  For recorder version 2 the key is REQUIRED on the
	// wire (no omitempty): a record without it is not decoded, and one that
	// reaches the merge in memory without it is an unknown write set whatever
	// its reasons say.  The merge honours a true value only when
	// Input.HonourWritesComplete is on.
	Complete *bool `json:"writes_complete"`
}

// RawRecord is the classifier's output, unchanged (internal/traced).  The wire
// form is the classifier's own JSON, so the shard can marshal a
// *traced.RawRecord directly.
type RawRecord = traced.RawRecord

// CompleteOf is the value of a shard record's "writes_complete" key for a
// freshly classified raw record: the classifier's own answer, never a default.
// (RawRecord.WritesComplete is not in the record's JSON, so the key travels
// on the shard record, where the merge reads it.)
func CompleteOf(r RawRecord) *bool {
	v := r.WritesComplete
	return &v
}

// CheckComplete is the check the shard writer's record must pass: the
// "writes_complete" key is present, and it says what the classifier said
// (r.Record.WritesComplete, available only in memory: it is not on the wire).
// A writer that forgot the key, defaulted it, or set it from anything but the
// classifier would hand the merge a claim nobody computed.
//
// A record that ran NOTHING (steps_run 0 and not built: refused or gated) has
// no classified raw record, so the flag is an explicit false and need not
// match one; the key stays required on the wire.  The merge charges such a
// record as a truncated execution (M-1), not as an unknown write set (KILN04
// 7.1).  A record that ran any step, failed or not, was classified, and its
// flag must agree with the classifier.
func CheckComplete(r ShardRecord) error {
	if r.Complete == nil {
		return fmt.Errorf("record %q: writes_complete is missing (derive it with CompleteOf)", short(r.Action))
	}
	if r.StepsRun == 0 && r.Status != StatusBuilt && !*r.Complete {
		return nil
	}
	if *r.Complete != r.Record.WritesComplete {
		return fmt.Errorf("record %q: writes_complete %v disagrees with the classifier's %v (derive it with CompleteOf)",
			short(r.Action), *r.Complete, r.Record.WritesComplete)
	}
	return nil
}

// Shard is one decoded shard artifact: its four files plus the artifact's own
// index (taken from the artifact NAME by the caller, so a record file cannot
// choose its own index).
type Shard struct {
	ArtifactIndex int
	Binding       Binding
	// BindingID is the SHA-256 of the binding file's bytes.
	BindingID string
	// HostFactsSHA256 and InventorySHA256 are the SHA-256 of the files' bytes;
	// the binding must name them (merge check B3).
	HostFactsSHA256 string
	InventorySHA256 string
	Facts           HostFacts
	Inventory       Inventory
	Header          ShardHeader
	Records         []ShardRecord
}

// ---------------------------------------------------------------------------
// Validation.  Every Validate is called by the decoder AND by the merge, so a
// record built in memory (a test, a fuzzer) is held to the same form.

func hexOK(s string, n int) bool {
	if len(s) != n {
		return false
	}
	for i := 0; i < len(s); i++ {
		c := s[i]
		if !(c >= '0' && c <= '9' || c >= 'a' && c <= 'f') {
			return false
		}
	}
	return true
}

// isDigest is 64 lowercase hex characters.
func isDigest(s string) bool { return hexOK(s, 64) }

func printable(what, s string, max int) error {
	if len(s) > max {
		return fmt.Errorf("%s is %d bytes, limit %d", what, len(s), max)
	}
	if !utf8.ValidString(s) {
		return fmt.Errorf("%s is not valid UTF-8", what)
	}
	for _, r := range s {
		// U+FFFD is what the decoder substitutes for a byte that was not text
		// (a lone \ud800 escape, say), so it is never a faithful character.
		if !unicode.IsPrint(r) || r == utf8.RuneError {
			return fmt.Errorf("%s holds a non-printable character", what)
		}
	}
	return nil
}

// short is a log-safe excerpt.
func short(s string) string {
	s = strings.Map(func(r rune) rune {
		if !unicode.IsPrint(r) {
			return '?'
		}
		return r
	}, s)
	if len(s) > 60 {
		s = strings.ToValidUTF8(s[:60], "?") + "..."
	}
	return s
}

// sortedUnique checks a list is strictly increasing bytewise.
func sortedUnique(what string, l []string) error {
	for i := 1; i < len(l); i++ {
		if l[i-1] >= l[i] {
			return fmt.Errorf("%s is not sorted and free of duplicates at %q", what, short(l[i]))
		}
	}
	return nil
}

func pathList(what string, l []string, max int) error {
	if len(l) > max {
		return fmt.Errorf("%s has %d entries, limit %d", what, len(l), max)
	}
	for _, p := range l {
		if err := checkPath(p); err != nil {
			return fmt.Errorf("%s: %v", what, err)
		}
	}
	return sortedUnique(what, l)
}

// checkPath is key.ValidatePath plus what a record path must also satisfy: a
// length cap, no NUL (a C string ends there, so two paths could read as one)
// and no U+FFFD (what the JSON decoder substitutes for a byte that is not
// UTF-8, so the path the merge sees would not be the path the shard wrote).
func checkPath(p string) error {
	if len(p) > MaxPath {
		return fmt.Errorf("path is %d bytes, limit %d", len(p), MaxPath)
	}
	if strings.Count(p, "/") >= MaxDepth {
		return fmt.Errorf("path is more than %d segments deep", MaxDepth)
	}
	if strings.ContainsRune(p, 0) || strings.ContainsRune(p, utf8.RuneError) {
		return fmt.Errorf("path %q holds a NUL or an invalid byte", short(p))
	}
	return key.ValidatePath(p)
}

func textList(what string, l []string, max, maxLen int) error {
	if len(l) > max {
		return fmt.Errorf("%s has %d entries, limit %d", what, len(l), max)
	}
	for _, s := range l {
		if s == "" {
			return fmt.Errorf("%s holds an empty element", what)
		}
		if err := printable(what, s, maxLen); err != nil {
			return err
		}
	}
	return sortedUnique(what, l)
}

// Validate checks a binding's form.  It does not compare it with anything.
func (b *Binding) Validate() error {
	switch {
	case b.Schema != SchemaBinding:
		return fmt.Errorf("binding: schema %q", short(b.Schema))
	case b.Repository == "" || printable("repository", b.Repository, 200) != nil || strings.Count(b.Repository, "/") != 1:
		return fmt.Errorf("binding: repository %q is not owner/name", short(b.Repository))
	case b.Run == "" || printable("run", b.Run, 40) != nil || strings.Trim(b.Run, "0123456789") != "":
		return fmt.Errorf("binding: run %q is not a run id", short(b.Run))
	case b.Attempt < 1:
		return fmt.Errorf("binding: attempt %d", b.Attempt)
	case !hexOK(b.Revision, 40) && !hexOK(b.Revision, 64):
		return fmt.Errorf("binding: revision %q is not a commit id", short(b.Revision))
	case b.ShardCount < 1 || b.ShardCount > MaxShards:
		return fmt.Errorf("binding: shard_count %d", b.ShardCount)
	case b.ShardIndex < 0 || b.ShardIndex >= b.ShardCount:
		return fmt.Errorf("binding: shard_index %d of %d", b.ShardIndex, b.ShardCount)
	case b.Recorder < 1:
		return fmt.Errorf("binding: recorder %d", b.Recorder)
	case b.JobStart < 0:
		return fmt.Errorf("binding: job_start %d", b.JobStart)
	}
	// Ordered slices, not maps: the FIRST failure is the one reported, and a
	// cause text must be the same bytes on every run (merge output is compared
	// byte for byte).
	for _, f := range []struct{ name, v string }{
		{"graph_sha256", b.GraphSHA256}, {"steps_sha256", b.StepsSHA256}, {"partition_sha256", b.PartitionSHA256},
		{"host_facts_sha256", b.HostFactsSHA256}, {"inventory_sha256", b.InventorySHA256},
		{"sandbox_config_sha256", b.SandboxConfigSHA256}, {"kiln_sha256", b.KilnSHA256},
		{"build_tool_sha256", b.BuildToolSHA256},
	} {
		if !isDigest(f.v) {
			return fmt.Errorf("binding: %s is not a SHA-256 in lowercase hex", f.name)
		}
	}
	for _, f := range []struct{ name, v string }{{"strace_version", b.StraceVersion}, {"kernel", b.Kernel}, {"prepared_at", b.PreparedAt}} {
		if err := printable("binding "+f.name, f.v, 200); err != nil {
			return err
		}
	}
	return nil
}

// Validate checks host facts' form.
func (f *HostFacts) Validate() error {
	if f.Schema != SchemaHostFacts {
		return fmt.Errorf("host facts: schema %q", short(f.Schema))
	}
	if f.Image == "" {
		return fmt.Errorf("host facts: image_id is empty")
	}
	if err := printable("image_id", f.Image, 200); err != nil {
		return fmt.Errorf("host facts: %v", err)
	}
	for _, p := range []struct{ name, v string }{{"root", f.Root}, {"home", f.Home}, {"tmpdir", f.Tmpdir}} {
		if p.v == "" && p.name != "home" && p.name != "tmpdir" {
			return fmt.Errorf("host facts: %s is empty", p.name)
		}
		if p.v != "" && (p.v[0] != '/' || printable(p.name, p.v, MaxText) != nil || hasDotDot(p.v)) {
			return fmt.Errorf("host facts: %s %q is not a clean absolute path", p.name, short(p.v))
		}
	}
	if len(f.ExecutorArgs) > MaxEnv || len(f.Env) > MaxEnv {
		return fmt.Errorf("host facts: too many executor args or environment entries")
	}
	for _, a := range f.ExecutorArgs {
		if a == "" {
			return fmt.Errorf("host facts: an executor arg is empty")
		}
		if err := printable("executor arg", a, MaxText); err != nil {
			return fmt.Errorf("host facts: %v", err)
		}
	}
	seen := map[string]bool{}
	for _, e := range f.Env {
		name, _, ok := strings.Cut(e, "=")
		if !ok || name == "" || printable("environment entry", e, 1<<16) != nil {
			return fmt.Errorf("host facts: environment entry %q is not NAME=value", short(e))
		}
		if strings.EqualFold(name, "PATH") {
			return fmt.Errorf("host facts: PATH is constructed from tools, never carried")
		}
		if seen[name] {
			return fmt.Errorf("host facts: environment name %q is repeated", short(name))
		}
		seen[name] = true
	}
	return nil
}

// Environ returns the environment as os/exec spells it (legacy.Options.Environ).
func (f HostFacts) Environ() []string { return append([]string(nil), f.Env...) }

// Validate checks an inventory's form.
func (i *Inventory) Validate() error {
	if i.Schema != SchemaInventory {
		return fmt.Errorf("inventory: schema %q", short(i.Schema))
	}
	return pathList("inventory paths", i.Paths, MaxInventory)
}

// Void tokens a header may carry; the merge job decides what each means.
var voidTokens = map[string]bool{"tracked-changed": true, "state-preloaded": true, "survivor": true, "records-too-large": true}

// KnownVoid reports whether a header void token is one the merge understands.
func KnownVoid(tok string) bool { return voidTokens[tok] }

// Validate checks a header's form.
func (h *ShardHeader) Validate() error {
	switch {
	case h.Schema != SchemaShard:
		return fmt.Errorf("shard header: schema %q", short(h.Schema))
	case !isDigest(h.BindingID):
		return fmt.Errorf("shard header: binding_id is not a SHA-256 in lowercase hex")
	case h.ShardCount < 1 || h.ShardCount > MaxShards || h.ShardIndex < 0 || h.ShardIndex >= h.ShardCount:
		return fmt.Errorf("shard header: shard %d of %d", h.ShardIndex, h.ShardCount)
	case h.PostInventoryUnexplained < 0 || h.TrackedChangedUnexplained < 0:
		return fmt.Errorf("shard header: a negative count (%d, %d)", h.PostInventoryUnexplained, h.TrackedChangedUnexplained)
	}
	return textList("shard header void", h.Void, 64, 64)
}

// kindOf is a reason's kind: the text before the first colon (traced.CapReasons).
func kindOf(r string) string {
	if i := strings.IndexByte(r, ':'); i >= 0 {
		return r[:i]
	}
	return r
}

// ReasonKind is the kind of a reason string, for the merge's rules.
func ReasonKind(r string) string { return kindOf(r) }

// Validate checks a record line's form.  It is the gate between "a shard said
// so" and "the merge reasons about it".
func (r *ShardRecord) Validate() error {
	if !isDigest(r.BindingID) {
		return fmt.Errorf("record: binding_id is not a SHA-256 in lowercase hex")
	}
	if err := checkPath(r.Action); err != nil {
		return fmt.Errorf("record action: %v", err)
	}
	if r.Record.Package != r.Action {
		return fmt.Errorf("record %q: its raw record is for package %q", short(r.Action), short(r.Record.Package))
	}
	for _, f := range []struct{ name, v string }{
		{"language", r.Language}, {"status", r.Status}, {"refusal", r.Refusal}, {"gate", r.Gate}, {"unknown_writes", r.Unknown},
	} {
		if err := printable(f.name, f.v, 200); err != nil {
			return fmt.Errorf("record %q: %v", short(r.Action), err)
		}
	}
	if !statusOK(r.Status) {
		return fmt.Errorf("record %q: status %q is not a lowercase word", short(r.Action), short(r.Status))
	}
	if r.StepsRun < 0 || r.StepsTotal < 0 || r.StepsRun > r.StepsTotal || r.StepsTotal > MaxSteps || len(r.Steps) != r.StepsRun {
		// One summary per step that RAN: a shorter list would hide an unclean
		// tracer, a longer one would claim steps that never ran.
		return fmt.Errorf("record %q: steps_run %d of %d with %d summaries", short(r.Action), r.StepsRun, r.StepsTotal, len(r.Steps))
	}
	if r.StepsSHA != "" && !isDigest(r.StepsSHA) {
		return fmt.Errorf("record %q: steps_sha256 is not a SHA-256 in lowercase hex", short(r.Action))
	}
	if r.StepsSHA == "" && r.Status == StatusBuilt {
		return fmt.Errorf("record %q: a built action must carry steps_sha256", short(r.Action))
	}
	if r.Weak != "" && !isDigest(r.Weak) {
		return fmt.Errorf("record %q: weak is not a SHA-256 in lowercase hex", short(r.Action))
	}
	if (r.Weak == "") == (r.Refusal == "") {
		return fmt.Errorf("record %q: exactly one of weak and refusal must be set", short(r.Action))
	}
	rr := &r.Record
	if r.StepsRun == 0 {
		// A record that ran nothing was never classified: any content in its
		// raw record is a recorder bug, and a classification nobody made must
		// not reach the merge (fail closed).
		if len(rr.Entries)+len(rr.Writes)+len(rr.TrackedWrites)+len(rr.AbsentObservations)+len(rr.Binds)+
			len(rr.UntrackedObservations)+len(rr.Traversed)+len(rr.Reasons) != 0 {
			return fmt.Errorf("record %q: ran no step but carries a classified raw record", short(r.Action))
		}
	}
	for _, l := range []struct {
		name string
		v    []string
	}{
		{"writes", rr.Writes}, {"tracked_writes", rr.TrackedWrites}, {"absent_observations", rr.AbsentObservations},
		{"untracked_observations", rr.UntrackedObservations}, {"traversed", rr.Traversed},
	} {
		if err := pathList(fmt.Sprintf("record %q %s", short(r.Action), l.name), l.v, MaxList); err != nil {
			return err
		}
	}
	if err := textList(fmt.Sprintf("record %q entries", short(r.Action)), rr.Entries, MaxList, MaxText); err != nil {
		return err
	}
	for _, e := range rr.Entries {
		// The manifest allowlist (pkg:, path:, list: and a valid path) applies
		// at decode, so no rule ever sees an entry of another shape.  Length
		// and count caps of the ledger are checked at emission.
		if err := key.CheckEntryName(e); err != nil {
			return fmt.Errorf("record %q: %v", short(r.Action), err)
		}
		// The path part gets every check a path gets: the caps, NUL and
		// U+FFFD (a lone surrogate escape decodes to U+FFFD).  "list:." names
		// the root and has no path part.
		if rest := e[strings.IndexByte(e, ':')+1:]; e != key.EntryList+"." {
			if err := checkPath(rest); err != nil {
				return fmt.Errorf("record %q entry: %v", short(r.Action), err)
			}
		}
	}
	if err := textList(fmt.Sprintf("record %q binds", short(r.Action)), rr.Binds, MaxList, 512); err != nil {
		return err
	}
	if err := textList(fmt.Sprintf("record %q reasons", short(r.Action)), rr.Reasons, MaxReasons, 512); err != nil {
		return err
	}
	kinds := map[string]int{}
	for _, reason := range rr.Reasons {
		if len(kinds) >= MaxReasonKinds && kinds[kindOf(reason)] == 0 {
			return fmt.Errorf("record %q: more than %d distinct reason kinds", short(r.Action), MaxReasonKinds)
		}
		if kinds[kindOf(reason)]++; kinds[kindOf(reason)] > MaxReasonsPerKind {
			return fmt.Errorf("record %q: more than %d reasons of kind %q", short(r.Action), MaxReasonsPerKind, short(kindOf(reason)))
		}
	}
	return nil
}

// sortedCopy returns a sorted copy.
func sortedCopy(l []string) []string {
	c := make([]string, len(l)) // never nil: a nil list would encode as null
	copy(c, l)
	sort.Strings(c)
	return c
}

// statusOK: a status is a short lowercase word ("built", "failed", "not-run").
func statusOK(s string) bool {
	if s == "" || len(s) > 32 {
		return false
	}
	for i := 0; i < len(s); i++ {
		if c := s[i]; !(c >= 'a' && c <= 'z' || c == '-') {
			return false
		}
	}
	return true
}

// hasDotDot reports a ".." segment (or a relative-looking "//" is left to
// callers): a recorded root must be a clean absolute path.
func hasDotDot(p string) bool {
	for _, seg := range strings.Split(p, "/") {
		if seg == ".." {
			return true
		}
	}
	return false
}
