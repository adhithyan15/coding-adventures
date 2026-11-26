package trace

import (
	"strings"
	"testing"
)

// Mutation check for KILN04 D50.  The refusal corpus is only worth something if
// it FAILS when a rule is weakened.  Each mutant below is installed in place of
// the real predicate (model.go's restartedSpawnCheck / bareReadOnlyOpenCheck, or
// the closed flag set) and the corpus must accept at least one shape it should
// refuse.

// d50Call builds a rawCall as the line readers hand it to the model.
func d50Call(name string, ret result, args ...string) *rawCall {
	return &rawCall{name: name, args: args, ret: ret}
}

var (
	d50Unknown = func(errno string) result { return result{unknown: true, errno: errno, spaced: true} }
	d50Fd      = result{hasVal: true, value: 3}
)

// d50RefusedSpawns are calls restartedSpawn must refuse.
func d50RefusedSpawns() []*rawCall {
	cloneArgs := []string{"child_stack=NULL", "flags=SIGCHLD"}
	return []*rawCall{
		d50Call("clone", d50Unknown("ERESTARTSYS"), cloneArgs...),
		d50Call("clone", d50Unknown("ERESTARTNOHAND"), cloneArgs...),
		d50Call("clone", d50Unknown("ERESTART_RESTARTBLOCK"), cloneArgs...),
		d50Call("clone", d50Unknown("?"), cloneArgs...), // a bare `= ?`
		d50Call("fork", d50Unknown("ERESTARTSYS")),
		d50Call("vfork", d50Unknown("?")),
		d50Call("clone3", d50Unknown("ERESTARTNOINTR"), "{flags=CLONE_VM}", "88"),
		d50Call("openat", d50Unknown("ERESTARTNOINTR"), "AT_FDCWD", "\"/x\"", "O_RDONLY"),
		d50Call("clone", d50Unknown("ERESTARTNOINTR"), "child_stack=NULL", "flags=CLONE_UNTRACED|SIGCHLD"),
		d50Call("clone", d50Unknown("ERESTARTNOINTR"), "child_stack=NULL", "flags=CLONE_BOGUS|SIGCHLD"),
		d50Call("clone", d50Unknown("ERESTARTNOINTR"), "child_stack=NULL", "flags=0x11"),
		{name: "clone", args: cloneArgs, ret: result{unknown: true, errno: "ERESTARTNOINTR", unavailable: true, spaced: true}},
		{name: "clone", args: cloneArgs, ret: result{unknown: true, errno: "ERESTARTNOINTR"}}, // `?ERESTARTNOINTR`, no space
	}
}

// d50RefusedOpens are calls bareReadOnlyOpen must refuse.
func d50RefusedOpens() []*rawCall {
	op := func(flags string) *rawCall {
		return d50Call("openat", d50Fd, "AT_FDCWD", "\"/repo/a\"", flags)
	}
	return []*rawCall{
		op("O_RDWR"), op("O_WRONLY"), op("O_PATH|O_WRONLY"), op("O_RDONLY|O_RDWR"), op("O_RDONLY|O_CREAT"), op("O_RDONLY|O_TRUNC"),
		op("O_RDONLY|O_TMPFILE"), op("O_RDONLY|O_APPEND"), op("O_RDONLY|O_EXCL"),
		op("O_RDONLY|O_BOGUS"), op("O_RDONLY|O_WRONLY"), op("O_RDONLY|O_CREAT|O_WRONLY"), op("O_RDONLY|0x40000"), op("O_RDONLY|64"), op("0"), op("O_CLOEXEC"),
		op("O_DIRECTORY"), op("o_rdonly"), op(""),
		d50Call("openat", d50Fd, "AT_FDCWD", "0x7ffd12345678", "O_RDONLY"), // a hex path
		d50Call("openat", d50Fd, "AT_FDCWD", "NULL", "O_RDONLY"),
		d50Call("open", d50Fd, "0x7ffd12345678", "O_RDONLY"),
		d50Call("openat2", d50Fd, "AT_FDCWD", "\"/repo/a\"", "{flags=O_RDONLY, resolve=0}", "24"),
		d50Call("creat", d50Fd, "\"/repo/a\"", "0644"),
		d50Call("openat", result{hasVal: true, value: 3, hex: true}, "AT_FDCWD", "\"/repo/a\"", "O_RDONLY"),
		d50Call("openat", result{hasVal: true, value: 3, hasHost: true, host: 4}, "AT_FDCWD", "\"/repo/a\"", "O_RDONLY"),
		d50Call("openat", result{hasVal: true, value: 3, hasAnn: true, ann: []byte("/repo/a")}, "AT_FDCWD", "\"/repo/a\"", "O_RDONLY"),
		d50Call("openat", result{hasVal: true, value: 3, errno: "FOO"}, "AT_FDCWD", "\"/repo/a\"", "O_RDONLY"),
		d50Call("openat", result{hasVal: true, value: -1}, "AT_FDCWD", "\"/repo/a\"", "O_RDONLY"),
	}
}

// d50Escapes counts the refused shapes the CURRENT predicates accept, at the
// predicate level and end to end.
func d50Escapes(t *testing.T) int {
	t.Helper()
	n := 0
	for _, c := range d50RefusedSpawns() {
		if restartedSpawnCheck(c, lookup(c.name)) {
			t.Logf("spawn accepted: %s %v %+v", c.name, c.args, c.ret)
			n++
		}
	}
	for _, c := range d50RefusedOpens() {
		if bareReadOnlyOpenCheck(c, lookup(c.name)) {
			t.Logf("open accepted: %s %v %+v", c.name, c.args, c.ret)
			n++
		}
	}
	return n
}

// d50EndToEndEscapes parses the neighbour traces and counts the accepted ones.
func d50EndToEndEscapes(t *testing.T) int {
	t.Helper()
	n := 0
	for _, text := range []string{
		"1  clone(" + d50Flags + ") = ? ERESTARTSYS (To be restarted if SA_RESTART is set)\n1  +++ exited with 0 +++\n",
		"1  fork() = ? ERESTARTSYS (To be restarted if SA_RESTART is set)\n1  +++ exited with 0 +++\n",
		"1  openat(AT_FDCWD, \"/repo/a\", O_RDONLY)" + d50Restart + "\n1  +++ exited with 0 +++\n",
		d50Open("openat(AT_FDCWD, \"/repo/a\", O_RDWR) = 3"),
		d50Open("openat(AT_FDCWD, \"/repo/a\", O_RDONLY|O_CREAT, 0644) = 3"),
		d50Open("openat(AT_FDCWD, \"/repo/a\", O_RDONLY|O_BOGUS) = 3"),
		d50Open("openat(AT_FDCWD, \"/repo/a\", O_RDONLY|0x40000) = 3"),
		d50Open("openat(AT_FDCWD, 0x7ffd12345678, O_RDONLY) = 3"),
	} {
		if _, err := parseText(t, text); err == nil {
			n++
		}
	}
	return n
}

func TestD50RealPredicatesHaveNoEscapes(t *testing.T) {
	if n := d50Escapes(t); n != 0 {
		t.Fatalf("%d refused shapes accepted", n)
	}
	if n := d50EndToEndEscapes(t); n != 0 {
		t.Fatalf("%d refused traces accepted", n)
	}
}

func TestD50MutantsAreKilled(t *testing.T) {
	widen := func(flag string) func() func() {
		return func() func() {
			old := bareReadOnlyOpenCheck
			bareReadOnlyOpenCheck = func(c *rawCall, ent entry) bool {
				if ent.flags < 1 || len(c.args) < ent.flags {
					return old(c, ent)
				}
				cc := *c
				cc.args = append([]string(nil), c.args...)
				var keep []string
				for _, tok := range strings.Split(cc.args[ent.flags-1], "|") {
					if tok != flag {
						keep = append(keep, tok)
					}
				}
				cc.args[ent.flags-1] = strings.Join(keep, "|")
				return old(&cc, ent)
			}
			return func() { bareReadOnlyOpenCheck = old }
		}
	}
	mutants := map[string]func() (restore func()){
		// The errno-name check dropped: any unknown result passes as ERESTARTNOINTR.
		"drop the errno-name check": func() func() {
			old := restartedSpawnCheck
			restartedSpawnCheck = func(c *rawCall, ent entry) bool {
				cc := *c
				cc.ret.errno = "ERESTARTNOINTR"
				return old(&cc, ent)
			}
			return func() { restartedSpawnCheck = old }
		},
		// The closed flag set widened by one WRITE flag.
		// The closed flag set widened by one WRITE flag: modelled by a wrapper that
		// removes that flag from the call before the real predicate sees it, so the
		// shared flag map is never written (no race with another parse).
		"widen the flag set by O_WRONLY": widen("O_WRONLY"),
		"widen the flag set by O_CREAT":  widen("O_CREAT"),
		// A hex (unquoted) path accepted: the path check is bypassed.
		"accept a hex path": func() func() {
			old := bareReadOnlyOpenCheck
			bareReadOnlyOpenCheck = func(c *rawCall, ent entry) bool {
				cc := *c
				cc.args = append([]string(nil), c.args...)
				cc.args[ent.pth-1] = "\"/x\""
				return old(&cc, ent)
			}
			return func() { bareReadOnlyOpenCheck = old }
		},
	}
	for name, install := range mutants {
		t.Run(name, func(t *testing.T) {
			restore := install()
			defer restore()
			if d50Escapes(t)+d50EndToEndEscapes(t) == 0 {
				t.Fatalf("mutant %q survives: the refusal corpus accepted nothing it should refuse", name)
			}
		})
	}
	// The real predicates are back.
	TestD50RealPredicatesHaveNoEscapes(t)
}
