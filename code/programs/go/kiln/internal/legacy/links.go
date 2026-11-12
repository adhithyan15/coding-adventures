package legacy

// Symlink resolution (KILN03 3.3, "Symlinks").
//
// A link entry's digest covers only its target TEXT, so a link that points
// outside everything the key covers is an escape the digest cannot see.  Every
// link in a package's own files and in every read is therefore resolved FULLY,
// the way a kernel would: through chains of links and through links to
// directories, with at most maxLinkHops links followed, and the final path is
// checked against the package's closure and reads.
//
//	a -> b, b -> c/d, c/d is a regular file            final c/d
//	x/lnk -> ../../out        (leaves the closure)     escape
//	a -> b, b -> a            (loop: hops run out)     escape
//	d -> some/dir, then d/f   (directory link)         final some/dir/f
//	a -> ../../../../..       (leaves the repository)  escape

import (
	"fmt"
	"path"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/source"
)

// maxLinkHops is the most links one resolution may follow (Linux allows 40).
const maxLinkHops = 40

// linkResolution is where one link finally points, or that it cannot be said.
type linkResolution struct {
	final string // repo-relative, cleaned, "/"-separated
	ok    bool   // false: loop, hop limit, absolute or repo-escaping target, or a file used as a directory
}

// linksOf lists the symlinks among the files a read item stands for.
func (pl *planner) linksOf(it readItem) ([]string, error) {
	id := "p:" + it.path
	if it.glob {
		id = "g:" + it.path
	}
	return pl.itemLinks.get(id, func() ([]string, error) {
		files, err := pl.itemFiles(it)
		if err != nil {
			return nil, err
		}
		var out []string
		for _, f := range files {
			e, err := pl.entry(f)
			if err != nil {
				return nil, err
			}
			if e.Kind == key.KindLink {
				out = append(out, f)
			}
		}
		return out, nil
	})
}

// target returns the target text of the link at p.
func (pl *planner) target(p string) (string, error) {
	info, err := pl.linkInfo.get(p, func() (source.Info, error) { return pl.opt.Source.Describe(p) })
	return info.Target, err
}

// isLink reports whether p is a listed symlink.  Directories are not listed
// by a Source, so only a file or a link can match.
func (pl *planner) isLink(p string) (bool, error) {
	if !pl.fileSet[p] {
		return false, nil
	}
	e, err := pl.entry(p)
	return e.Kind == key.KindLink, err
}

// absoluteTarget reports a link target that names a place outside the
// repository by construction: a rooted path or a Windows drive path.
func absoluteTarget(t string) bool {
	return t == "" || strings.HasPrefix(t, "/") || (len(t) >= 2 && t[1] == ':' && (t[0]|0x20 >= 'a' && t[0]|0x20 <= 'z'))
}

// resolveLink resolves the link at l (a listed path).  It is the classic
// realpath loop: keep a stack of path components still to walk; a component
// that is a link is replaced by its target's components (spending one hop), ".."
// pops the resolved prefix, anything else extends it.  A loop never ends, so
// the hop limit is what refuses it.
func (pl *planner) resolveLink(l string) (linkResolution, error) {
	return pl.linkRes.get(l, func() (linkResolution, error) {
		pending := strings.Split(l, "/")
		var resolved []string
		hops := 0
		bad := linkResolution{}
		for len(pending) > 0 {
			c := pending[0]
			pending = pending[1:]
			switch c {
			case "", ".":
				continue
			case "..":
				if len(resolved) == 0 {
					return bad, nil // above the repository root
				}
				resolved = resolved[:len(resolved)-1]
				continue
			}
			cand := strings.Join(append(append([]string{}, resolved...), c), "/")
			isl, err := pl.isLink(cand)
			if err != nil {
				return bad, fmt.Errorf("%s: %w", short(cand), err)
			}
			if !isl {
				if pl.fileSet[cand] && len(pending) > 0 {
					return bad, nil // a regular file used as a directory
				}
				resolved = append(resolved, c)
				continue
			}
			hops++
			if hops > maxLinkHops {
				return bad, nil
			}
			t, err := pl.target(cand)
			if err != nil {
				return bad, fmt.Errorf("%s: %w", short(cand), err)
			}
			if absoluteTarget(t) {
				return bad, nil
			}
			pending = append(strings.Split(t, "/"), pending...)
		}
		return linkResolution{final: path.Clean(strings.Join(resolved, "/")), ok: true}, nil
	})
}
