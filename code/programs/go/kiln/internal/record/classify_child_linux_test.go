//go:build linux

package record

import (
	"strings"
	"testing"
)

// A child that tries to use more memory than its RLIMIT_DATA is aborted by the
// Go runtime ("out of memory"); the supervisor sees a process that did not exit
// 0, writes record.failed, and the step's status is untouched. (RLIMIT_DATA is
// enforced for anonymous mappings on Linux 4.7 and later.)
func TestChildExceedingItsMemoryBoundIsRecordFailed(t *testing.T) {
	line := playLine("", "z", 7)
	f := newStepFixCustom(t, planOf(nil, nil, []string{line}), nil, true)
	f.give("z", actZ, line, 7, cannedOpts{})
	o := f.opts(actZ, 0, 1, line)
	o.ChildCmd = testChildCmd("alloc")
	o.ChildDataLimit = 400 << 20
	res, err := RunStep(o)
	if err != nil {
		t.Fatal(err)
	}
	if res.ExitCode != 7 || res.Record != RecordFailed || strings.Contains(res.RecordWhy, "allocated") {
		t.Fatalf("exit %d record %q why %q", res.ExitCode, res.Record, res.RecordWhy)
	}
	if exists(RecordFile(f.p.dir, actZ)) {
		t.Fatal("a record.json beside the record.failed")
	}
}
