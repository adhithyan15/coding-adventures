package key

import (
	"crypto/sha256"
	"encoding/hex"
	"testing"
)

// Definitions of the traced-key golden vectors (KILN04 section 11).  Like the
// rest of the file they are written by `-update`; the expected bytes are then
// cross-checked against an independent encoder written from the spec text.

func sum(s string) string {
	d := sha256.Sum256([]byte(s))
	return hex.EncodeToString(d[:])
}

func missingHex() string {
	d := MissingPackageDigest()
	return hex.EncodeToString(d[:])
}

// weakAction is the action of a weak key: semantics 2, empty inputs and reads,
// the four tool:/policy: consumed entries, the scrubbed environment and the
// kiln:* entries.
func weakAction() ActionJSON {
	host := "ubuntu-24.04-image-7"
	return ActionJSON{
		SemanticsVersion: 2, OS: "linux", Arch: "x64", HostImageID: &host,
		Target: "code/packages/go/example", Phase: "test",
		RecipeHex: hex.EncodeToString(mustRecipe([]byte("go test ./... -cover\n"), "BUILD")),
		Inputs:    []FileJSON{}, Reads: []FileJSON{},
		Consumed: []ConsumedJSON{
			{"policy:env.json", "build", dg(31)},
			{"policy:git", "build", dg(32)},
			{"policy:host-roots.json", "build", dg(33)},
			{"tool:build-tool", "build", dg(34)},
		},
		Tools: []ToolJSON{
			{"cc", "image", 2, hx("ubuntu-24.04-image-7")},
			{"go", "1.26.2", 2, hx("1.26.2")},
			{"sh", "image", 2, hx("ubuntu-24.04-image-7")},
		},
		Env: []EnvJSON{
			{"CI", "1", 0},
			{"HOME", "/home/runner", 0},
			{"LANG", "C.UTF-8", 0},
			{"PATH", "/opt/hostedtoolcache/go/1.26.2/x64/bin:/usr/bin", 0},
			{"TZ", "UTC", 0},
			{"kiln:executor-args", `"-emit-results=/r.json" "-exec-wrapper=/k"`, 0},
			{"kiln:recorder", "2", 0},
			{"kiln:root", "/home/runner/work/repo/repo", 0},
		},
	}
}

func tracedEntries() []EntryDigestJSON {
	return []EntryDigestJSON{
		{"pkg:code/packages/go/dep", sum("T(dep)")},
		{"path:code/scripts/helper.py", sum("state(helper)")},
		{"list:code/shared", sum("listing(shared)")},
		{"pkg:code/packages/go/gone", missingHex()},
	}
}

func weakKeyOf(t *testing.T) [32]byte {
	t.Helper()
	wa, err := weakAction().ToAction()
	if err != nil {
		t.Fatal(err)
	}
	wk, err := Key(wa)
	if err != nil {
		t.Fatal(err)
	}
	return wk
}

func tracedConsumed(t *testing.T) []ConsumedJSON {
	t.Helper()
	es, err := entryDigests(tracedEntries())
	if err != nil {
		t.Fatal(err)
	}
	cons, err := StrongConsumed(weakKeyOf(t), es)
	if err != nil {
		t.Fatal(err)
	}
	var out []ConsumedJSON
	for _, c := range cons {
		out = append(out, ConsumedJSON{c.Target, c.Phase, hex.EncodeToString(c.Key)})
	}
	return out
}

func tracedValid(t *testing.T) []namedVec {
	t.Helper()
	strong := weakAction()
	strong.Consumed = tracedConsumed(t)
	return []namedVec{
		{"traced-weak-key", "A KILN04 weak key: semantics 2, empty inputs and reads, tool:/policy: consumed entries, scrubbed env plus kiln:root, kiln:recorder and kiln:executor-args (KILN04 3.1).", weakAction()},
		{"traced-strong-key", "The strong key of traced-weak-key with four manifest entries (pkg:, path:, list:, and a missing package): consumed is weak: -> W plus the entries, everything else identical (KILN04 3.2). Its key differs from the weak key's.", strong},
	}
}

func tracedDigests() []DigestVector {
	own := sum("D(q)")
	return []DigestVector{
		{Name: "state-absent", Description: "A path with nothing tracked: SHA-256 of the single byte 00.", Kind: DigestAbsent},
		{Name: "state-directory", Description: "A directory: SHA-256 of the single byte 01.", Kind: DigestDirectory},
		{Name: "state-file", Description: "A regular file, not executable: SHA-256 of 02 || 00 || SHA-256(content).", Kind: DigestFile, Exec: 0, ContentHex: sum("echo hi\n")},
		{Name: "state-file-exec", Description: "The same content, executable: differs from state-file.", Kind: DigestFile, Exec: 1, ContentHex: sum("echo hi\n")},
		{Name: "state-link", Description: "A symbolic link: SHA-256 of 03 || bytes(target).", Kind: DigestLink, Target: "../lib/real.so"},
		{Name: "missing-package", Description: "A package that no longer exists: SHA-256 of str(\"kiln-missing-package\").", Kind: DigestMissingPkg},
		{Name: "listing-not-directory", Description: "A path that is not a directory in the tracked tree (absent, file or link): SHA-256 of the single byte 11.", Kind: DigestNotDir},
		{Name: "listing-mixed", Description: "A tracked directory (tag 10): a file, a link and a directory; names sorted bytewise ('B' before 'a'), non-ASCII included.", Kind: DigestListing,
			Entries: []ListingJSON{{"B", ListFile}, {"a", ListDir}, {"café", ListLink}, {"x.txt", ListFile}}},
		{Name: "listing-kind-matters", Description: "Same names as listing-mixed with 'a' a file instead of a directory: the digest differs.", Kind: DigestListing,
			Entries: []ListingJSON{{"B", ListFile}, {"a", ListFile}, {"café", ListLink}, {"x.txt", ListFile}}},
		{Name: "traced-package-no-nested", Description: "T(q) of a package with no nested package.", Kind: DigestTracedPkg, OwnDigestHex: own},
		{Name: "traced-package-nested", Description: "T(q) of the same D(q) with two directly nested packages: differs from traced-package-no-nested (KILN04 5.2).", Kind: DigestTracedPkg, OwnDigestHex: own,
			Nested: []string{"code/packages/go/q/ext/a", "code/packages/go/q/ext/b"}},
	}
}

func tracedStrongConsumed(t *testing.T) []StrongConsumedVector {
	t.Helper()
	weakHex := Hex(weakKeyOf(t))
	return []StrongConsumedVector{
		{Name: "strong-consumed-sorted", WeakKeyHex: weakHex, Entries: tracedEntries(), Consumed: tracedConsumed(t),
			Description: "Entries given out of order come out sorted bytewise (list:, path:, pkg:, then weak:); every phase is build."},
		{Name: "strong-consumed-empty-manifest", WeakKeyHex: weakHex, Entries: []EntryDigestJSON{},
			Description: "A manifest with no entries: consumed holds only weak: -> W.",
			Consumed:    []ConsumedJSON{{"weak:", "build", weakHex}}},
	}
}

func tracedRefused() []RefusedEntryVector {
	d := sum("x")
	wk := sum("weak")
	r := func(name, desc, errs string, es ...EntryDigestJSON) RefusedEntryVector {
		return RefusedEntryVector{Name: name, Description: desc, WeakKeyHex: wk, Entries: es, Error: errs}
	}
	return []RefusedEntryVector{
		r("refused-weak-prefix", "A manifest entry with the weak: prefix could forge the strong key's own entry.", "prefix is not pkg:, path: or list:", EntryDigestJSON{"weak:" + sum("other"), d}),
		r("refused-bare-weak", "The bare target weak: is not a manifest entry either.", "prefix is not pkg:, path: or list:", EntryDigestJSON{"weak:", d}),
		r("refused-unknown-prefix", "tool: is a weak-key entry, not a manifest entry.", "prefix is not pkg:, path: or list:", EntryDigestJSON{"tool:build-tool", d}),
		r("refused-no-prefix", "An entry with no prefix.", "prefix is not pkg:, path: or list:", EntryDigestJSON{"code/x", d}),
		r("refused-dotdot", "A path entry that leaves the repository.", "malformed path", EntryDigestJSON{"path:../etc/passwd", d}),
		r("refused-duplicate", "The same entry twice.", "appears twice", EntryDigestJSON{"path:a", d}, EntryDigestJSON{"path:a", d}),
		r("refused-path-root", "path:. is not a path (only list:. names the root).", "malformed path", EntryDigestJSON{"path:.", d}),
	}
}
