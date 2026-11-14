package legacy

// What `kiln record prepare` needs from the key code (KILN04 section 4.8,
// stage 4b-1).
//
// The recorder keys every package it runs, and it must key them with exactly
// the facts the merge job will later be handed. Three small pieces live here,
// beside the code that consumes them, so the recorder and the key code cannot
// drift apart:
//
//	RecordExecutorArgs   the flags the build tool runs with (keyed as
//	                     `kiln:executor-args`), as a constant, not a guess
//	RecordEnviron        which variables of the process environment become the
//	                     host facts' `env` (the input of ScrubbedEnv)
//	Traced.RecordEnv     the environment and tools of a package that still RUNS
//	                     though its key was refused
//	Traced.HostRoots     the parsed host-roots.json, from which the sandbox's
//	                     read-only roots are made

import (
	"fmt"
	"sort"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/envscrub"
	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/key"
)

// RecordExecutorArgs are the build tool's flags during a recording run, in the
// spelling CanonicalExecutorArgs requires (`-name=value`). The build tool
// refuses `-exec-wrapper` together with either being on, and both default to
// off, so a recording run passes neither; the weak key still names them, so a
// lookup under another executor configuration can never hit a manifest
// recorded under this one.
var RecordExecutorArgs = []string{"-clippy=false", "-prune-leaf-targets=false"}

// RecordEnviron selects the process-environment entries that the host facts
// carry (KILN04 section 7.1, G4) and from which every package's scrubbed
// environment is derived:
//
//  1. every variable env.json allows (EnvPolicy.EnvFor, which refuses a
//     secret-like name or value instead of skipping it);
//  2. HOME, when set;
//  3. TMPDIR, always, as tmpdir: the recorder fixes it (RecordTmpdir), because
//     hosted runners do not set one and the value is keyed.
//
// PATH is never carried: it is constructed from each package's tools
// (ConstructPATH), so an inherited one could only be wrong. The result is
// sorted by name and has no repeated name; feeding it back to ScrubbedEnv gives
// the same variables the traced run gets.
func RecordEnviron(src Source, environ []string, tmpdir string) ([]string, error) {
	data, err := readSmall(src, EnvPath, maxPolicyBytes)
	if err != nil {
		return nil, err
	}
	pol, err := ParseEnvPolicy(data)
	if err != nil {
		return nil, fmt.Errorf("%s: %w", EnvPath, err)
	}
	vars, err := pol.EnvFor(environ)
	if err != nil {
		return nil, err
	}
	byName := map[string]string{}
	for _, v := range vars {
		byName[v.Name] = v.Value
	}
	// HOME is not an env.json name, so it is taken from environ directly. The
	// LAST entry wins, as in os/exec.
	for _, e := range environ {
		if name := envscrub.EntryName(e); foldEnv(name) == foldEnv("HOME") {
			byName["HOME"] = strings.TrimPrefix(e[len(name):], "=")
		}
	}
	byName["TMPDIR"] = tmpdir
	delete(byName, "PATH") // env.json cannot smuggle one in either
	out := make([]string, 0, len(byName))
	for n, v := range byName {
		out = append(out, n+"="+v)
	}
	sort.Strings(out)
	return out, nil
}

// HostRoots is the parsed host-roots.json of the tree this Traced was built on.
func (t *Traced) HostRoots() *HostRoots { return t.pl.pol.roots }

// RecordEnv is the scrubbed environment and the tools of the package at rel,
// whether or not its weak key exists. A package whose key was refused
// (`unknown-toolchain:<x>`, `no-build-file:linux`, ...) still runs under the
// build tool, so the sandbox needs an environment for it; it gets the one its
// KNOWN tools build, and its record will say it was refused. An unknown
// package is an error.
func (t *Traced) RecordEnv(rel string) ([]key.EnvVar, []key.Tool, error) {
	p, ok := t.pkgAt(rel)
	if !ok {
		return nil, nil, fmt.Errorf("traced key: no package at %q", short(rel))
	}
	pl := t.pl
	tools, _ := pl.pol.tc.ToolsFor(pl.closureNames(pl.closure(p)), pl.opt.OS, pl.opt.Host.ImageID())
	env, err := ScrubbedEnv(pl.pol.env, pl.pol.tc, tools, pl.opt.Environ)
	if err != nil {
		return nil, nil, err
	}
	return env, tools, nil
}
