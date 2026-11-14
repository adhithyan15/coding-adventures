package record

// TreeError.Paths (KILN04 section 4.8; `collect` needs the whole set of changed
// paths to count the ones no record explains, M-2).

import (
	"fmt"
	"testing"
)

func TestTreeErrorPathsAreComplete(t *testing.T) {
	var p problems
	for i := 40; i > 0; i-- { // out of order, and more than MaxReported
		p.add(fmt.Sprintf("dir/f%02d", i), "content differs from the committed blob")
	}
	// One path with two problems is one path.
	p.add("dir/f07", "mode differs")
	// A path with control characters is kept RAW in Paths and printable in Problems.
	p.add("dir/odd\nname", "missing on disk")
	err := p.err("working tree is not HEAD")
	te, ok := err.(*TreeError)
	if !ok {
		t.Fatalf("%T", err)
	}
	if te.Total != 42 || len(te.Problems) != MaxReported {
		t.Fatalf("total %d, %d problems named", te.Total, len(te.Problems))
	}
	if len(te.Paths) != 41 {
		t.Fatalf("%d paths, want all 41 distinct ones", len(te.Paths))
	}
	for i := 1; i < len(te.Paths); i++ {
		if te.Paths[i-1] >= te.Paths[i] {
			t.Fatalf("not sorted and unique at %q", te.Paths[i])
		}
	}
	found := false
	for _, q := range te.Paths {
		found = found || q == "dir/odd\nname"
	}
	if !found {
		t.Fatal("the raw path is not in Paths")
	}
	for _, line := range te.Problems {
		for _, c := range line {
			if c == '\n' && line != "" {
				t.Fatalf("a problem line holds a newline: %q", line)
			}
		}
	}
}

func TestTreeErrorPathsOfAnUnresolvedMerge(t *testing.T) {
	m, err := parseIndex([]byte("100644 " + "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678" + " 2\tcode/x\x00"))
	if m != nil || err == nil {
		t.Fatalf("%v %v", m, err)
	}
	te, ok := err.(*TreeError)
	if !ok || len(te.Paths) != 1 || te.Paths[0] != "code/x" {
		t.Fatalf("%#v", err)
	}
}
