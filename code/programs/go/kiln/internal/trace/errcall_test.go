package trace

import (
	"errors"
	"testing"
)

// Error.Call names the call a refusal was raised for, from the closed table
// only, and never changes the error's text (the census reads it; no record does).
func TestErrorCallNamesTheRefusedSyscall(t *testing.T) {
	share := "1  clone(child_stack=NULL, flags=CLONE_VM|CLONE_FS|SIGCHLD) = 2\n"
	for _, c := range []struct{ text, call string }{
		// The sharer's open ran inside the chdir's interval (D46).
		{share + "1  chdir(\"/x\" <unfinished ...>\n2  openat(AT_FDCWD, \"f\", O_RDONLY) = 3</repo/f>\n1  <... chdir resumed>)  = 0\n", "chdir"},
		{share + "1  fchdir(3</x> <unfinished ...>\n2  openat(AT_FDCWD, \"f\", O_RDONLY) = 3</repo/f>\n1  <... fchdir resumed>)  = 0\n", "fchdir"},
		{"1  fchmod(3, 0644) = 0\n", "fchmod"},
	} {
		_, err := parseAt(t, "/repo", c.text, Options{})
		var te *Error
		if !errors.As(err, &te) {
			t.Fatalf("%q: %v", c.text, err)
		}
		if te.Call != c.call {
			t.Errorf("%q: Call = %q, want %q (err %v)", c.text, te.Call, c.call, err)
		}
		if te.Error() != "trace-incomplete:"+te.Reason {
			t.Errorf("Error() carries the call: %q", te.Error())
		}
	}
	// A line the grammar stops at before any call is modelled has no call.
	_, err := parseAt(t, "/repo", "1  ???\n", Options{})
	var te *Error
	if !errors.As(err, &te) || te.Call != "" {
		t.Errorf("unparsable line: %v call %q", err, func() string {
			if te != nil {
				return te.Call
			}
			return ""
		}())
	}
}
