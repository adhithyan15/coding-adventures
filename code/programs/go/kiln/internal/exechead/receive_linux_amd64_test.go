//go:build linux && amd64

package exechead

// The hand-over's receiving side (KILN04 D48 step 3's contract, step 4's check):
// exactly one descriptor and exactly one data byte, 'L'; anything else fails the
// step; and a deadline, because strace holds its own copy of the socket and never
// closes it, so end of file will not come when the hand-over is lost.
//
// These tests need no seccomp and no namespace: the "listener" is /dev/null.

import (
	"os"
	"strings"
	"syscall"
	"testing"
	"time"
	"unsafe"
)

// uintptrOf is the address of a byte of this process's own memory.
func uintptrOf(p *byte) uintptr { return uintptr(unsafe.Pointer(p)) }

// recvFixture is a supervisor with only what receiveListener touches, and the
// sending end of its socket.
func recvFixture(t *testing.T, handover time.Duration) (*Supervisor, int) {
	t.Helper()
	pair, err := syscall.Socketpair(syscall.AF_UNIX, syscall.SOCK_STREAM|syscall.SOCK_CLOEXEC, 0)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { syscall.Close(pair[0]); syscall.Close(pair[1]) })
	return &Supervisor{parent: pair[0], tm: Timing{Handover: handover}.withDefaults()}, pair[1]
}

func devNull(t *testing.T) int {
	t.Helper()
	fd, err := syscall.Open("/dev/null", syscall.O_RDONLY|syscall.O_CLOEXEC, 0)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { syscall.Close(fd) })
	return fd
}

func send(t *testing.T, sock int, data []byte, fds ...int) {
	t.Helper()
	var oob []byte
	if len(fds) > 0 {
		oob = syscall.UnixRights(fds...)
	}
	if _, err := syscall.SendmsgN(sock, data, oob, nil, syscall.MSG_NOSIGNAL); err != nil {
		t.Fatal(err)
	}
}

func isCloexec(t *testing.T, fd int) bool {
	t.Helper()
	flags, _, e := syscall.Syscall(syscall.SYS_FCNTL, uintptr(fd), syscall.F_GETFD, 0)
	if e != 0 {
		t.Fatal(e)
	}
	return flags&syscall.FD_CLOEXEC != 0
}

func TestReceiveListenerAcceptsExactlyOneDescriptorAndOneByte(t *testing.T) {
	s, tx := recvFixture(t, 5*time.Second)
	send(t, tx, []byte{'L'}, devNull(t))
	fd, _, err := s.receiveListener()
	if err != nil {
		t.Fatal(err)
	}
	defer syscall.Close(fd)
	if !isCloexec(t, fd) {
		t.Error("the received descriptor is not close-on-exec (MSG_CMSG_CLOEXEC)")
	}
}

func TestReceiveListenerRefusesAnythingElse(t *testing.T) {
	cases := []struct {
		name string
		do   func(t *testing.T, tx int)
		want string
	}{
		{"two descriptors", func(t *testing.T, tx int) { send(t, tx, []byte{'L'}, devNull(t), devNull(t)) }, "2 descriptors"},
		{"no descriptor", func(t *testing.T, tx int) { send(t, tx, []byte{'L'}) }, "0 descriptors"},
		{"the wrong byte", func(t *testing.T, tx int) { send(t, tx, []byte{'X'}, devNull(t)) }, "want exactly one 'L' or 'K'"},
		{"two data bytes", func(t *testing.T, tx int) { send(t, tx, []byte("LL"), devNull(t)) }, "2 data bytes"},
		{"more descriptors than the buffer", func(t *testing.T, tx int) {
			fds := make([]int, 8)
			for i := range fds {
				fds[i] = devNull(t)
			}
			send(t, tx, []byte{'L'}, fds...)
		}, ""},
		{"a second message behind the first", func(t *testing.T, tx int) {
			send(t, tx, []byte{'L'}, devNull(t))
			send(t, tx, []byte{'L'}, devNull(t))
		}, "more than the one message"},
		{"extra data behind the first", func(t *testing.T, tx int) {
			send(t, tx, []byte{'L'}, devNull(t))
			send(t, tx, []byte("x"))
		}, "more than the one message"},
		{"end of file with nothing sent", func(t *testing.T, tx int) { syscall.Close(tx) }, "closed without the listener"},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			s, tx := recvFixture(t, 5*time.Second)
			c.do(t, tx)
			fd, _, err := s.receiveListener()
			if err == nil {
				syscall.Close(fd)
				t.Fatal("accepted")
			}
			if c.want != "" && !strings.Contains(err.Error(), c.want) {
				t.Fatalf("error %q does not mention %q", err, c.want)
			}
		})
	}
}

// strace holds its own copy of the child end and never closes it, so a lost
// hand-over is silence, not end of file: the deadline is the ONLY way to notice.
func TestReceiveListenerDeadlineIsTheOnlyWayToNoticeALostHandover(t *testing.T) {
	s, tx := recvFixture(t, 300*time.Millisecond)
	_ = tx // open, silent: another holder keeps it alive
	start := time.Now()
	fd, _, err := s.receiveListener()
	if err == nil {
		syscall.Close(fd)
		t.Fatal("a listener out of nothing")
	}
	if took := time.Since(start); took < 250*time.Millisecond || took > 10*time.Second {
		t.Fatalf("returned after %s, want about 300ms", took)
	}
	if !strings.Contains(err.Error(), "no listener within") {
		t.Fatalf("%v", err)
	}
}

func TestReceiveListenerSurvivesASlowSender(t *testing.T) {
	s, tx := recvFixture(t, 10*time.Second)
	go func() {
		time.Sleep(200 * time.Millisecond)
		send(t, tx, []byte{'L'}, devNull(t))
	}()
	fd, _, err := s.receiveListener()
	if err != nil {
		t.Fatal(err)
	}
	syscall.Close(fd)
}

// A refused hand-over closes every descriptor it received: nothing leaks.
func TestReceiveListenerClosesWhatItRefused(t *testing.T) {
	count := func() int {
		ents, err := os.ReadDir("/proc/self/fd")
		if err != nil {
			t.Fatal(err)
		}
		return len(ents)
	}
	s, tx := recvFixture(t, 5*time.Second)
	nul := devNull(t)
	before := count()
	send(t, tx, []byte("LL"), nul, nul)
	if fd, _, err := s.receiveListener(); err == nil {
		syscall.Close(fd)
		t.Fatal("accepted")
	}
	if after := count(); after != before {
		t.Fatalf("%d descriptors open before, %d after a refused hand-over", before, after)
	}
}

func TestNewMakesACloexecSocketAndWritesTheHeader(t *testing.T) {
	out, err := os.CreateTemp(t.TempDir(), "exechead")
	if err != nil {
		t.Fatal(err)
	}
	defer out.Close()
	s, err := New(Config{Out: out, Header: Header{Nonce: testNonce, ActionSHA256: testAction, Step: 1}})
	if err != nil {
		t.Fatal(err)
	}
	if !isCloexec(t, s.parent) {
		t.Error("the supervisor's end of the socket is not close-on-exec")
	}
	if s.ChildEnd() == nil {
		t.Fatal("no child end")
	}
	res := s.Finish() // never started: closes both ends, reports the header only
	if res.Lines != 0 || res.Bytes == 0 || len(res.Unclean) != 0 {
		t.Fatalf("%+v", res)
	}
	data, err := os.ReadFile(out.Name())
	if err != nil {
		t.Fatal(err)
	}
	f, err := Decode(data, Limits{}, Expect{Nonce: testNonce, ActionSHA256: testAction, Step: 1})
	if err != nil {
		t.Fatal(err)
	}
	if f.Header.Kernel == "" || f.Header.V != Version {
		t.Fatalf("%+v", f.Header)
	}
	if f.SHA256 != res.SHA256 || f.Bytes != res.Bytes {
		t.Fatal("the result's digest is not the file's")
	}
}

func TestReadCStringIsBoundedAndPageSafe(t *testing.T) {
	pid := os.Getpid()
	// A string that ends just before an unmapped page is read whole.
	mem, err := syscall.Mmap(-1, 0, 2*pageSize, syscall.PROT_READ|syscall.PROT_WRITE, syscall.MAP_ANON|syscall.MAP_PRIVATE)
	if err != nil {
		t.Fatal(err)
	}
	defer syscall.Munmap(mem)
	if err := syscall.Mprotect(mem[pageSize:], syscall.PROT_NONE); err != nil {
		t.Fatal(err)
	}
	copy(mem[pageSize-6:], "/a/bc\x00")
	addr := uint64(uintptrOf(&mem[pageSize-6]))
	if got, res := readCString(pid, addr); res != pathOK || got != "/a/bc" {
		t.Fatalf("%q %v", got, res)
	}
	// A string that runs into the unmapped page without a NUL is unreadable.
	for i := range mem[:pageSize] {
		mem[i] = 'x'
	}
	if _, res := readCString(pid, uint64(uintptrOf(&mem[pageSize-6]))); res != pathUnreadable {
		t.Fatalf("a string into a hole: %v", res)
	}
	// No NUL within PATH_MAX is too long.
	long, err := syscall.Mmap(-1, 0, 3*pageSize, syscall.PROT_READ|syscall.PROT_WRITE, syscall.MAP_ANON|syscall.MAP_PRIVATE)
	if err != nil {
		t.Fatal(err)
	}
	defer syscall.Munmap(long)
	for i := range long {
		long[i] = 'y'
	}
	if _, res := readCString(pid, uint64(uintptrOf(&long[0]))); res != pathTooLong {
		t.Fatalf("no NUL: %v", res)
	}
	// A string of exactly PATH_MAX-1 bytes and a NUL is read.
	long[PathMax-1] = 0
	if got, res := readCString(pid, uint64(uintptrOf(&long[0]))); res != pathOK || len(got) != PathMax-1 {
		t.Fatalf("%d %v", len(got), res)
	}
	// An address nobody owns.
	if _, res := readCString(pid, 8); res != pathUnreadable {
		t.Fatalf("address 8: %v", res)
	}
}

func TestReceiveListenerSaysWhetherItIsKillable(t *testing.T) {
	for b, want := range map[byte]bool{'L': false, 'K': true} {
		s, tx := recvFixture(t, 5*time.Second)
		send(t, tx, []byte{b}, devNull(t))
		fd, killable, err := s.receiveListener()
		if err != nil {
			t.Fatalf("%q: %v", b, err)
		}
		syscall.Close(fd)
		if killable != want {
			t.Errorf("%q: killable %v", b, killable)
		}
	}
}

func TestCheckNotifSizesMatchesTheKernel(t *testing.T) {
	if err := checkNotifSizes(); err != nil {
		t.Fatal(err)
	}
}

func TestLastNSpidIsThisProcessIdInItsInnermostNamespace(t *testing.T) {
	if n := lastNSpid(os.Getpid()); n < 1 {
		t.Fatalf("lastNSpid = %d", n)
	}
	if n := lastNSpid(1 << 30); n != 0 {
		t.Fatalf("a thread that does not exist: %d", n)
	}
}

// A closed listener is never used again, and closing twice is harmless: the
// users hold the read lock across their ioctl, the closer the write lock.
func TestListenerCloseExcludesUsers(t *testing.T) {
	s := &Supervisor{}
	s.lfd.Store(int32(devNull(t)))
	// Detach the descriptor from the test's cleanup: closeListener owns it now.
	fd, err := syscall.Dup(int(s.lfd.Load()))
	if err != nil {
		t.Fatal(err)
	}
	s.lfd.Store(int32(fd))
	s.closeListener()
	s.closeListener()
	if s.lfd.Load() != -1 {
		t.Fatal("not closed")
	}
	if s.valid(1) {
		t.Error("a closed listener answered valid")
	}
	s.answer(1) // a no-op, not a failure
	if len(s.unclean) != 0 {
		t.Errorf("unclean %v", s.unclean)
	}
}
