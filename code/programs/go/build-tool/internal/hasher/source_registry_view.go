package hasher

import "path"

// SourceInputRegistryDigest returns the digest of the generated, governed
// source-input registry embedded in this build-tool binary.
func SourceInputRegistryDigest() string {
	return languageSourceInputRegistryDigest
}

// GeneratedDirectoryComponents returns a defensive copy of the exact
// generated-directory set owned by the source-input registry.
func GeneratedDirectoryComponents() []string {
	return generatedDirectoryComponents()
}

// MatchesGovernedSourceEvidence reports whether repoPath is source evidence
// for the named language and direct package root. BUILD files and paths that
// enter generated directories are deliberately not source evidence.
func MatchesGovernedSourceEvidence(language, packageRoot, repoPath string) bool {
	selectors, ok := languageSourceInputRegistryByName[language]
	if !ok || !stringsHasPathPrefix(repoPath, packageRoot) {
		return false
	}
	relativePath := repoPath[len(packageRoot)+1:]
	if relativePath == "" || registryPathEntersGeneratedComponent(
		relativePath,
		languageSourceInputRegistry.UniversalInputs.GeneratedDirectoryComponents,
	) {
		return false
	}
	base := path.Base(relativePath)
	if containsString(languageSourceInputRegistry.UniversalInputs.BuildFilenames, base) {
		return false
	}
	return matchesSourceInput(
		selectors,
		packageRoot,
		relativePath,
		base,
		!stringsContainsSlash(relativePath),
		false,
		nil,
	)
}

func stringsHasPathPrefix(value, prefix string) bool {
	return len(value) > len(prefix) && value[:len(prefix)] == prefix && value[len(prefix)] == '/'
}

func stringsContainsSlash(value string) bool {
	for _, character := range value {
		if character == '/' {
			return true
		}
	}
	return false
}
