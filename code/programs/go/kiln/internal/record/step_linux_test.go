//go:build linux

package record

// Pdeathsig (KILN04 section 4.8, decision P12): a supervisor that is killed
// takes strace with it.
//
// The supervisor is a library function here, so the test runs it in a child
// process: this very test binary, started again with -test.run pointing at
// TestStepHelperProcess and told what to do by a JSON file named in the
// environment. The "strace" is the fixture's shell script, whose step line
// records its own pid and then `exec sleep 60`s (so the pid is the one the
// supervisor started). The test kills the helper with SIGKILL, which gives it
// no chance to clean anything up, and then waits for the "strace" to die.

import (
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"
	"testing"
	"time"
)

const helperEnv = "KILN_STEP_HELPER_PAYLOAD"

// helperPayload is the part of StepOptions a child process can be given (the
// rest are functions and writers).
type helperPayload struct {
	Dir, Cwd, KilnExe string
	Argv, Environ     []string
	Poll, Drain       time.Duration
}

// TestStepHelperProcess is the child. It does nothing unless the test below
// started it.
func TestStepHelperProcess(t *testing.T) {
	payload := os.Getenv(helperEnv)
	if payload == "" {
		t.Skip("only the helper process of TestStepKillingTheSupervisorKillsStrace runs this")
	}
	var hp helperPayload
	data, err := os.ReadFile(payload)
	if err != nil {
		os.Exit(90)
	}
	if err := json.Unmarshal(data, &hp); err != nil {
		os.Exit(91)
	}
	res, err := RunStep(StepOptions{Dir: hp.Dir, Argv: hp.Argv, Environ: hp.Environ, Cwd: hp.Cwd, KilnExe: hp.KilnExe, Poll: hp.Poll, Drain: hp.Drain})

	if err != nil {
		os.Stderr.WriteString(err.Error() + "\n")
		os.Exit(ExitRefused)
	}
	os.Exit(res.ExitCode)
}

func TestChildAttrAsksTheKernelToKillStraceWithTheSupervisor(t *testing.T) {
	a := childAttr()
	if a == nil || a.Pdeathsig != syscall.SIGKILL {
		t.Fatalf("%+v", a)
	}
}

func processGone(pid int) bool {
	if err := syscall.Kill(pid, 0); err != nil {
		return true
	}
	// A zombie that nobody has reaped (a container's init may not) is dead.
	stat, err := os.ReadFile("/proc/" + strconv.Itoa(pid) + "/stat")
	if err != nil {
		return true
	}
	rest := string(stat)
	if i := strings.LastIndexByte(rest, ')'); i >= 0 && i+2 < len(rest) {
		return rest[i+2] == 'Z'
	}
	return false
}

func TestStepKillingTheSupervisorKillsStrace(t *testing.T) {
	f := newStepFix(t)
	pidFile := filepath.Join(f.aux, "pid")
	line := `echo $$ > '` + pidFile + `.tmp'; mv '` + pidFile + `.tmp' '` + pidFile + `'; exec sleep 60`
	o := f.opts(actZ, 0, 1, line)
	payload := filepath.Join(f.aux, "payload.json")
	data, err := json.Marshal(helperPayload{Dir: o.Dir, Cwd: o.Cwd, KilnExe: o.KilnExe, Argv: o.Argv, Environ: o.Environ, Poll: o.Poll, Drain: o.Drain})
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(payload, data, 0o600); err != nil {
		t.Fatal(err)
	}
	cmd := exec.Command(os.Args[0], "-test.run=^TestStepHelperProcess$")
	cmd.Env = []string{helperEnv + "=" + payload, "PATH=/usr/bin:/bin"}
	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}
	reaped := false
	defer func() {
		if !reaped {
			cmd.Process.Kill()
			cmd.Wait()
		}
	}()
	var pid int
	for deadline := time.Now().Add(30 * time.Second); ; time.Sleep(20 * time.Millisecond) {
		if b, err := os.ReadFile(pidFile); err == nil {
			if pid, err = strconv.Atoi(strings.TrimSpace(string(b))); err == nil && pid > 0 {
				break
			}
		}
		if time.Now().After(deadline) {
			t.Fatal("the supervised step never started")
		}
	}
	defer syscall.Kill(pid, syscall.SIGKILL) // whatever happens, do not leave a sleeper
	if processGone(pid) {
		t.Fatalf("pid %d is not running before the supervisor was killed", pid)
	}
	if err := cmd.Process.Kill(); err != nil {
		t.Fatal(err)
	}
	cmd.Wait()
	reaped = true
	for deadline := time.Now().Add(15 * time.Second); !processGone(pid); time.Sleep(20 * time.Millisecond) {
		if time.Now().After(deadline) {
			t.Fatalf("strace (pid %d) outlived its supervisor", pid)
		}
	}
}
