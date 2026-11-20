package trace

import (
	"errors"
	"strings"
	"testing"
)

// KILN04 D41: a metadata write through a pipe, socket or anonymous inode
// changes nothing that lives in a directory, so the call records no event.
// Every Parse here is cross-checked (crosscheck_init_test.go): the old scanner
// and the grammar reader must agree.

// fdMetaCalls maps each descriptor metadata write to its argument list, with
// FD standing where the descriptor goes.
var fdMetaCalls = map[string]string{
	"fchmod":       "fchmod(FD, 0600)",
	"fchown":       "fchown(FD, 1, 2)",
	"fsetxattr":    "fsetxattr(FD, \"user.x\", \"v\", 1, 0)",
	"fremovexattr": "fremovexattr(FD, \"user.x\")",
}

func fdMetaLine(call, fd, result string) string {
	return "1  " + strings.Replace(fdMetaCalls[call], "FD", fd, 1) + " = " + result + "\n1  +++ exited with 0 +++\n"
}

func TestAnonymousDescriptorMetadataWriteRecordsNothing(t *testing.T) {
	for call := range fdMetaCalls {
		for _, fd := range []string{"3<socket:[1234]>", "3<pipe:[77]>", "3<anon_inode:[eventfd]>"} {
			for _, res := range []string{"0", "-1 EPERM (Operation not permitted)", "-1 EBADF (Bad file descriptor)"} {
				text := fdMetaLine(call, fd, res)
				tr, err := parseText(t, text)
				if err != nil {
					t.Errorf("%q: %v", text, err)
					continue
				}
				if len(tr.Events) != 0 {
					t.Errorf("%q: events %v, want none", text, tr.Events)
				}
			}
		}
	}
}

func TestMetadataWriteStillRefusedWithoutAnAbsoluteOrAnonymousAnnotation(t *testing.T) {
	for call := range fdMetaCalls {
		for _, fd := range []string{"3", "3<x>", "3<socketx:[1]>", "3<anon_inode>", "3<Socket:[1]>", "AT_FDCWD"} {
			text := fdMetaLine(call, fd, "0")
			if _, err := parseText(t, text); err == nil {
				t.Errorf("%q accepted, want a refusal", text)
			}
		}
	}
	for _, fd := range []string{"3", "3<x>"} {
		_, err := parseText(t, fdMetaLine("fchmod", fd, "0"))
		var te *Error
		if !errors.As(err, &te) || te.Reason != reasonFDPath {
			t.Errorf("%s: %v, want %s", fd, err, reasonFDPath)
		}
	}
}

func TestMetadataWriteOnAnAbsolutePathIsUnchanged(t *testing.T) {
	for call := range fdMetaCalls {
		tr := mustParse(t, fdMetaLine(call, "3</r/a>", "0"))
		if len(tr.Events) != 1 || tr.Events[0].Path != "/r/a" || tr.Events[0].Kind != KindWrite {
			t.Errorf("%s: %v", call, tr.Events)
		}
	}
	// A path that merely contains the prefix is a path, not an anonymous object.
	tr := mustParse(t, fdMetaLine("fchmod", "3</tmp/socket:[1]>", "0"))
	if len(tr.Events) != 1 || tr.Events[0].Path != "/tmp/socket:[1]" {
		t.Errorf("path containing socket:[ : %v", tr.Events)
	}
}

// The annotation is the lexer's ANNOTATED token.  A quoted argument is a
// different token kind, so it cannot stand in for it.
func TestQuotedTextCannotFakeTheAnonymousAnnotation(t *testing.T) {
	for _, text := range []string{
		"1  fchmod(\"socket:[1]\", 0600) = 0\n1  +++ exited with 0 +++\n",
		"1  fchmod(3, \"socket:[1]\") = 0\n1  +++ exited with 0 +++\n",
		"1  fchmod(3<x>\"socket:[1]\", 0600) = 0\n1  +++ exited with 0 +++\n",
	} {
		if _, err := parseText(t, text); err == nil {
			t.Errorf("%q accepted, want a refusal", text)
		}
	}
	if !AnonymousFD("socket:[1]") || !AnonymousFD("pipe:[1]") || !AnonymousFD("anon_inode:[x]") ||
		AnonymousFD("/socket:[1]") || AnonymousFD("") || AnonymousFD("sock") {
		t.Error("AnonymousFD prefix test")
	}
}
