package record

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
)

const goodPlan = `{
  "schema_version": 1, "diff_base": "x", "force": true, "some_future_field": [1,2],
  "packages": [
    {"name": "go/a", "rel_path": "code/packages/go/a", "language": "go"},
    {"name": "go/b", "rel_path": "code/packages/go/b", "language": "go"},
    {"name": "py/z", "rel_path": "code/packages/python/z", "language": "python"}
  ],
  "shards": [
    {"index": 1, "name": "s1", "assigned_packages": ["go/b"], "package_names": ["go/b"]},
    {"index": 0, "name": "s0", "assigned_packages": ["go/a", "py/z"], "package_names": ["py/z", "go/a", "go/b"]}
  ]
}`

func TestParsePartition(t *testing.T) {
	p, err := parsePartition([]byte(goodPlan), 2)
	if err != nil {
		t.Fatal(err)
	}
	want := []recfmt.ShardPlan{
		{Index: 0, Packages: []string{"code/packages/go/a", "code/packages/go/b", "code/packages/python/z"},
			Assigned: []string{"code/packages/go/a", "code/packages/python/z"}},
		{Index: 1, Packages: []string{"code/packages/go/b"}, Assigned: []string{"code/packages/go/b"}},
	}
	if !reflect.DeepEqual(p.Shards, want) {
		t.Fatalf("%+v", p.Shards)
	}
	runs, err := p.Runs(0)
	if err != nil || len(runs) != 3 {
		t.Fatalf("%v %v", runs, err)
	}
	runs[0] = "mutated"
	if again, _ := p.Runs(0); again[0] == "mutated" {
		t.Fatal("Runs returned the plan's own slice")
	}
	if _, err := p.Runs(2); err == nil {
		t.Fatal("a shard that does not exist was found")
	}
	d1, err := p.Digest()
	if err != nil || len(d1) != 64 {
		t.Fatalf("%q %v", d1, err)
	}
	// Package order within the plan file is irrelevant to the digest.
	p2, _ := parsePartition([]byte(strings.Replace(goodPlan, `"package_names": ["py/z", "go/a", "go/b"]`, `"package_names": ["go/b", "py/z", "go/a"]`, 1)), 2)
	if d2, _ := p2.Digest(); d2 != d1 {
		t.Fatal("the digest depends on list order")
	}
}

func TestParsePartitionRefusals(t *testing.T) {
	sub := func(old, new string) string { return strings.Replace(goodPlan, old, new, 1) }
	cases := map[string]struct{ plan, want string }{
		"not json":            {`{`, "plan:"},
		"trailing data":       {goodPlan + `{}`, "unexpected data"},
		"wrong shard count":   {goodPlan, "has 2 shards, want exactly 3"},
		"index out of range":  {sub(`"index": 1`, `"index": 5`), "out of range"},
		"repeated index":      {sub(`"index": 1`, `"index": 0`), "repeated"},
		"unknown package":     {sub(`"package_names": ["go/b"]`, `"package_names": ["go/nope"]`), "not in the plan"},
		"unknown assigned":    {sub(`"assigned_packages": ["go/b"]`, `"assigned_packages": ["go/nope"]`), "not in the plan"},
		"assigned not run":    {sub(`"assigned_packages": ["go/b"], "package_names": ["go/b"]`, `"assigned_packages": ["go/a"], "package_names": ["go/b"]`), "does not run it"},
		"duplicate in list":   {sub(`"package_names": ["go/b"]`, `"package_names": ["go/b", "go/b"]`), "listed twice"},
		"duplicate package":   {sub(`{"name": "go/b", "rel_path": "code/packages/go/b", "language": "go"}`, `{"name": "go/a", "rel_path": "code/packages/go/b", "language": "go"}`), "listed twice"},
		"duplicate path":      {sub(`"rel_path": "code/packages/go/b"`, `"rel_path": "code/packages/go/a"`), "listed twice"},
		"path with ..":        {sub(`"rel_path": "code/packages/go/b"`, `"rel_path": "code/../b"`), "plan: package"},
		"absolute path":       {sub(`"rel_path": "code/packages/go/b"`, `"rel_path": "/code/b"`), "plan: package"},
		"trailing slash path": {sub(`"rel_path": "code/packages/go/b"`, `"rel_path": "code/b/"`), "plan: package"},
		"no name":             {sub(`"name": "go/b"`, `"name": ""`), "no name"},
	}
	for name, c := range cases {
		t.Run(name, func(t *testing.T) {
			count := 2
			if name == "wrong shard count" {
				count = 3
			}
			if _, err := parsePartition([]byte(c.plan), count); err == nil || !strings.Contains(err.Error(), c.want) {
				t.Fatalf("got %v, want %q", err, c.want)
			}
		})
	}
}

func TestReadPartitionFiles(t *testing.T) {
	d := t.TempDir()
	p := filepath.Join(d, "plan.json")
	if err := os.WriteFile(p, []byte(goodPlan), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, err := ReadPartition(p, 2); err != nil {
		t.Fatal(err)
	}
	if _, err := ReadPartition(filepath.Join(d, "absent.json"), 2); err == nil {
		t.Fatal("missing file accepted")
	}
	if _, err := ReadPartition(d, 2); err == nil || !strings.Contains(err.Error(), "not a regular file") {
		t.Fatalf("a directory: %v", err)
	}
	if err := os.WriteFile(p, []byte(`{"packages":[],"shards":[]}`), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, err := ReadPartition(p, 1); err == nil {
		t.Fatal("a plan with no shards accepted")
	}
}

func TestReadPartitionRefusesAnOversizedPlan(t *testing.T) {
	old := planLimit
	planLimit = 100
	defer func() { planLimit = old }()
	p := filepath.Join(t.TempDir(), "plan.json")
	if err := os.WriteFile(p, []byte(goodPlan), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, err := ReadPartition(p, 2); err == nil || !strings.Contains(err.Error(), "exceeds") {
		t.Fatal(err)
	}
}
