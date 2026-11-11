package validator

// Package-local metadata must stay data
// =====================================
//
// Diff selection treats a few package-root files as package-local metadata
// (internal/pkgmeta): changing one rebuilds the package itself but none of its
// dependents. That is only safe while those files are DATA. If a BUILD ran
// `sh evidence/run.sh`, or a crate compiled `#[path = "evidence/x.rs"]`,
// editing the file would change what gets executed while the build tool kept
// the change local, and a broken dependent could merge.
//
// pkgmeta already closes the list by type (CHANGELOG.d/ takes only direct
// *.md shards, evidence/ only .json, .cnf and .drat), so a script dropped
// there is ordinary source. This validation is a best-effort tripwire for the
// other half: it fails when a tracked file visibly USES a metadata-class
// location as code, whatever that location's extension. It runs with
// -validate-build-files, i.e. in the detect job of every pull request, so a
// new violation fails the PR that introduces it no matter which packages that
// PR selects.
//
// It is NOT a security boundary. It matches literal paths in the forms in the
// table below, so indirection is not seen: a shell variable, a pipe or
// redirect, a wrapper (bash -c, env, timeout, make -f, go run), include! of
// concat!/env!, #[cfg_attr(..., path = ...)], a build.rs that copies the file,
// package.json scripts, Makefiles and non-Rust sources. Wiring code to a
// metadata file is itself a source change that propagates and is reviewed;
// the tripwire only catches the honest mistake early.
//
// A metadata-class path has a component `CHANGELOG.d` or `evidence` with
// something below it, or a basename CHANGELOG.md, required_capabilities.json,
// effect_certificate.json or assurance_report.json. That is deliberately
// broader than pkgmeta's root-only list: the guard should fail early.
//
//	file                      flagged reference
//	────────────────────────  ─────────────────────────────────────────────
//	BUILD, BUILD_*, *.sh,     the command word of a command, or the first
//	*.bash, *.ps1, *.cmd,     operand of an interpreter in command position
//	*.bat                     (sh, bash, source, ., python, node, pwsh, call…)
//	Cargo.toml                any `path = "…"` or `build = "…"` value
//	*.rs                      #[path = "…"] and include!("…")
//
// Reading metadata as data (include_str!, `--check assurance_report.json`,
// `WARRANT_EVIDENCE_OUT=evidence/test-manifest.json cargo test`) is allowed.

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
)

var (
	metadataExecBasenames = map[string]bool{
		"CHANGELOG.md": true, "required_capabilities.json": true,
		"effect_certificate.json": true, "assurance_report.json": true,
	}
	metadataExecDirectories = map[string]bool{"CHANGELOG.d": true, "evidence": true}

	// Words after which the next operand is executed, sourced or interpreted
	// when they stand in command position.
	metadataExecInterpreters = map[string]bool{
		"sh": true, "bash": true, "zsh": true, "dash": true, "source": true,
		".": true, "&": true, "call": true, "exec": true, "python": true,
		"python3": true, "py": true, "node": true, "deno": true, "bun": true,
		"tsx": true, "ruby": true, "perl": true, "lua": true, "php": true,
		"pwsh": true, "powershell": true, "cmd": true, "elixir": true,
		"escript": true, "runghc": true, "julia": true,
	}

	metadataExecAssignment  = regexp.MustCompile(`^[A-Za-z_][A-Za-z0-9_]*=`)
	metadataExecSeparators  = regexp.MustCompile(`&&|\|\||[;|()]`)
	metadataExecCargoPath   = regexp.MustCompile(`\b(?:path|build)\s*=\s*["']([^"']+)["']`)
	metadataExecRustPath    = regexp.MustCompile(`#\s*!?\s*\[\s*path\s*=\s*["']([^"']+)["']`)
	metadataExecRustInclude = regexp.MustCompile(`(?:^|[^A-Za-z0-9_])include!\s*\(\s*(?:concat!\s*\([^)]*?)?["']([^"']+)["']`)
)

// IsMetadataClassPath reports whether a referenced path lies in a
// metadata-class location, in any position and with either separator.
//
//	evidence/run.sh, ./CHANGELOG.d/x.md, ..\x\CHANGELOG.md  → true
//	src/lib.rs, evidence, CHANGELOG.d, evidence.bindings    → false
func IsMetadataClassPath(reference string) bool {
	cleaned := strings.Trim(strings.TrimSpace(reference), `"'`+"`")
	cleaned = strings.ReplaceAll(cleaned, `\`, "/")
	var parts []string
	for _, part := range strings.Split(cleaned, "/") {
		if part != "" && part != "." {
			parts = append(parts, part)
		}
	}
	if len(parts) == 0 {
		return false
	}
	if metadataExecBasenames[parts[len(parts)-1]] {
		return true
	}
	for _, part := range parts[:len(parts)-1] {
		if metadataExecDirectories[part] {
			return true
		}
	}
	return false
}

// metadataCommandReferences scans shell-like text (BUILD fronts and scripts).
func metadataCommandReferences(text string) []string {
	var found []string
	for _, raw := range strings.Split(text, "\n") {
		line := raw
		if strings.HasPrefix(strings.TrimSpace(line), "#") {
			continue
		}
		if index := strings.Index(line, " #"); index >= 0 {
			line = line[:index]
		}
		for _, segment := range metadataExecSeparators.Split(line, -1) {
			var words []string
			for _, word := range strings.Fields(segment) {
				words = append(words, strings.Trim(word, `"'`+"`"))
			}
			for len(words) > 0 && metadataExecAssignment.MatchString(words[0]) {
				words = words[1:]
			}
			if len(words) == 0 {
				continue
			}
			switch strings.ToLower(words[0]) {
			case "set", "rem", "echo", "printf", "::":
				continue
			}
			// The command word itself is executed ...
			if IsMetadataClassPath(words[0]) {
				found = append(found, words[0])
			}
			// ... and so is the first operand of an interpreter in command
			// position. `--dir .` is an argument, not the `.` builtin.
			command := strings.TrimSuffix(strings.ToLower(path.Base(strings.ReplaceAll(words[0], `\`, "/"))), ".exe")
			if !metadataExecInterpreters[command] && words[0] != "." && words[0] != "&" {
				continue
			}
			for _, operand := range words[1:] {
				if strings.HasPrefix(operand, "-") || strings.HasPrefix(operand, "/") {
					continue
				}
				if IsMetadataClassPath(operand) {
					found = append(found, operand)
				}
				break
			}
		}
	}
	return found
}

func metadataPatternReferences(text string, patterns ...*regexp.Regexp) []string {
	var found []string
	for _, line := range strings.Split(text, "\n") {
		trimmed := strings.TrimSpace(line)
		if strings.HasPrefix(trimmed, "#") && !strings.HasPrefix(trimmed, "#[") && !strings.HasPrefix(trimmed, "#!") {
			continue // a TOML comment
		}
		if strings.HasPrefix(trimmed, "//") {
			continue
		}
		for _, pattern := range patterns {
			for _, match := range pattern.FindAllStringSubmatch(line, -1) {
				if IsMetadataClassPath(match[1]) {
					found = append(found, match[1])
				}
			}
		}
	}
	return found
}

// MetadataExecutionReferences returns the metadata-class paths that the
// tracked file at relative path name (slash-separated) uses as code.
func MetadataExecutionReferences(name, text string) []string {
	base := path.Base(name)
	switch {
	case base == "BUILD" || strings.HasPrefix(base, "BUILD_"),
		strings.HasSuffix(base, ".sh"), strings.HasSuffix(base, ".bash"),
		strings.HasSuffix(base, ".ps1"), strings.HasSuffix(base, ".cmd"),
		strings.HasSuffix(base, ".bat"):
		return metadataCommandReferences(text)
	case base == "Cargo.toml":
		return metadataPatternReferences(text, metadataExecCargoPath)
	case strings.HasSuffix(base, ".rs"):
		if !strings.Contains(text, "path") && !strings.Contains(text, "include!") {
			return nil
		}
		return metadataPatternReferences(text, metadataExecRustPath, metadataExecRustInclude)
	}
	return nil
}

// ValidateNoExecutedMetadata fails when any tracked file under code/ uses a
// metadata-class location as code (see the table at the top of this file).
// It is a best-effort tripwire, not a security boundary (see above).
func ValidateNoExecutedMetadata(repoRoot string) error {
	cmd := exec.Command("git", "-C", repoRoot, "ls-files", "-z", "--", "code")
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	output, err := cmd.Output()
	if err != nil {
		return fmt.Errorf("listing tracked files for metadata-execution validation: %w: %s", err, strings.TrimSpace(stderr.String()))
	}
	var findings []string
	for _, record := range bytes.Split(output, []byte{0}) {
		name := string(record)
		if name == "" || !metadataExecCandidate(name) {
			continue
		}
		data, readErr := os.ReadFile(filepath.Join(repoRoot, filepath.FromSlash(name)))
		if readErr != nil {
			continue // deleted in the working tree, or not a regular file
		}
		for _, reference := range MetadataExecutionReferences(name, string(data)) {
			findings = append(findings, fmt.Sprintf("%s: executes %q", name, reference))
		}
	}
	if len(findings) == 0 {
		return nil
	}
	sort.Strings(findings)
	return fmt.Errorf(
		"package-local metadata validation failed: these files use a CHANGELOG.d/, evidence/ or manifest path as code:\n  - %s\n"+
			"Diff selection does not propagate changes to those locations to dependents "+
			"(code/specs/12-build-system.md, \"Package-local metadata\"); move the code elsewhere.",
		strings.Join(findings, "\n  - "),
	)
}

// metadataExecCandidate reports whether a tracked file is of a kind the
// validation scans.
func metadataExecCandidate(name string) bool {
	base := path.Base(name)
	for _, suffix := range []string{".rs", ".sh", ".bash", ".ps1", ".cmd", ".bat"} {
		if strings.HasSuffix(base, suffix) {
			return true
		}
	}
	return base == "BUILD" || strings.HasPrefix(base, "BUILD_") || base == "Cargo.toml"
}
