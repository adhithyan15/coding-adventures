package main

// The policy files of TestSandboxRecordStep's fixture repository, built here so
// that a test on ANY OS decodes them with the strict decoders `prepare` uses
// (the Linux-only test once wrote host-roots.json with roots that were not
// strictly sorted by path, which only a Linux run could see).

import (
	"encoding/json"
	"fmt"
	"sort"
	"sync"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/legacy"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/policyfix"
)

// The fixture's ephemeral and absent-ok lists ARE the committed
// code/kiln/legacy/host-roots.json lists (KILN04 D27), through the one shared
// helper: the fixture is neither more nor less permissive than the real policy,
// so the kiln-sandbox job on ubuntu-26.04 (libselinux 3.9 reads
// /proc/filesystems and probes /sys/fs/selinux at start-up) exercises the real
// allowances, and a policy change cannot leave the fixture behind.
//
// The lists are loaded LAZILY, never at package initialisation: the sandbox
// tests re-exec this test binary as a helper inside the sandbox, where the
// source tree is not visible, and a package-level load would panic in every
// helper process.  Only the code that builds a fixture repository (always in
// the outer test process) calls recStepLists.
var recStepListsOnce struct {
	sync.Once
	ephemeral, absentOK string
}

func recStepLists() (ephemeral, absentOK string) {
	recStepListsOnce.Do(func() {
		recStepListsOnce.ephemeral, recStepListsOnce.absentOK = policyfix.Lists()
	})
	return recStepListsOnce.ephemeral, recStepListsOnce.absentOK
}

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
	eph, absent := recStepLists()
	hostRootsJSON, err := marshalRecStepHostRoots(roots, eph, absent)
	if err != nil {
		panic(err)
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
		"code/kiln/legacy/host-roots.json": hostRootsJSON,
	}
}

func marshalRecStepHostRoots(roots []string, eph, absent string) (string, error) {
	rawRoots := make([]json.RawMessage, len(roots))
	for i, r := range roots {
		rawRoots[i] = json.RawMessage(r)
	}
	encoded, err := json.Marshal(struct {
		Roots     []json.RawMessage `json:"roots"`
		Ephemeral json.RawMessage   `json:"ephemeral"`
		AbsentOK  json.RawMessage   `json:"absent-ok"`
	}{
		Roots:     rawRoots,
		Ephemeral: json.RawMessage(eph),
		AbsentOK:  json.RawMessage(absent),
	})
	if err != nil {
		return "", err
	}
	return string(encoded), nil
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
