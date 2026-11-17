//go:build linux

package record

import (
	"errors"
	"io"
	"os"
	"strconv"
	"strings"
	"time"
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
	data, err := io.ReadAll(io.LimitReader(f, maxProcRead+1))
	if err != nil {
		return nil, err
	}
	if len(data) > maxProcRead {
		return nil, errTruncated // never parse a partial last line
	}
	return data, nil
}

// Boot is /proc/stat's btime and the clock tick rate. CLK_TCK is 100 on every
// Linux kernel this runs on (USER_HZ is fixed at 100 for the user-space ABI of
// the architectures kiln supports), and Go has no sysconf without cgo.
func (liveProc) Boot() (time.Time, int, error) {
	data, err := os.ReadFile("/proc/stat")
	if err != nil {
		return time.Time{}, 0, err
	}
	for _, line := range strings.Split(string(data), "\n") {
		if rest, ok := strings.CutPrefix(line, "btime "); ok {
			n, err := strconv.ParseInt(strings.TrimSpace(rest), 10, 64)
			if err != nil {
				return time.Time{}, 0, err
			}
			return time.Unix(n, 0), 100, nil
		}
	}
	return time.Time{}, 0, errors.New("/proc/stat has no btime")
}

// liveSurvivors scans the real /proc.
func liveSurvivors(q SurvivorQuery) ([]string, error) {
	return scanSurvivors(liveProc{}, q)
}
