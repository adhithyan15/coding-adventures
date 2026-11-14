package main

import (
	"errors"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/recfmt"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/record"
)

// recordCollectHooks restores the collect hook after a test.
func recordCollectHooks(t *testing.T) {
	t.Helper()
	recordHooks(t)
	c := recordCollect
	t.Cleanup(func() { recordCollect = c })
}

func collectArgs() []string {
	return []string{"collect", "--root", absPath("/r"), "--dir", absPath("/d"), "--results", absPath("/d/results.json"), "--out", absPath("/d/records.jsonl")}
}

// `collect` is no longer a stub: usage and platform come first, and nothing
// reaches record.Collect on a refusal.
func TestRecordCollectUsageAndPlatform(t *testing.T) {
	recordCollectHooks(t)
	recordCollect = func(record.CollectOptions) (*record.CollectResult, error) {
		t.Fatal("Collect was reached on a refusal")
		return nil, nil
	}
	recordPlatformOK = func() bool { return true }
	for name, args := range map[string][]string{
		"nothing":         {"collect"},
		"no root":         {"collect", "--dir", absPath("/d"), "--results", absPath("/x"), "--out", absPath("/o")},
		"no dir":          {"collect", "--root", absPath("/r"), "--results", absPath("/x"), "--out", absPath("/o")},
		"no results":      {"collect", "--root", absPath("/r"), "--dir", absPath("/d"), "--out", absPath("/o")},
		"no out":          {"collect", "--root", absPath("/r"), "--dir", absPath("/d"), "--results", absPath("/x")},
		"extra argument":  append(collectArgs(), "stray"),
		"an unknown flag": append(collectArgs(), "--nope"),
		"relative root":   {"collect", "--root", "r", "--dir", absPath("/d"), "--results", absPath("/x"), "--out", absPath("/o")},
		"relative dir":    {"collect", "--root", absPath("/r"), "--dir", "d", "--results", absPath("/x"), "--out", absPath("/o")},
		"relative result": {"collect", "--root", absPath("/r"), "--dir", absPath("/d"), "--results", "x", "--out", absPath("/o")},
		"relative out":    {"collect", "--root", absPath("/r"), "--dir", absPath("/d"), "--results", absPath("/x"), "--out", "o"},
	} {
		if code, out, e := runRec(args...); code != 2 || out != "" || e == "" {
			t.Errorf("%s: %d %q %q", name, code, out, e)
		}
	}
	recordPlatformOK = func() bool { return false }
	code, out, e := runRec(collectArgs()...)
	if code != 3 || out != "" || !strings.Contains(e, "linux/amd64") || !strings.Contains(e, "nothing was written") {
		t.Fatalf("%d %q %q", code, out, e)
	}
}

func TestRecordCollectPassesTheFlagsAndReports(t *testing.T) {
	recordCollectHooks(t)
	recordPlatformOK = func() bool { return true }
	recordSelf = func() (string, error) { return "/canonical/kiln", nil }
	var got record.CollectOptions
	recordCollect = func(o record.CollectOptions) (*record.CollectResult, error) {
		got = o
		return &record.CollectResult{
			Out: o.Out, Records: 5,
			Header:  recfmt.ShardHeader{ShardIndex: 1, ShardCount: 4, Void: []string{"tracked-changed"}, TrackedChangedUnexplained: 2, PostInventoryUnexplained: 3},
			Unknown: []string{"code/x: serial-violated"},
		}, nil
	}
	code, out, e := runRec(collectArgs()...)
	if code != 0 || e != "" {
		t.Fatalf("%d %q", code, e)
	}
	if got.Root != absPath("/r") || got.Dir != absPath("/d") || got.Results != absPath("/d/results.json") || got.Out != absPath("/d/records.jsonl") || got.KilnExe != "/canonical/kiln" {
		t.Errorf("options %+v", got)
	}
	for _, want := range []string{"shard 1 of 4", "5 records", "tracked-changed", "tracked_changed_unexplained 2", "post_inventory_unexplained 3", "unknown write set: code/x: serial-violated"} {
		if !strings.Contains(out, want) {
			t.Errorf("output lacks %q:\n%s", want, out)
		}
	}
}

func TestRecordCollectRefusalsExitOne(t *testing.T) {
	recordCollectHooks(t)
	recordPlatformOK = func() bool { return true }
	recordSelf = func() (string, error) { return "", errors.New("no executable") }
	if code, out, e := runRec(collectArgs()...); code != 1 || out != "" || !strings.Contains(e, "cannot locate") {
		t.Fatalf("%d %q %q", code, out, e)
	}
	recordSelf = func() (string, error) { return "/k", nil }
	recordCollect = func(record.CollectOptions) (*record.CollectResult, error) {
		return nil, errors.New("the directory is locked")
	}
	if code, out, e := runRec(collectArgs()...); code != 1 || out != "" || !strings.Contains(e, "the directory is locked") {
		t.Fatalf("%d %q %q", code, out, e)
	}
}
