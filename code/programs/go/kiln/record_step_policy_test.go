package main

// The policy files of TestSandboxRecordStep's fixture repository, built here so
// that a test on ANY OS decodes them with the strict decoders `prepare` uses
// (the Linux-only test once wrote host-roots.json with roots that were not
// strictly sorted by path, which only a Linux run could see).

import (
	"fmt"
	"sort"
	"strings"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
)

// recStepEphemeral is the fixture's ephemeral list: exactly the committed
// code/kiln/legacy/host-roots.json policy (sorted, as the decoder requires), so
// the fixture is no more permissive about /proc and /sys than the real policy.
// Ubuntu 26.04's binaries (libselinux 3.9, newer coreutils) read some of these
// at start-up; 24.04's, in the kiln-sandbox job, do not.
const recStepEphemeral = `["/dev/fd/*", "/dev/null", "/dev/urandom", "/proc/cpuinfo", "/proc/meminfo",
  "/proc/self/cgroup", "/proc/self/exe", "/proc/self/maps", "/proc/self/mountinfo", "/proc/stat",
  "/sys/devices/system/cpu/**", "/sys/fs/cgroup/**"]`

// recStepAbsentOK: libselinux's start-up probes of the SELinux mount points.
// They are absent on a runner without SELinux; a present one still voids the run.
const recStepAbsentOK = `["/selinux", "/sys/fs/selinux"]`

// recStepPolicyFiles returns the fixture's code/kiln/legacy files. imagePaths
// are the `image` roots, in any order and with duplicates: they are sorted and
// de-duplicated the way the decoder requires (strictly ascending by path).
func recStepPolicyFiles(imagePaths []string) map[string]string {
	seen := map[string]bool{}
	var paths []string
	for _, p := range imagePaths {
		if !seen[p] {
			seen[p] = true
			paths = append(paths, p)
		}
	}
	sort.Strings(paths)
	var roots []string
	for _, p := range paths {
		roots = append(roots, fmt.Sprintf(`{"path": %q, "kind": "image"}`, p))
	}
	return map[string]string{
		"code/kiln/legacy/pinned.json": `{"languages": []}`,
		"code/kiln/legacy/never.json":  `{"packages": []}`,
		"code/kiln/legacy/toolchains.json": `{
  "sh": {"always": true, "pins": {"sh": "image"}, "roots": {"sh": ["/usr/bin", "/bin"]}}
}
`,
		"code/kiln/legacy/env.json":        `{"names": ["FIXTURE_*", "LANG"]}`,
		legacy.BoundaryPath:                `{"schema_version": 1, "language_source_input_registry_sha256": "00", "boundaries": []}`,
		"code/kiln/legacy/host-roots.json": `{"roots": [` + strings.Join(roots, ",") + `], "ephemeral": ` + recStepEphemeral + `, "absent-ok": ` + recStepAbsentOK + `}`,
	}
}

func TestRecordStepFixturePolicyFilesDecode(t *testing.T) {
	// The order a Linux host's imageRoots() lists them in, plus a duplicate.
	files := recStepPolicyFiles([]string{"/usr", "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/libx32", "/etc", "/bin"})
	tc, err := legacy.ParseToolchains([]byte(files["code/kiln/legacy/toolchains.json"]))
	if err != nil {
		t.Fatal(err)
	}
	hr, err := legacy.ParseHostRoots([]byte(files["code/kiln/legacy/host-roots.json"]), tc)
	if err != nil {
		t.Fatal(err)
	}
	if len(hr.Roots) != 8 {
		t.Fatalf("%d roots", len(hr.Roots))
	}
	if _, err := legacy.ParseEnvPolicy([]byte(files["code/kiln/legacy/env.json"])); err != nil {
		t.Fatal(err)
	}
}
