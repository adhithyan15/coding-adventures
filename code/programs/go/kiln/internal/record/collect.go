package record

// `kiln record collect` (KILN04 section 4.8, stage 4 PR 2b-2b-ii)
// ================================================================
//
//	kiln record collect --root <abs> --dir <abs> --results <abs> --out <abs>
//
// The last command of a recording shard. After the traced build (the build tool
// ran every step through `kiln record step`, one action at a time) it assembles
// what the shard learned into ONE file, `records.jsonl`, for the merge job:
//
//	pre/ (sealed)    binding, host facts, pre-build inventory, the plan
//	traces/          per action: sidecars, traces, record.json (classified at the
//	                 action's end by `step`, P13)
//	--results        the build tool's status and step count per package
//	the checkout     after the build: still the commit? anything new on disk?
//	        |
//	        v
//	--out            line 1: the header (void tokens, the two "unexplained" counts)
//	                 then one record per action, in order
//
// `collect` classifies nothing and parses no trace. Its job is to be the last
// gate before the merge, so what it does is to REFUSE TO BELIEVE:
//
//	1  the record directory is held exclusively (no step, no orphaned supervisor)
//	2  `pre/` is still what `prepare` sealed, and the programs it uses are the
//	   pinned ones (Git by its recorded path and the recorded PATH, never looked up)
//	3  every action's directory is accounted for (collect_actions.go): what
//	   cannot be proven becomes an UNKNOWN WRITE SET
//	4  the serial order holds across the shard (seq numbers distinct, below the
//	   final counter, no overlap marker)
//	5  after the build the checkout is still the commit the shard started from
//	   (VerifyTree with the pinned commit), the object store is connected
//	   (`git fsck --connectivity-only`), and nothing new is on the disk that no
//	   record's writes explain (Inventory against the pre-build inventory)
//	6  the shard decodes strictly with `recfmt`, as the merge will read it
//
// Two levels of "no", as in the merge (section 7.1):
//
//	REFUSE    exit 1, nothing written: `collect` cannot say what the build did (the
//	          directory is locked, `pre/` was changed, the results are unreadable)
//	VOID      a shard is written whose header or records make the merge discard it:
//	          an unknown write set (run-void, G1), a changed tree or a failed
//	          check (`tracked-changed`, shard-void M4, and the counts that make the
//	          run void when a write nobody traced is possible)
//
// A plain failure of Git, of `fsck` or of the post-build inventory is NOT a crash:
// the shard is still written, with the void and with a non-zero "unexplained"
// count, because the same fact (we cannot enumerate what changed) is what the
// count means. See collect_post.go.
//
// The output. `--out` is `records.jsonl`. The three other files of a shard
// artifact (binding, host facts, pre-build inventory) are already in `pre/`,
// byte for byte as `prepare` wrote them, and the merge decodes the four together
// (recfmt.DecodeShard). `collect` decodes its own output with them before it
// writes anything, so a shard the merge would refuse is never produced.

import (
	"bytes"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sort"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// CollectOptions are the inputs of Collect. Every path is absolute and clean.
type CollectOptions struct {
	// Root is the checkout, Dir the record directory, Results the build tool's
	// `-emit-results` file and Out where `records.jsonl` goes (it must not exist).
	Root, Dir, Results, Out string
	// KilnExe is the running binary; its SHA-256 must be the binding's.
	KilnExe string

	// The seams, each defaulting to the real thing; a test replaces one to make
	// the step it stands for fail.
	Lock       func(dir string, exclusive bool) (func(), error)
	VerifyTree func(g *source.Git, root, expectedCommit string) (Result, error)
	Inventory  func(g *source.Git, root string) (recfmt.Inventory, error)
	Fsck       func(g *source.Git) error
}

// CollectResult says what a successful Collect wrote.
type CollectResult struct {
	Out    string
	Header recfmt.ShardHeader
	// Records is the number of record lines written (one per action of the shard).
	Records int
	// Unknown lists "<action>: <reason>" for every action whose write set is
	// unknown (the merge voids the run for each), sorted.
	Unknown []string
	// PostProblems says why each post-build check that failed did (the error, the
	// first paths it named, the unexplained ones), printable and bounded. It is
	// for the operator: the shard cannot carry it, and a CI run that voids a shard
	// must say why.
	PostProblems []string
}

// CollectedMarker is the file `collect` creates in traces/ before it reads
// anything there (O_EXCL). `step` refuses to run once it exists.
const CollectedMarker = ".collected"

func (o *CollectOptions) defaults() {
	if o.Lock == nil {
		o.Lock = LockDir
	}
	if o.VerifyTree == nil {
		o.VerifyTree = VerifyTreeWith
	}
	if o.Inventory == nil {
		o.Inventory = InventoryWith
	}
	if o.Fsck == nil {
		o.Fsck = Fsck
	}
}

// Fsck is `git fsck --connectivity-only --no-dangling --no-progress`: every
// object reachable from a ref exists. The commit pin protects object ids, not
// the integrity of the object store (section 4.8); the sandbox has no `.git` at
// all (the mask), and this is the second line.
func Fsck(g *source.Git) error {
	_, err := g.Output("fsck", "--connectivity-only", "--no-dangling", "--no-progress")
	return err
}

// collector carries what the per-action code shares.
type collector struct {
	dir     string
	pre     *Pre
	results map[string]ResultsPackage
}

// Collect runs `kiln record collect`.
func Collect(o CollectOptions) (*CollectResult, error) {
	o.defaults()
	for _, p := range []struct{ what, v string }{{"--root", o.Root}, {"--dir", o.Dir}, {"--results", o.Results}, {"--out", o.Out}, {"the kiln binary", o.KilnExe}} {
		if !filepath.IsAbs(p.v) || filepath.Clean(p.v) != p.v {
			return nil, fmt.Errorf("%s %q is not an absolute, clean path", p.what, printable(p.v))
		}
	}
	if _, err := Canonical("--dir", o.Dir); err != nil {
		return nil, err
	}

	// 1. The exclusive lock, FIRST, before anything of the directory is read: a
	// step, or a supervisor orphaned by a dead build tool, holds the shared one
	// and may still be writing.
	unlock, err := o.Lock(o.Dir, true)
	if err != nil {
		return nil, err
	}
	defer unlock()

	// 2. pre/ is what prepare sealed, and the programs are the pinned ones.
	pre, err := VerifyPre(o.Dir)
	if err != nil {
		return nil, err
	}
	if pre.Config.RepoRoot != o.Root {
		return nil, fmt.Errorf("--root %q is not the checkout the shard was prepared for (%q)", printable(o.Root), printable(pre.Config.RepoRoot))
	}
	kiln, err := ResolveBinary("the kiln binary", o.KilnExe)
	if err != nil {
		return nil, err
	}
	if kiln.SHA256 != pre.Binding.KilnSHA256 || kiln.Path != pre.Config.KilnExe {
		return nil, fmt.Errorf("the running kiln is not the binary prepare hashed into the binding")
	}
	git, err := ResolveBinary("git", pre.Tools.Git)
	if err != nil {
		return nil, err
	}
	if git.SHA256 != pre.Binding.GitSHA256 {
		return nil, fmt.Errorf("git %q is not the binary prepare hashed into the binding", printable(git.Path))
	}
	names, err := envNames(pre.Tools.BuildToolEnv)
	if err != nil {
		return nil, fmt.Errorf("tools.json: %w", err)
	}
	home, ok := names["HOME"]
	if !ok {
		return nil, fmt.Errorf("tools.json: build_tool_env has no HOME")
	}
	// The environment prepare gave its own Git calls, less one variable: HOME and
	// the recorded PATH, and the fixed hardening source.NewGitAt adds. prepare also
	// passed XDG_CONFIG_HOME when the workflow set it; that is not recorded in
	// tools.json, so collect does not pass it (the pinned GIT_CONFIG_GLOBAL, the
	// null device, replaces the user-level configuration it would have named on any
	// git that knows the variable, and a git too old for it reads files the
	// recorder keeps unwritable, P9). Nothing of this process's environment reaches
	// git, and git is run by path.
	g := source.NewGitAt(o.Root, git.Path, []string{home, "PATH=" + pre.Tools.Path})
	if err := checkOutPlacement(o); err != nil {
		return nil, err
	}

	// 3. The build tool's results, then every action.
	res, err := ReadResults(o.Results)
	if err != nil {
		return nil, err
	}
	c := &collector{dir: o.Dir, pre: pre, results: res.ByPackage()}
	for _, p := range res.Packages {
		if _, ok := pre.Action(p.Package); !ok {
			return nil, fmt.Errorf("--results lists %q, which is not an action of this shard's plan", printable(p.Package))
		}
	}
	// Every refusal that is about the ARGUMENTS is behind us. From here `collect`
	// reads the directory, so nothing may write into it afterwards: the tombstone
	// makes `step` refuse (125, no sequence number) from now on, which closes the
	// window in which a late supervisor, one that took the shared lock after this
	// process released the exclusive one, could add a step after the verdict.
	// O_EXCL: a directory is collected once.
	if err := writeExcl(filepath.Join(o.Dir, "traces", CollectedMarker), []byte{}); err != nil {
		return nil, fmt.Errorf("traces/%s: %w (a record directory is collected once)", CollectedMarker, err)
	}
	var (
		outs    []actionOut
		recs    []recfmt.ShardRecord
		overlap bool
	)
	for _, name := range pre.Actions() {
		planned, _ := pre.Action(name)
		ao := c.assembleAction(name, planned)
		overlap = overlap || ao.overlap
		outs = append(outs, ao)
	}

	// 4. The serial order across the shard. Whatever fails here makes EVERY record
	// an unknown write set: the order is what lets each action be classified on
	// the disk it left, and a shard whose order is in doubt has no such action.
	if why := c.globalSeqCheck(outs, overlap); why != "" {
		for i := range outs {
			markUnknown(&outs[i].rec, why)
		}
	}
	for _, ao := range outs {
		recs = append(recs, ao.rec)
	}

	// 5. After the build.
	post := c.postBuild(o, g, recs)

	// 6. The header and the file.
	header := recfmt.ShardHeader{
		Schema: recfmt.SchemaShard, BindingID: pre.BindingID,
		ShardIndex: pre.Binding.ShardIndex, ShardCount: pre.Binding.ShardCount,
		Void: post.void, PostInventoryUnexplained: post.inventoryUnexplained, TrackedChangedUnexplained: post.trackedUnexplained,
	}
	data, header, recs, err := c.encodeShard(header, recs)
	if err != nil {
		return nil, err
	}
	if err := writeAtomic(o.Out, data); err != nil {
		return nil, fmt.Errorf("--out: %w", err)
	}
	r := &CollectResult{Out: o.Out, Header: header, Records: len(recs), PostProblems: post.problems}
	for _, rec := range recs {
		switch {
		case rec.Unknown != "":
			r.Unknown = append(r.Unknown, printable(rec.Action)+": "+rec.Unknown)
		case rec.StepsRun > 0 && rec.Complete != nil && !*rec.Complete:
			// Believed record, but the classifier could not vouch for its write
			// set: the merge reads it as an unknown write set all the same, so the
			// operator is told here too.
			r.Unknown = append(r.Unknown, printable(rec.Action)+": writes_complete is false (the classifier could not vouch for the write set)")
		}
	}
	sort.Strings(r.Unknown)
	return r, nil
}

// checkOutPlacement refuses an --out that would put the shard where it could be
// confused with, or damage, what the recorder keeps: inside the sealed `pre/`,
// inside `traces/`, or inside the checkout (the output is not part of the tree).
// The parent must be a canonical directory, so that creating the file cannot be
// redirected, and the file must not exist (it is never replaced).
func checkOutPlacement(o CollectOptions) error {
	if within(o.Out, filepath.Join(o.Dir, "pre")) || within(o.Out, filepath.Join(o.Dir, "traces")) {
		return fmt.Errorf("--out %q is inside the sealed pre/ or the traces", printable(o.Out))
	}
	if within(o.Out, o.Root) {
		return fmt.Errorf("--out %q is inside the checkout", printable(o.Out))
	}
	// The results are the build tool's output, read as a fact about the build. A
	// file inside the checkout is a file a BUILD can write.
	if within(o.Results, o.Root) {
		return fmt.Errorf("--results %q is inside the checkout, where a BUILD can write it", printable(o.Results))
	}
	// And its parent must be a canonical directory (no symlink anywhere in it), or
	// a link a BUILD planted could make a path that looks outside the checkout
	// lead into it: the spelling above is only as good as what it resolves to.
	if _, err := Canonical("the parent of --results", filepath.Dir(o.Results)); err != nil {
		return err
	}
	if _, err := Canonical("the parent of --out", filepath.Dir(o.Out)); err != nil {
		return err
	}
	// Never replaced: refused HERE, before the tombstone, so a name in the way
	// does not use up the directory's one collection.
	if _, err := os.Lstat(o.Out); err == nil {
		return fmt.Errorf("--out %q exists; it is never replaced", printable(o.Out))
	}
	return nil
}

// globalSeqCheck is the part of the serial-order rule that needs every action
// (P13): the numbers are globally distinct, each below the final value of
// traces/.seq, and as many as the counter says; no `.overlap` marker anywhere;
// and nothing else is in traces/ than the two small files and the actions'
// directories. It returns "" when the order holds.
func (c *collector) globalSeqCheck(outs []actionOut, overlap bool) string {
	traces := filepath.Join(c.dir, "traces")
	if overlap {
		return "serial-violated: an overlap marker"
	}
	data, err := readBoundedFile(filepath.Join(traces, ".seq"), 64)
	if err != nil {
		return "serial-violated: traces/.seq: " + err.Error()
	}
	s := string(data)
	if len(s) < 2 || s[len(s)-1] != '\n' {
		return "serial-violated: traces/.seq is not a counter"
	}
	final, err := canonicalInt("traces/.seq", s[:len(s)-1])
	if err != nil {
		return "serial-violated: traces/.seq is not a counter"
	}
	seen := map[int]bool{}
	total := 0
	for _, ao := range outs {
		for _, u := range ao.seqs {
			switch {
			case u.seq >= final:
				return fmt.Sprintf("serial-violated: seq %d is not below the final counter %d", u.seq, final)
			case seen[u.seq]:
				return fmt.Sprintf("serial-violated: seq %d was used twice", u.seq)
			}
			seen[u.seq] = true
			total++
		}
	}
	// Every number taken has a sidecar: a step that took one and left none was
	// killed between its trace and its sidecar.
	if total != final {
		return fmt.Sprintf("serial-violated: the counter reached %d and %d sidecars hold a number", final, total)
	}
	// Only what the supervisors and `prepare` make is in traces/.
	known := map[string]bool{".seq": true, ".serial": true, CollectedMarker: true}
	for _, ao := range outs {
		known[filepath.Base(StepDir(c.dir, ao.name))] = true
	}
	entries, err := readDirNames(traces)
	if err != nil {
		return "serial-violated: traces/: " + err.Error()
	}
	for _, n := range entries {
		if !known[n] {
			return "serial-violated: traces/ holds an unexpected entry " + reasonText(n)
		}
	}
	return ""
}

// encodeShard renders the header and the records as one records.jsonl and proves
// the merge can read it. A record that cannot be encoded (a list over a cap, a
// name the format refuses) becomes an unknown write set rather than a crash. If
// the whole shard does not decode under the format's size caps, the shard is
// replaced by one that says so (`records-too-large`, run-void in the merge) with
// every record unknown: a shard is never dropped silently and never half
// believed. The returned header and records are what was written.
func (c *collector) encodeShard(h recfmt.ShardHeader, recs []recfmt.ShardRecord) ([]byte, recfmt.ShardHeader, []recfmt.ShardRecord, error) {
	h.Void = uniqSorted(h.Void)
	for i := range recs {
		if _, err := recfmt.EncodeRecord(recs[i]); err != nil {
			markUnknown(&recs[i], "record-unencodable: "+err.Error())
			if _, err2 := recfmt.EncodeRecord(recs[i]); err2 != nil {
				return nil, h, nil, fmt.Errorf("record %q cannot be encoded even as an unknown write set: %v", printable(recs[i].Action), err2)
			}
		}
	}
	data, err := recfmt.EncodeShardRecords(h, recs)
	if err != nil {
		return nil, h, nil, err
	}
	if err := c.selfCheck(data); err == nil {
		return data, h, recs, nil
	}
	// Too big (the only way a shard of valid records fails to decode): say so.
	h.Void = uniqSorted(append(h.Void, "records-too-large"))
	for i := range recs {
		markUnknown(&recs[i], "records-too-large")
	}
	if data, err = recfmt.EncodeShardRecords(h, recs); err != nil {
		return nil, h, nil, err
	}
	if err := c.selfCheck(data); err != nil {
		return nil, h, nil, fmt.Errorf("the shard does not decode even with every record unknown: %w", err)
	}
	return data, h, recs, nil
}

// selfCheck decodes records.jsonl with the three sealed files exactly as the
// merge will (recfmt.DecodeShard, default limits) and requires the binding the
// records name to be this one.
func (c *collector) selfCheck(records []byte) error {
	files := recfmt.ShardFiles{Records: bytes.NewReader(records)}
	var err error
	for _, f := range []struct {
		rel string
		dst *[]byte
	}{{"binding.json", &files.Binding}, {"host-facts.json", &files.HostFacts}, {"inventory.json", &files.Inventory}} {
		if *f.dst, err = c.pre.ReadSealed(f.rel); err != nil {
			return err
		}
	}
	sh, err := recfmt.DecodeShard(c.pre.Binding.ShardIndex, files, recfmt.DefaultLimits)
	if err != nil {
		return err
	}
	if sh.BindingID != c.pre.BindingID || sh.Header.BindingID != c.pre.BindingID {
		return errors.New("the shard names another binding")
	}
	if len(sh.Records) != len(c.pre.Actions()) {
		return fmt.Errorf("the shard holds %d records for %d actions", len(sh.Records), len(c.pre.Actions()))
	}
	return nil
}

// uniqSorted returns the strings sorted and without repeats (the header's void
// list must be strictly sorted).
func uniqSorted(in []string) []string {
	out := append([]string{}, in...)
	sort.Strings(out)
	j := 0
	for i, s := range out {
		if i == 0 || s != out[j-1] {
			out[j] = s
			j++
		}
	}
	return out[:j]
}
