package recfmt

import "testing"

// The binding names the digests of every binary the recorder ran from the
// host side: kiln, the build tool, strace and git. A merge that does not
// compare them yet still refuses a binding without them, so that a recorder
// that stops recording one is noticed.
func TestBindingRequiresTheStraceAndGitDigests(t *testing.T) {
	b := goodBinding()
	if err := b.Validate(); err != nil {
		t.Fatal(err)
	}
	for name, mut := range map[string]func(b *Binding){
		"strace digest empty":  func(b *Binding) { b.StraceSHA256 = "" },
		"strace digest short":  func(b *Binding) { b.StraceSHA256 = "abc" },
		"git digest empty":     func(b *Binding) { b.GitSHA256 = "" },
		"git digest uppercase": func(b *Binding) { b.GitSHA256 = "ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789" },
	} {
		c := goodBinding()
		mut(&c)
		if err := c.Validate(); err == nil {
			t.Errorf("%s: accepted", name)
		}
	}
	// Both keys are required on the wire too.
	data, err := EncodeBinding(b)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := DecodeBinding(data); err != nil {
		t.Fatal(err)
	}
	for _, key := range []string{`"strace_sha256"`, `"git_sha256"`} {
		if !containsStr(string(data), key) {
			t.Errorf("%s missing from %s", key, data)
		}
	}
}

func containsStr(s, sub string) bool {
	for i := 0; i+len(sub) <= len(s); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}
