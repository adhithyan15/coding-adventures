package trace

import (
	"strings"
	"testing"
)

// KILN04 D45: completed NULL-path statx failing with EFAULT (without
// AT_EMPTY_PATH).  Every Parse here runs both readers under the cross-check
// hook (crosscheck_init_test.go), so each case is also a parity test between
// the oracle scanner and the grammar parser.

func TestStatxNullEfaultAccepted(t *testing.T) {
	// Each accepted probe yields zero events; a trailing stat confirms the
	// surrounding trace still records normally and split-call state is reset.
	cases := []struct {
		name string
		line string
	}{
		{
			name: "rust std statx probe",
			line: `statx(AT_FDCWD</work/repo>, NULL, AT_STATX_SYNC_AS_STAT, STATX_ALL, 0x7ffd12345678) = -1 EFAULT (Bad address)`,
		},
		{
			name: "null buffer and numeric zero flags",
			line: `statx(AT_FDCWD</work/repo>, NULL, 0, STATX_ALL, NULL) = -1 EFAULT (Bad address)`,
		},
		{
			name: "unannotated AT_FDCWD with flags",
			line: `statx(AT_FDCWD, NULL, AT_SYMLINK_NOFOLLOW|AT_NO_AUTOMOUNT, STATX_BASIC_STATS, 0x1) = -1 EFAULT (Bad address)`,
		},
		{
			name: "annotated dirfd without AT_EMPTY_PATH",
			line: `statx(3</work/repo/sub>, NULL, AT_STATX_SYNC_AS_STAT, STATX_ALL, 0x1) = -1 EFAULT (Bad address)`,
		},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			tr := mustParse(t, wrap(tc.line, `stat("a", 0x1) = 0`))
			if len(tr.Events) != 1 || tr.Events[0].Syscall != "stat" || tr.Events[0].Path != "a" {
				t.Fatalf("events = %+v, want only trailing stat", tr.Events)
			}
			// Check zero cross-check divergences explicitly via ParseCrossCheckedDetail.
			_, err, mismatch, detail := ParseCrossCheckedDetail(strings.NewReader(wrap(tc.line)), "/work/repo", Options{})
			if err != nil || mismatch != nil || detail.Allowed != 0 {
				t.Fatalf("crosscheck: err=%v mismatch=%v detail=%+v", err, mismatch, detail)
			}
		})
	}
}

func TestStatxNullEfaultSplitCall(t *testing.T) {
	// A split statx(..., NULL, ...) = -1 EFAULT must still reset the per-proc
	// start state so the next call uses the post-chdir working directory and its
	// own StartLine.
	text := "1  statx(AT_FDCWD</work/repo>, NULL, AT_STATX_SYNC_AS_STAT <unfinished ...>\n" +
		"1  <... statx resumed>, STATX_ALL, NULL) = -1 EFAULT (Bad address)\n" +
		"1  chdir(\"/work/repo/sub\") = 0\n" +
		"1  stat(\"b\", 0x1) = 0\n" +
		"1  +++ exited with 0 +++\n"
	tr := mustParse(t, text)
	if len(tr.Events) != 2 || tr.Events[1].Syscall != "stat" {
		t.Fatalf("events = %+v", tr.Events)
	}
	if tr.Events[1].Cwd != "/work/repo/sub" || tr.Events[1].StartLine != 4 {
		t.Fatalf("trailing stat leaked split state: %+v", tr.Events[1])
	}
}

func TestStatxNullEfaultNegativeControls(t *testing.T) {
	cases := []struct {
		name   string
		line   string
		reason string
	}{
		{
			name:   "AT_EMPTY_PATH symbolic",
			line:   `statx(3</w/f>, NULL, AT_EMPTY_PATH, STATX_ALL, {}) = -1 EFAULT (Bad address)`,
			reason: reasonPathArg,
		},
		{
			name:   "AT_EMPTY_PATH hex",
			line:   `statx(3</w/f>, NULL, 0x1000, STATX_ALL, {}) = -1 EFAULT (Bad address)`,
			reason: reasonPathArg,
		},
		{
			name:   "AT_EMPTY_PATH decimal",
			line:   `statx(3</w/f>, NULL, 4096, STATX_ALL, {}) = -1 EFAULT (Bad address)`,
			reason: reasonPathArg,
		},
		{
			name:   "AT_EMPTY_PATH combined flags",
			line:   `statx(3</w/f>, NULL, AT_NO_AUTOMOUNT|0x1100, STATX_ALL, {}) = -1 EFAULT (Bad address)`,
			reason: reasonPathArg,
		},
		{
			name:   "unreadable flags",
			line:   `statx(3</w/f>, NULL, lower_bogus, STATX_ALL, {}) = -1 EFAULT (Bad address)`,
			reason: reasonFlags,
		},
		{
			name:   "success result with NULL path",
			line:   `statx(3</w/f>, NULL, 0, STATX_ALL, {}) = 0`,
			reason: reasonPathArg,
		},
		{
			name:   "ENOENT result with NULL path",
			line:   `statx(AT_FDCWD</w>, NULL, 0, STATX_ALL, 0x1) = -1 ENOENT (No such file or directory)`,
			reason: reasonPathArg,
		},
		{
			name:   "EINVAL result with NULL path",
			line:   `statx(AT_FDCWD</w>, NULL, 0, STATX_ALL, 0x1) = -1 EINVAL (Invalid argument)`,
			reason: reasonPathArg,
		},
		{
			name:   "unknown result with NULL path",
			line:   `statx(AT_FDCWD</w>, NULL, 0, STATX_ALL, 0x1) = ?`,
			reason: reasonResult,
		},
		{
			name:   "non-NULL pointer address with EFAULT",
			line:   `statx(3</w/f>, 0x1, 0, STATX_ALL, 0x1) = -1 EFAULT (Bad address)`,
			reason: reasonPathArg,
		},
		{
			name:   "unannotated dirfd",
			line:   `statx(3, NULL, 0, STATX_ALL, 0x1) = -1 EFAULT (Bad address)`,
			reason: reasonFDPath,
		},
		{
			name:   "invalid dirfd",
			line:   `statx(-1, NULL, 0, STATX_ALL, 0x1) = -1 EFAULT (Bad address)`,
			reason: reasonDirfd,
		},
		{
			name:   "newfstatat NULL EFAULT stays refused",
			line:   `newfstatat(AT_FDCWD</w>, NULL, 0x1, 0) = -1 EFAULT (Bad address)`,
			reason: reasonPathArg,
		},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			wantReason(t, wrap(tc.line), tc.reason)
		})
	}
}

func TestStatxNullQuotedStringNotDropped(t *testing.T) {
	// A quoted literal "NULL" is a real relative filename, not a NULL pointer.
	tr := mustParse(t, wrap(`statx(AT_FDCWD</work/repo>, "NULL", 0, STATX_ALL, 0x1) = -1 EFAULT (Bad address)`))
	if len(tr.Events) != 1 || tr.Events[0].Path != "NULL" || tr.Events[0].Result.Errno != "EFAULT" {
		t.Fatalf("events = %+v", tr.Events)
	}
}

func TestStatxNullDyingCallsRemainRefused(t *testing.T) {
	// D45 only applies to completed calls failing with EFAULT; dying calls keep
	// their D35/D44 refusal.
	for shape, body := range map[string]string{
		"entry":               "2  statx(AT_FDCWD</w>, NULL, <unfinished ...>\n" + dyGroupEnd,
		"resumed unavailable": "2  statx(AT_FDCWD</w>, NULL, 0, STATX_ALL, 0x1 <unfinished ...>\n" + dyGroupEnd + "2  <... statx resumed>) = ? <unavailable>\n",
		"resumed unfinished":  "2  statx(AT_FDCWD</w>, NULL, 0 <unfinished ...>\n" + dyGroupEnd + "2  <... statx resumed> <unfinished ...>) = ?\n",
	} {
		t.Run(shape, func(t *testing.T) {
			e := refusal(t, dyTrace(body), false)
			if e.Reason != reasonUnresumed {
				t.Errorf("%s: refused %s, want %s", shape, e.Reason, reasonUnresumed)
			}
		})
	}
}
