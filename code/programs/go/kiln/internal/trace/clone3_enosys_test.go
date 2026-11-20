package trace

import "testing"

// The sandbox's seccomp filter answers clone3 with ENOSYS so that glibc falls
// back to clone (KILN04 section 4.3 item 5), and answers every other refused
// call with EPERM.  strace still prints the failed call:
//
//	clone3({flags=...}, 88) = -1 ENOSYS (Function not implemented)
//
// Those failed calls are the filter WORKING and are tolerated (no event, no
// error).  Anything else for a refused name (a success, another errno, "= ?",
// a call that never returned) means the filter did not do its job and stays
// refused.  Every Parse here runs both readers and the cross-check hook
// (crosscheck_init_test.go), so a disagreement between them panics.

const (
	enosys = " = -1 ENOSYS (Function not implemented)"

	// glibc's pthread_create: nine flags and a tls= pointer (the real struct
	// of the thread tests above, as strace prints it for a call that failed:
	// there is no "=> {...}" output annotation).
	pthreadStruct = "{flags=" + realThreadFlags + realClone3Tail

	// glibc's posix_spawn: three flags and an exit_signal=.
	spawnStruct = "{flags=CLONE_VM|CLONE_VFORK|CLONE_CLEAR_SIGHAND, exit_signal=SIGCHLD, stack=0x7f0a4b1ff000, stack_size=0x9000}"
)

// clone3Lines is one clone3 call of pid by the given struct, written four ways:
// inline and split, each with the given result text (e.g. " = -1 ENOSYS (...)").
func clone3Forms(pid, st, result string) map[string]string {
	return map[string]string{
		"inline": pid + "  clone3(" + st + ", 88)" + result + "\n",
		"split":  pid + "  clone3(" + st + " <unfinished ...>\n" + pid + "  <... clone3 resumed>, 88)" + result + "\n",
	}
}

func TestClone3FailedWithENOSYSIsTheFilter(t *testing.T) {
	structs := map[string]string{"pthread_create": pthreadStruct, "posix_spawn": spawnStruct}
	for sname, st := range structs {
		for form, line := range clone3Forms("11", st, enosys) {
			name := sname + "/" + form
			// Without the helper option.
			mustParse(t, "11  clone3("+st+", 88)"+enosys+"\n11  +++ exited with 0 +++\n")
			// In the init window (before the step's exec) and in the step.
			mustHelper(t, helperRootSetup+line+helperInitSetup+helperReExec+helperExec+helperEnd)
			mustHelper(t, helperRootSetup+helperInitSetup+helperReExec+helperExec+line+helperEnd)
			// A tolerated call leaves no event and no process behind.
			tr := mustHelper(t, helperRootSetup+helperInitSetup+helperReExec+helperExec+line+helperEnd)
			if len(tr.Procs) != 2 {
				t.Errorf("%s: %d processes, want the root and the init child: %+v", name, len(tr.Procs), tr.Procs)
			}
			// By the root (pid 10): the root group's own threads.
			root := "10  clone3(" + st + ", 88)" + enosys + "\n"
			if form == "split" {
				root = "10  clone3(" + st + " <unfinished ...>\n10  <... clone3 resumed>, 88)" + enosys + "\n"
			}
			mustHelper(t, helperRootExec+root+helperRootSetup[len(helperRootExec):]+helperInitSetup+helperReExec+helperExec+helperEnd)
		}
	}
}

func TestClone3NotTheFiltersAnswerIsRefused(t *testing.T) {
	cases := map[string]string{
		"success":       " = 12",
		"EPERM":         " = -1 EPERM (Operation not permitted)",
		"EAGAIN":        " = -1 EAGAIN (Resource temporarily unavailable)",
		"unknown":       " = ?",
		"zero":          " = 0",
		"ENOSYS unseen": " = -1",
	}
	for cname, result := range cases {
		for form := range clone3Forms("11", pthreadStruct, result) {
			name := cname + "/" + form
			build := func(pid string) string {
				return clone3Forms(pid, pthreadStruct, result)[form]
			}
			// Not ok in the step, and not ok outside the helper either (the
			// result of a clone3 that is not ENOSYS is never the filter).
			_, err := parseAt(t, "/start", helperRootSetup+helperInitSetup+helperReExec+helperExec+build("11")+helperEnd, helperOpts)
			if err == nil {
				t.Errorf("%s: step: parsed", name)
			}
			_, err = parseAt(t, "/r", build("1")+"1  +++ exited with 0 +++\n", Options{})
			if err == nil {
				t.Errorf("%s: plain: parsed", name)
			}
		}
	}
	// The ones that parse as a refusal of the NAME say so.
	step := helperRootSetup + helperInitSetup + helperReExec + helperExec
	for _, result := range []string{" = -1 EPERM (Operation not permitted)", " = ?"} {
		wantReasonOpts(t, step+clone3Forms("11", spawnStruct, result)["inline"]+helperEnd, helperOpts, "refused:clone3")
	}
	wantReason(t, "1  clone3("+pthreadStruct+", 88) = -1 EPERM (Operation not permitted)\n1  +++ exited with 0 +++\n", "refused:clone3")
}

func TestClone3NeverResumedStaysUnresumed(t *testing.T) {
	// A clone3 that began but never returned (the pid exited inside it) is an
	// unresumed call, not a tolerated one, in the step and outside the helper.
	half := "11  clone3(" + pthreadStruct + " <unfinished ...>\n"
	step := helperRootSetup + helperInitSetup + helperReExec + helperExec
	wantReasonOpts(t, step+half+helperEnd, helperOpts, reasonUnresumed)
	wantReason(t, "1  clone3("+pthreadStruct+" <unfinished ...>\n1  +++ exited with 0 +++\n", reasonUnresumed)
}

func TestRefusedNamesTolerateOnlyTheFiltersEPERM(t *testing.T) {
	step := helperRootSetup + helperInitSetup + helperReExec + helperExec
	for _, n := range []string{"mount", "umount2", "unshare", "setns", "ptrace", "bpf", "pivot_root", "io_uring_setup"} {
		eperm := "11  " + n + "(0, 0) = -1 EPERM (Operation not permitted)\n"
		split := "11  " + n + "(0,  <unfinished ...>\n11  <... " + n + " resumed>0) = -1 EPERM (Operation not permitted)\n"
		for _, l := range []string{eperm, split} {
			// In the step, in the init window, and with no helper at all.
			mustHelper(t, step+l+helperEnd)
			mustHelper(t, helperRootSetup+l+helperInitSetup+helperReExec+helperExec+helperEnd)
		}
		mustParse(t, wrap(n+"(0, 0) = -1 EPERM (Operation not permitted)"))
		for _, bad := range []string{" = 0", " = -1 ENOENT (No such file or directory)", " = ?"} {
			wantReasonOpts(t, step+"11  "+n+"(0, 0)"+bad+"\n"+helperEnd, helperOpts, "refused:"+n)
			wantReason(t, wrap(n+"(0, 0)"+bad), "refused:"+n)
		}
	}
	// The split half of a refused name that is not the filter's is judged at
	// the resumed half, with the result.
	wantReasonOpts(t, step+"11  mount(0,  <unfinished ...>\n11  <... mount resumed>0) = 0\n"+helperEnd, helperOpts, "refused:mount")
	wantReason(t, "1  mount(0,  <unfinished ...>\n1  <... mount resumed>0) = 0\n1  +++ exited with 0 +++\n", "refused:mount")
}

func TestCloneNamespaceFlagsRefusedByTheFilterAreTolerated(t *testing.T) {
	// The filter answers a clone with CLONE_UNTRACED or CLONE_NEW* with EPERM.
	// It spawned nothing, so it is tolerated; a success is clone-flags, and a
	// flag we cannot name is clone-flags whatever the result.
	eperm := " = -1 EPERM (Operation not permitted)"
	for _, flags := range []string{"CLONE_NEWUSER|SIGCHLD", "CLONE_NEWNS|CLONE_NEWPID|SIGCHLD", "CLONE_UNTRACED|SIGCHLD"} {
		call := "clone(child_stack=NULL, flags=" + flags + ")"
		mustParse(t, wrap(call+eperm))
		step := helperRootSetup + helperInitSetup + helperReExec + helperExec
		mustHelper(t, step+"11  "+call+eperm+"\n"+helperEnd)
		wantReason(t, wrap(call+" = 2"), reasonCloneFlags)
		wantReason(t, wrap(call+" = -1 EAGAIN (Resource temporarily unavailable)"), reasonCloneFlags)
		wantReason(t, wrap(call+" = ?"), reasonCloneFlags)
	}
	wantReason(t, wrap("clone(child_stack=NULL, flags=CLONE_NEWUSER|0x10|SIGCHLD)"+eperm), reasonCloneFlags)
	// An EPERM for a clone with only harmless flags is not the filter's.
	mustParse(t, wrap("clone(child_stack=NULL, flags=SIGCHLD)"+eperm))
}
