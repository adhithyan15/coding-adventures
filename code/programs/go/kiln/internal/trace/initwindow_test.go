package trace

import "testing"

// The shapes the sandbox init's setup makes in its window (KILN04 section 4.7):
// every one must parse, and none may leak into the events.  The lines follow
// strace 6.8 -y output as seen in the first real CI traces.
func TestInitWindowSetupLinesAreTolerated(t *testing.T) {
	const p = "AT_FDCWD</start>"
	lines := []string{
		`11  openat(` + p + `, "/proc/self/setgroups", O_WRONLY) = 5</proc/11/setgroups>`,
		`11  openat(` + p + `, "/proc/self/uid_map", O_WRONLY) = 5</proc/11/uid_map>`,
		`11  socket(AF_INET, SOCK_DGRAM|SOCK_CLOEXEC, IPPROTO_IP) = 5<socket:[123]>`,
		`11  mount("", "/", "", MS_REC|MS_PRIVATE, NULL) = 0`,
		`11  mount("tmpfs", "/tmp/x", "tmpfs", MS_NOSUID|MS_NODEV, "mode=0755,uid=1000,gid=1000") = 0`,
		`11  mkdir("/tmp/x/.kiln-newroot", 0755) = 0`,
		`11  mkdir("/tmp/x/.kiln-newroot/usr", 0755) = -1 EEXIST (File exists)`,
		`11  newfstatat(` + p + `, "/usr", {st_mode=S_IFDIR|0755, st_size=4096, ...}, AT_SYMLINK_NOFOLLOW) = 0`,
		`11  readlinkat(` + p + `, "/bin", "usr/bin", 128) = 7`,
		`11  symlink("usr/bin", "/tmp/x/.kiln-newroot/bin") = 0`,
		`11  symlink("/proc/self/fd", "/tmp/x/.kiln-newroot/dev/fd") = 0`,
		`11  openat(` + p + `, "/tmp/x/.kiln-newroot/dev/null", O_WRONLY|O_CREAT|O_EXCL|O_CLOEXEC, 0666) = 6</tmp/x/.kiln-newroot/dev/null>`,
		`11  openat(` + p + `, "/tmp/x/.kiln-newroot/start/.git", O_RDONLY|O_NOFOLLOW|O_PATH|O_CLOEXEC) = 6</tmp/x/.kiln-newroot/start/.git>`,
		`11  openat(` + p + `, "/tmp/x/.kiln-newroot/start/.git", O_PATH|O_NOFOLLOW|O_CLOEXEC) = 6</tmp/x/.kiln-newroot/start/.git>`,
		`11  mount("tmpfs", "/proc/self/fd/6", "tmpfs", MS_RDONLY|MS_NOSUID|MS_NODEV|MS_NOEXEC, "mode=0555,size=4k,nr_inodes=2") = 0`,
		`11  mount("/dev/null", "/proc/self/fd/6", NULL, MS_BIND, NULL) = 0`,
		`11  mount("", "/tmp/x/.kiln-newroot", "", MS_RDONLY|MS_NOSUID|MS_NODEV|MS_REMOUNT, NULL) = 0`,
		`11  chdir("/tmp/x/.kiln-newroot") = 0`,
		`11  pivot_root(".", ".") = 0`,
		`11  umount2(".", MNT_DETACH) = 0`,
		`11  chdir("/") = 0`,
		`11  lstat("/run/docker.sock", 0x7ffd1) = -1 ENOENT (No such file or directory)`,
		`11  newfstatat(AT_FDCWD</>, "/run/user", 0x7ffd1, AT_SYMLINK_NOFOLLOW) = -1 ENOENT (No such file or directory)`,
		`11  openat(AT_FDCWD</>, "/dev/null", O_RDONLY|O_CLOEXEC) = 6</dev/null>`,
		`11  dup3(6, 0, 0) = 0`,
		`11  chdir("/start") = 0`,
	}
	for _, l := range lines {
		text := helperRootSetup + "11  unshare(CLONE_FS) = 0\n" + l + "\n" + "11  chdir(\"/start\") = 0\n" + helperExec + helperEnd
		tr, err := parseAt(t, "/start", text, helperOpts)
		if err != nil {
			t.Errorf("%s\n   %v", l, err)
			continue
		}
		for _, e := range tr.Events {
			if e.Syscall != "execve" {
				t.Errorf("%s leaked into the events: %+v", l, e)
			}
		}
	}
}
