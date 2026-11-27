package main

import "testing"

func TestStracePtraceListenRace(t *testing.T) {
	const race = "/usr/bin/strace: ptrace(PTRACE_LISTEN,pid:4242,sig:0): Input/output error\n"
	cases := []struct {
		name   string
		code   int
		stderr string
		want   bool
	}{
		{"the race, strace 6.8 spelling", 1, race, true},
		{"the race without a trailing newline", 1, race[:len(race)-1], true},
		{"the race twice", 1, race + race, true},
		{"the race with a blank line", 1, race + "\n", true},
		{"signal-killed strace", -1, race, false},
		{"exit 2", 2, race, false},
		{"exit 0 with the message", 0, race, false},
		{"empty stderr", 1, "", false},
		{"only whitespace", 1, " \n", false},
		{"another ptrace failure", 1, "/usr/bin/strace: ptrace(PTRACE_SEIZE,pid:1): Operation not permitted\n", false},
		{"LISTEN with another errno", 1, "/usr/bin/strace: ptrace(PTRACE_LISTEN,pid:1,sig:0): No such process\n", false},
		{"the race plus another line", 1, race + "/usr/bin/strace: Exit of unknown pid 7 ignored\n", false},
		{"the race plus a helper line", 1, race + "kiln sandbox-init: something\n", false},
		{"a helper line that mentions the race", 1, "kiln sandbox-init: ptrace(PTRACE_LISTEN): Input/output error\n", false},
		{"unknown text", 1, "boom\n", false},
	}
	for _, c := range cases {
		if got := stracePtraceListenRace(c.code, []byte(c.stderr)); got != c.want {
			t.Errorf("%s: stracePtraceListenRace(%d, %q) = %v, want %v", c.name, c.code, c.stderr, got, c.want)
		}
	}
}

func TestStraceShouldRetry(t *testing.T) {
	for attempt := 1; attempt <= stracePtraceListenAttempts; attempt++ {
		wantRace := attempt < stracePtraceListenAttempts
		if got := straceShouldRetry(attempt, true); got != wantRace {
			t.Errorf("race on attempt %d: retry = %v, want %v", attempt, got, wantRace)
		}
		if straceShouldRetry(attempt, false) {
			t.Errorf("no race on attempt %d must never retry", attempt)
		}
	}
	if stracePtraceListenAttempts != 3 {
		t.Errorf("the attempt cap is %d, want 3", stracePtraceListenAttempts)
	}
}
