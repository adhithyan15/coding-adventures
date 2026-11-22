package policyfix

import (
	"encoding/json"
	"strings"
	"testing"
)

// The helper returns the real lists, and D27's invariant holds in them:
// /sys/fs/selinux is never ephemeral, only absent-ok.
func TestListsAreTheRealPolicyAndKeepSELinuxAbsentOnly(t *testing.T) {
	e, a := Lists()
	var eph, absent []string
	if err := json.Unmarshal([]byte(e), &eph); err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal([]byte(a), &absent); err != nil {
		t.Fatal(err)
	}
	has := func(l []string, s string) bool {
		for _, x := range l {
			if x == s {
				return true
			}
		}
		return false
	}
	if !has(eph, "/proc/filesystems") || !has(absent, "/selinux") || !has(absent, "/sys/fs/selinux") {
		t.Fatalf("D27 entries missing: %s %s", e, a)
	}
	for _, x := range eph {
		if strings.Contains(x, "selinux") {
			t.Fatalf("%q must never be ephemeral", x)
		}
	}
}
