package traced

import (
	"errors"
	"io"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// PackageIndex is the package-boundary index (KILN03 section 2): which
// package, if any, encloses a repository-relative path.  Packages nest
// (code/packages/go/a/b may be a package inside code/packages/go/a), and the
// DEEPEST enclosing one wins, matching D(q) which excludes nested packages
// (KILN04 section 5.2).
//
//	packages:  code/p/a        code/p/a/b
//	Enclosing("code/p/a/b/x.go")  -> code/p/a/b
//	Enclosing("code/p/a/x.go")    -> code/p/a
//	Enclosing("code/p/a/newdir/")  -> code/p/a   (a path absent from the tree
//	                                              still belongs to its package)
//	Enclosing("code/scripts/x")    -> none
type PackageIndex struct {
	set map[string]bool
}

// NewPackageIndex builds the index from the packages' rel_paths.
func NewPackageIndex(relPaths []string) *PackageIndex {
	ix := &PackageIndex{set: make(map[string]bool, len(relPaths))}
	for _, p := range relPaths {
		ix.set[p] = true
	}
	return ix
}

// Enclosing returns the deepest package that contains rel (or is rel itself).
func (ix *PackageIndex) Enclosing(rel string) (string, bool) {
	for p := rel; p != ""; {
		if ix.set[p] {
			return p, true
		}
		i := strings.LastIndexByte(p, '/')
		if i < 0 {
			break
		}
		p = p[:i]
	}
	return "", false
}

// headLimit is how many leading bytes the exec rules look at: a "#!" line is
// 255 bytes at most, and an ELF program-header table sits at the start of the
// file.
const headLimit = 64 << 10

// SourceTree adapts a source.Source to the Tree interface.  It lists the
// tracked files once, derives the directories (a directory is tracked when it
// holds a tracked file), and asks a source.Describer, when there is one, which
// entries are symbolic links.
type SourceTree struct {
	src   source.Source
	nodes map[string]Node
	dirs  map[string]bool
}

// NewSourceTree reads the tracked file list from src.
func NewSourceTree(src source.Source) (*SourceTree, error) {
	files, err := src.Files("", nil)
	if err != nil {
		return nil, err
	}
	desc, _ := src.(source.Describer)
	t := &SourceTree{src: src, nodes: make(map[string]Node, len(files)), dirs: map[string]bool{}}
	for _, f := range files {
		n := Node{Kind: KindFile}
		if desc != nil {
			info, err := desc.Describe(f)
			if err != nil {
				return nil, err
			}
			if info.Link {
				n = Node{Kind: KindLink, Target: info.Target}
			}
		}
		t.nodes[f] = n
		for d := f; ; {
			i := strings.LastIndexByte(d, '/')
			if i < 0 {
				break
			}
			d = d[:i]
			if t.dirs[d] {
				break
			}
			t.dirs[d] = true
		}
	}
	return t, nil
}

// Stat implements Tree.
func (t *SourceTree) Stat(rel string) Node {
	if rel == "" {
		return Node{Kind: KindDir}
	}
	if n, ok := t.nodes[rel]; ok {
		return n
	}
	if t.dirs[rel] {
		return Node{Kind: KindDir}
	}
	return Node{}
}

// Head implements Tree.  Only regular files can be read: Source.Open refuses
// symbolic links.
func (t *SourceTree) Head(rel string, n int) ([]byte, error) {
	if t.nodes[rel].Kind != KindFile {
		return nil, errors.New("traced: not a tracked regular file")
	}
	r, err := t.src.Open(rel)
	if err != nil {
		return nil, err
	}
	defer r.Close()
	buf := make([]byte, n)
	got, err := io.ReadFull(r, buf)
	if err != nil && !errors.Is(err, io.ErrUnexpectedEOF) && !errors.Is(err, io.EOF) {
		return nil, err
	}
	return buf[:got], nil
}
