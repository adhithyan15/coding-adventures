package record

// VerifyTree (KILN04 section 4.8, G5)
// ===================================
//
// The question: "is every file in this checkout exactly what the commit says?"
//
// It is asked three times in a recording run (before any BUILD, after the
// build, and in the merge job before any digest), and the answer must not be
// fooled by anything a repository can configure. So the check is done in three
// layers, each closing a gap the one before leaves:
//
//	layer 0  the COMMIT is pinned
//	         HEAD^{commit} is resolved ONCE to a full id (replace refs are off),
//	         everything below is checked against that id, and the id is
//	         returned so a later call can demand the same one. The root must be
//	         the top of its repository.
//
//	layer 1  the INDEX equals that commit
//	         `git ls-tree -r -z <id>` reads the object store; `git ls-files
//	         --stage -z` reads the index. They must list the same paths with the
//	         same modes and the same object ids, all at stage 0. Without this a
//	         staged edit (or an unresolved merge) would be "the tree" while HEAD
//	         says otherwise.
//
//	layer 2  nothing hides files from Git, so nothing is hidden from us
//	         `git ls-files -v` marks skip-worktree ("S") and assume-unchanged
//	         (lowercase) entries: Git would not notice if those files changed on
//	         disk, so neither would any check that asked Git. A sparse checkout
//	         (`core.sparseCheckout`) is the same trick at scale: files simply
//	         are not there. Both are refused outright rather than reasoned about.
//
//	layer 3  the DISK equals the index, byte for byte
//	         For every entry we `lstat` the path (never follow), and then:
//	             120000  a symlink, whose target text hashes to the blob id
//	             100644  a regular file, opened O_NOFOLLOW|O_NONBLOCK|O_NOCTTY,
//	             100755  hashed as a Git blob; and the exec bit must agree
//	             160000  a submodule: refused outright ("submodule-unsupported")
//	             other   "non-regular": a FIFO, a socket, a device, a directory
//	                     where a file belongs
//	         Hashing as a Git blob (`"blob <size>\0" + bytes`) means a smudge
//	         filter, `ident`, `autocrlf`, `eol=crlf` or an LFS pointer shows up
//	         for what it is: bytes that do not hash to the committed object.
//
// Why the opens are paranoid. A FIFO named like a tracked file would make a
// plain `open` block forever waiting for a writer; O_NONBLOCK makes it return
// at once, and the `fstat` on the open handle refuses it. O_NOFOLLOW refuses a
// symlink that appeared between the `lstat` and the `open`. O_NOCTTY stops a
// terminal device from becoming our controlling terminal. And every ANCESTOR
// directory is `lstat`ed too: a tracked directory replaced by a symlink would
// otherwise let `a/f` read some other directory's `f` while `lstat("a/f")`
// looked perfectly normal.
//
// What this does NOT promise: it is not atomic. A process that races the
// check can change a file after we hashed it. The recorder runs it before any
// BUILD starts and again after every BUILD has exited, and the merge job runs
// it on a tree no BUILD has touched, which is what makes a one-time check
// meaningful.
//
// Limits (G5 step 4): a file over 1 GiB, a check that runs over 20 minutes and
// a file whose length changes under us are errors; 8 goroutines hash in
// parallel; at most 16 paths are named in the error.

import (
	"bytes"
	"context"
	"crypto/sha1"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"hash"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
	"sync"
	"time"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// Limits of one VerifyTree call. They are variables so tests can lower them
// instead of writing a gigabyte; production code never changes them.
var (
	maxFileBytes = int64(1) << 30
	budget       = 20 * time.Minute
	workers      = 8
)

// Git file modes, as `ls-tree` and `ls-files --stage` print them.
const (
	modeFile    = "100644"
	modeExec    = "100755"
	modeSymlink = "120000"
	modeGitlink = "160000"
)

// entry is one tracked path: its mode and object id (hex, 40 or 64 digits).
type entry struct{ mode, id string }

// Result is what a successful VerifyTree vouches for.
type Result struct {
	// Commit is the full id of the commit the tree was proven to equal
	// (`HEAD^{commit}`, resolved once). The post-build check passes it back
	// as expectedCommit, so a BUILD that ran `git commit -a` (moving HEAD to
	// a commit that DOES match the edited disk) is caught.
	Commit string
}

// VerifyTree proves the checkout at root equals one commit (see the file
// comment) and returns that commit's id.
//
// expectedCommit pins the commit: when it is not empty the tree must equal
// THAT commit and HEAD must still name it. The first call (before any BUILD)
// passes "" and keeps the id it gets back; the backstop after the build and
// the merge job pass it.
//
// root must be the top level of its repository. A subdirectory of a repository
// (or a directory with no .git of its own inside a parent repository) is
// refused: `ls-files` would then list only the subtree and everything else
// would go unchecked.
//
// A submodule (mode 160000) is an error for now ("submodule-unsupported"):
// nothing inside one is covered by this check, so a recording over a tree that
// has one would key unverified bytes. The repository has none today.
//
// A nil error means every tracked path is on disk with exactly the committed
// bytes and mode. Any other outcome is an error: a *TreeError for a tree that
// differs, or a plain error when git itself could not be run.
func VerifyTree(root, expectedCommit string) (Result, error) {
	return VerifyTreeWith(source.NewGit(root), root, expectedCommit)
}

// VerifyTreeWith is VerifyTree with the Git runner chosen by the caller. The
// recorder passes one pinned to an absolute git path (source.NewGitAt), so no
// lookup on the process PATH can run a git a BUILD planted.
func VerifyTreeWith(g *source.Git, root, expectedCommit string) (Result, error) {
	if err := requireTopLevel(g); err != nil {
		return Result{}, err
	}
	commit, err := resolveHead(g)
	if err != nil {
		return Result{}, err
	}
	if expectedCommit != "" && commit != expectedCommit {
		return Result{}, fmt.Errorf("HEAD is %s but %s was expected: the commit moved", short(commit), short(expectedCommit))
	}

	headTree, err := lsTree(g, commit)
	if err != nil {
		return Result{}, err
	}
	index, err := lsIndex(g)
	if err != nil {
		return Result{}, err
	}

	var bad problems
	compareTrees(headTree, index, &bad)
	if err := checkHidden(g, len(index), &bad); err != nil {
		return Result{}, err
	}
	// If the index and HEAD disagree there is no single "expected" content to
	// hash against; report that and stop, rather than a second, noisier list.
	if len(bad.list) > 0 {
		return Result{}, bad.err("working tree is not HEAD")
	}

	if err := checkDisk(root, headTree, &bad); err != nil {
		return Result{}, err
	}
	if e := bad.err("working tree is not HEAD"); e != nil {
		return Result{}, e
	}
	return Result{Commit: commit}, nil
}

// requireTopLevel refuses a root that is not the top of its work tree:
// `git rev-parse --show-prefix` is empty exactly there.
func requireTopLevel(g *source.Git) error {
	out, err := g.Output("rev-parse", "--show-prefix")
	if err != nil {
		return err
	}
	if p := strings.TrimSpace(string(out)); p != "" {
		return fmt.Errorf("the root is not the top level of its repository (it is %q inside it)", printable(p))
	}
	return nil
}

// resolveHead names HEAD's commit once, as a full hex id. Replace refs are off
// (see source.gitHardening), so the id means what its hash says.
func resolveHead(g *source.Git) (string, error) {
	out, err := g.Output("rev-parse", "--verify", "--quiet", "HEAD^{commit}")
	if err != nil {
		return "", err
	}
	id := strings.TrimSpace(string(out))
	if _, err := newHasher(id); err != nil || !isHex(id) {
		return "", fmt.Errorf("git rev-parse HEAD: %q is not a full object id", printable(id))
	}
	return id, nil
}

func isHex(s string) bool {
	for i := 0; i < len(s); i++ {
		c := s[i]
		if !(c >= '0' && c <= '9' || c >= 'a' && c <= 'f') {
			return false
		}
	}
	return true
}

// checkTreePath refuses a path a tree should never hold and that a walk or an
// `open` could be steered by: an empty, "." or ".." component, a ".git"
// component in any case (a checkout must never write into the repository's own
// directory), a leading slash, a backslash (a separator on Windows) or a NUL.
// Git itself refuses most of these when it writes a tree, but a tree object can
// be forged with plumbing, and this is the last gate before `filepath.Join`.
func checkTreePath(p string) string {
	switch {
	case p == "":
		return "empty path"
	case p[0] == '/':
		return "absolute path"
	case strings.ContainsAny(p, "\\\x00"):
		return "backslash or NUL in the path"
	}
	for _, c := range strings.Split(p, "/") {
		switch {
		case c == "" || c == "." || c == "..":
			return fmt.Sprintf("path has the component %q", c)
		case strings.EqualFold(c, ".git"):
			return "path has a .git component"
		}
	}
	return ""
}

// lsTree reads the tree of commit: path -> (mode, id). `-z` gives raw
// NUL-terminated records (no quoting of odd file names); `-r` recurses into
// subtrees; `--full-tree` makes the paths relative to the repository root
// whatever the working directory. A gitlink is an error.
func lsTree(g *source.Git, commit string) (map[string]entry, error) {
	out, err := g.Output("ls-tree", "-r", "-z", "--full-tree", commit)
	if err != nil {
		return nil, err
	}
	return parseTree(out)
}

// parseTree parses `ls-tree -r -z` output.
func parseTree(out []byte) (map[string]entry, error) {
	m := map[string]entry{}
	var bad problems
	for _, rec := range splitNUL(out) {
		meta, path, ok := strings.Cut(rec, "\t")
		f := strings.Fields(meta)
		if !ok || len(f) != 3 {
			return nil, fmt.Errorf("git ls-tree: unparsable record %q", printable(rec))
		}
		if why := checkTreePath(path); why != "" {
			return nil, fmt.Errorf("git ls-tree: %q: %s", printable(path), why)
		}
		if _, dup := m[path]; dup {
			return nil, fmt.Errorf("git ls-tree: %q listed twice", printable(path))
		}
		if f[0] == modeGitlink {
			bad.add(path, "a submodule (gitlink)")
		}
		m[path] = entry{mode: f[0], id: f[2]}
	}
	return m, bad.err("submodule-unsupported")
}

// lsIndex reads the index: path -> (mode, id). Any entry whose stage is not 0
// (an unresolved merge) is an error here: there is no single content for it.
func lsIndex(g *source.Git) (map[string]entry, error) {
	out, err := g.Output("ls-files", "--stage", "-z")
	if err != nil {
		return nil, err
	}
	return parseIndex(out)
}

// parseIndex parses `ls-files --stage -z` output.
func parseIndex(out []byte) (map[string]entry, error) {
	m := map[string]entry{}
	for _, rec := range splitNUL(out) {
		meta, path, ok := strings.Cut(rec, "\t")
		f := strings.Fields(meta)
		if !ok || len(f) != 3 || path == "" {
			return nil, fmt.Errorf("git ls-files --stage: unparsable record %q", printable(rec))
		}
		if f[2] != "0" {
			return nil, &TreeError{What: "working tree is not HEAD", Total: 1,
				Problems: []string{fmt.Sprintf("%s: index entry at stage %s (unresolved merge)", printable(path), f[2])}}
		}
		if _, dup := m[path]; dup {
			return nil, fmt.Errorf("git ls-files: %q listed twice", printable(path))
		}
		m[path] = entry{mode: f[0], id: f[1]}
	}
	return m, nil
}

func splitNUL(b []byte) []string {
	var out []string
	for _, f := range bytes.Split(b, []byte{0}) {
		if len(f) > 0 {
			out = append(out, string(f))
		}
	}
	return out
}

// compareTrees records every path where HEAD and the index differ.
func compareTrees(head, index map[string]entry, bad *problems) {
	for path, h := range head {
		i, ok := index[path]
		switch {
		case !ok:
			bad.add(path, "in HEAD but not in the index")
		case i != h:
			bad.add(path, fmt.Sprintf("index has %s %s, HEAD has %s %s", i.mode, short(i.id), h.mode, short(h.id)))
		}
	}
	for path := range index {
		if _, ok := head[path]; !ok {
			bad.add(path, "in the index but not in HEAD")
		}
	}
}

func short(id string) string {
	if len(id) > 12 {
		return id[:12]
	}
	return id
}

// checkHidden refuses what would let a file change without Git noticing.
func checkHidden(g *source.Git, entries int, bad *problems) error {
	out, err := g.Output("ls-files", "-v", "-z")
	if err != nil {
		return err
	}
	if err := parseHidden(out, entries, bad); err != nil {
		return err
	}
	return checkSparseConfig(g, bad)
}

// parseHidden reads `ls-files -v -z` output and records every entry that is
// not plain "H".
func parseHidden(out []byte, entries int, bad *problems) error {
	seen := 0
	for _, rec := range splitNUL(out) {
		// "<tag> <path>": one tag character, a space, the path.
		if len(rec) < 3 || rec[1] != ' ' {
			return fmt.Errorf("git ls-files -v: unparsable record %q", printable(rec))
		}
		seen++
		switch tag := rec[0]; {
		case tag == 'H':
		case tag == 'S':
			bad.add(rec[2:], "skip-worktree bit set")
		case tag >= 'a' && tag <= 'z':
			bad.add(rec[2:], "assume-unchanged bit set")
		default:
			bad.add(rec[2:], fmt.Sprintf("unexpected status tag %q", tag))
		}
	}
	if seen != entries {
		return fmt.Errorf("git ls-files -v listed %d paths, --stage listed %d", seen, entries)
	}
	return nil
}

// checkSparseConfig refuses `core.sparseCheckout`.
func checkSparseConfig(g *source.Git, bad *problems) error {
	// `--bool` turns yes/on/1/true into "true", so every spelling of "on" is
	// caught. Exit status 1 means the key is unset, which is the good case.
	cfg, err := g.Output("config", "--bool", "--get", "core.sparseCheckout")
	if err != nil {
		var ee *exec.ExitError
		if errors.As(err, &ee) && ee.ExitCode() == 1 {
			return nil
		}
		return err
	}
	if strings.TrimSpace(string(cfg)) == "true" {
		bad.add(".", "core.sparseCheckout is true: the checkout is sparse")
	}
	return nil
}

// checkDisk hashes every entry against the disk, 8 at a time, under the
// time budget.
func checkDisk(root string, tree map[string]entry, bad *problems) error {
	paths := make([]string, 0, len(tree))
	for p := range tree {
		paths = append(paths, p)
	}
	sort.Strings(paths)

	ctx, cancel := context.WithTimeout(context.Background(), budget)
	defer cancel()

	v := &verifier{root: root, ctx: ctx, dirs: map[string]error{}}
	var (
		mu sync.Mutex
		wg sync.WaitGroup
	)
	jobs := make(chan string)
	for w := 0; w < workers; w++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for p := range jobs {
				why := v.check(p, tree[p])
				mu.Lock()
				if why != "" {
					bad.add(p, why)
				}
				mu.Unlock()
			}
		}()
	}
feed:
	for _, p := range paths {
		select {
		case jobs <- p:
		case <-ctx.Done():
			break feed
		}
	}
	close(jobs)
	wg.Wait()
	if ctx.Err() != nil {
		return fmt.Errorf("working tree check exceeded its %v budget", budget)
	}
	return nil
}

// verifier holds what the workers share: the root, the deadline, and a cache
// of ancestor directories already proven to be real directories.
type verifier struct {
	root string
	ctx  context.Context
	mu   sync.Mutex
	dirs map[string]error // directory (slash path) -> nil if a real directory
}

// check returns why the path is wrong ("" when it is right). A gitlink never
// reaches here (lsTree refuses the whole tree), so mode 160000 is just one
// more unsupported mode.
func (v *verifier) check(path string, want entry) string {
	if err := v.ctx.Err(); err != nil {
		return "" // the caller reports the budget once
	}
	if why := v.ancestors(path); why != "" {
		return why
	}
	full := filepath.Join(v.root, filepath.FromSlash(path))
	fi, err := os.Lstat(full)
	if err != nil {
		return "missing on disk"
	}
	switch want.mode {
	case modeSymlink:
		return v.checkSymlink(full, fi, want)
	case modeFile, modeExec:
		return v.checkFile(full, fi, want)
	default:
		return fmt.Sprintf("unsupported mode %s", want.mode)
	}
}

// ancestors proves every directory above path is a real directory, not a
// symlink to one. Results are cached: a directory with a thousand files is
// lstat-ed once.
func (v *verifier) ancestors(path string) string {
	for i := 0; i < len(path); i++ {
		if path[i] != '/' {
			continue
		}
		dir := path[:i]
		v.mu.Lock()
		err, seen := v.dirs[dir]
		v.mu.Unlock()
		if !seen {
			fi, lerr := os.Lstat(filepath.Join(v.root, filepath.FromSlash(dir)))
			switch {
			case lerr != nil:
				err = fmt.Errorf("directory %s is missing", printable(dir))
			case !fi.IsDir():
				err = fmt.Errorf("%s is not a real directory (a symlink or file stands in for it)", printable(dir))
			}
			v.mu.Lock()
			v.dirs[dir] = err
			v.mu.Unlock()
		}
		if err != nil {
			return err.Error()
		}
	}
	return ""
}

func (v *verifier) checkSymlink(full string, fi os.FileInfo, want entry) string {
	if fi.Mode()&os.ModeSymlink == 0 {
		return fmt.Sprintf("tracked as a symlink but is %s", kind(fi))
	}
	target, err := os.Readlink(full)
	target = linkTargetText(target)
	if err != nil {
		return "cannot read the symlink"
	}
	h, err := newHasher(want.id)
	if err != nil {
		return err.Error()
	}
	writeBlobHeader(h, int64(len(target)))
	_, _ = io.WriteString(h, target)
	if hex.EncodeToString(h.Sum(nil)) != want.id {
		return "symlink target differs from the committed one"
	}
	return ""
}

func (v *verifier) checkFile(full string, fi os.FileInfo, want entry) string {
	if !fi.Mode().IsRegular() {
		if fi.Mode()&os.ModeSymlink != 0 {
			return "tracked as a file but is a symlink"
		}
		return fmt.Sprintf("tracked as a file but is %s (non-regular)", kind(fi))
	}
	if fi.Size() > maxFileBytes {
		return fmt.Sprintf("file is %d bytes, over the %d byte limit", fi.Size(), maxFileBytes)
	}
	f, err := openNoFollow(full)
	if err != nil {
		return "cannot be opened without following a symlink"
	}
	defer f.Close()
	// Re-check on the HANDLE: the path may have been swapped after the lstat.
	hi, err := f.Stat()
	if err != nil || !hi.Mode().IsRegular() {
		return "tracked as a file but is non-regular"
	}
	if hi.Size() > maxFileBytes {
		return fmt.Sprintf("file is %d bytes, over the %d byte limit", hi.Size(), maxFileBytes)
	}
	if bad := execMismatch(hi.Mode(), want.mode); bad != "" {
		return bad
	}

	h, err := newHasher(want.id)
	if err != nil {
		return err.Error()
	}
	if why := hashBody(h, &ctxReader{ctx: v.ctx, r: f}, hi.Size()); why != "" {
		return why
	}
	if hex.EncodeToString(h.Sum(nil)) != want.id {
		return "content differs from the committed blob"
	}
	return ""
}

// hashBody feeds h a Git blob of exactly size bytes taken from r: the header
// first, then the bytes. The size comes from the OPEN HANDLE, so a file that
// shrinks or grows while it is being read is caught, not silently hashed under
// a header that no longer matches. It returns "" on success.
func hashBody(h hash.Hash, r io.Reader, size int64) string {
	writeBlobHeader(h, size)
	n, err := io.Copy(h, io.LimitReader(r, size))
	if err != nil {
		return "cannot be read"
	}
	if n != size {
		return "shrank while being read"
	}
	// One more byte would mean it GREW.
	var extra [1]byte
	if m, _ := r.Read(extra[:]); m != 0 {
		return "grew while being read"
	}
	return ""
}

func kind(fi os.FileInfo) string {
	m := fi.Mode()
	switch {
	case m.IsDir():
		return "a directory"
	case m&os.ModeNamedPipe != 0:
		return "a FIFO"
	case m&os.ModeSocket != 0:
		return "a socket"
	case m&os.ModeDevice != 0:
		return "a device"
	case m&os.ModeSymlink != 0:
		return "a symlink"
	}
	return "an irregular file"
}

// newHasher picks the hash by the id's length: 40 hex digits is a SHA-1
// repository, 64 is SHA-256 (`git init --object-format=sha256`).
func newHasher(id string) (hash.Hash, error) {
	switch len(id) {
	case 40:
		return sha1.New(), nil
	case 64:
		return sha256.New(), nil
	}
	return nil, fmt.Errorf("object id of %d digits is neither SHA-1 nor SHA-256", len(id))
}

// writeBlobHeader writes Git's object header: `blob <size>` and a NUL.
func writeBlobHeader(h hash.Hash, size int64) {
	_, _ = fmt.Fprintf(h, "blob %d\x00", size)
}

// ctxReader fails a read once the budget has run out, so one enormous or
// stalled file cannot outlive the deadline.
type ctxReader struct {
	ctx context.Context
	r   io.Reader
}

func (c *ctxReader) Read(p []byte) (int, error) {
	if err := c.ctx.Err(); err != nil {
		return 0, err
	}
	return c.r.Read(p)
}
