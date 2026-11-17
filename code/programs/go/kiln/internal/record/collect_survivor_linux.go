//go:build linux

package record

import (
	"io"
	"os"
	"strconv"
)

// maxProcRead bounds one read of a /proc file (a command line or a mountinfo of
// a process that is not ours must not be able to make collect allocate).
const maxProcRead = 4 << 20

// liveProc is the real /proc.
type liveProc struct{}

func (liveProc) Pids() ([]int, error) {
	entries, err := os.ReadDir("/proc")
	if err != nil {
		return nil, err
	}
	var out []int
	for _, e := range entries {
		if n, err := strconv.Atoi(e.Name()); err == nil && n > 0 && strconv.Itoa(n) == e.Name() {
			out = append(out, n)
		}
	}
	return out, nil
}

func (liveProc) Self() int   { return os.Getpid() }
func (liveProc) UID() uint32 { return uint32(os.Geteuid()) }

func (liveProc) Owner(pid int) (uint32, error) {
	st, err := os.Lstat("/proc/" + strconv.Itoa(pid))
	if err != nil {
		return 0, err
	}
	uid, ok := ownerOf(st)
	if !ok {
		return 0, os.ErrInvalid
	}
	return uid, nil
}

func (liveProc) ReadLink(pid int, name string) (string, error) {
	return os.Readlink("/proc/" + strconv.Itoa(pid) + "/" + name)
}

func (liveProc) ReadFile(pid int, name string) ([]byte, error) {
	f, err := os.Open("/proc/" + strconv.Itoa(pid) + "/" + name)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	return io.ReadAll(io.LimitReader(f, maxProcRead))
}

// liveSurvivors scans the real /proc.
func liveSurvivors(repoRoot, cfgPath string) ([]string, error) {
	return scanSurvivors(liveProc{}, repoRoot, cfgPath)
}
