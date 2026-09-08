package hasher

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/globmatch"
)

// languageSourceInputRegistryDocument mirrors the closed neutral v1 registry.
// The fields retain descriptive ownership metadata as well as selectors so the
// native equality test proves the whole reviewed document was adopted.
type languageSourceInputRegistryDocument struct {
	SchemaVersion   int                            `json:"schema_version"`
	UniversalInputs universalSourceInputs          `json:"universal_inputs"`
	Languages       []languageSourceInputSelectors `json:"languages"`
}

type universalSourceInputs struct {
	BuildFilenames               []string `json:"build_filenames"`
	GeneratedDirectoryComponents []string `json:"generated_directory_components"`
	RootExactBasenames           []string `json:"root_exact_basenames"`
}

type languageSourceInputSelectors struct {
	Language                string                    `json:"language"`
	RecursiveSuffixes       []string                  `json:"recursive_suffixes"`
	RecursiveExactBasenames []string                  `json:"recursive_exact_basenames"`
	RootExactBasenames      []string                  `json:"root_exact_basenames"`
	RootVariableSuffixes    []string                  `json:"root_variable_suffixes"`
	RootExactRelativePaths  []string                  `json:"root_exact_relative_paths"`
	PackageExactInputs      []packageExactSourceInput `json:"package_exact_inputs"`
	CaseAliasGroups         [][]string                `json:"case_alias_groups"`
	ScopedInputs            []scopedSourceInput       `json:"scoped_inputs"`
}

type packageExactSourceInput struct {
	ID          string   `json:"id"`
	PackageRoot string   `json:"package_root"`
	Paths       []string `json:"paths"`
	Reason      string   `json:"reason"`
	Owner       string   `json:"owner"`
}

type scopedSourceInput struct {
	ID             string   `json:"id"`
	Role           string   `json:"role"`
	Decision       string   `json:"decision"`
	Scope          string   `json:"scope"`
	PathPrefix     string   `json:"path_prefix,omitempty"`
	Suffixes       []string `json:"suffixes"`
	ExactBasenames []string `json:"exact_basenames"`
	Reason         string   `json:"reason"`
	Owner          string   `json:"owner"`
}

var languageSourceInputRegistry = mustDecodeLanguageSourceInputRegistry()

var languageSourceInputRegistryByName = indexLanguageSourceInputRegistry(languageSourceInputRegistry)

func mustDecodeLanguageSourceInputRegistry() languageSourceInputRegistryDocument {
	registry, err := decodeLanguageSourceInputRegistry([]byte(languageSourceInputRegistryJSON))
	if err != nil {
		panic("generated language source-input registry is invalid")
	}
	return registry
}

func decodeLanguageSourceInputRegistry(data []byte) (languageSourceInputRegistryDocument, error) {
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.DisallowUnknownFields()
	var registry languageSourceInputRegistryDocument
	if err := decoder.Decode(&registry); err != nil {
		return languageSourceInputRegistryDocument{}, fmt.Errorf("invalid language source-input registry")
	}
	var trailing any
	if err := decoder.Decode(&trailing); err != io.EOF {
		return languageSourceInputRegistryDocument{}, fmt.Errorf("invalid language source-input registry")
	}
	if err := validateLanguageSourceInputRegistry(registry); err != nil {
		return languageSourceInputRegistryDocument{}, err
	}
	return registry, nil
}

func validateLanguageSourceInputRegistry(registry languageSourceInputRegistryDocument) error {
	if registry.SchemaVersion != 1 || len(registry.Languages) < 23 || len(registry.Languages) > 32 {
		return fmt.Errorf("invalid language source-input registry")
	}
	selectorCount := len(registry.UniversalInputs.BuildFilenames) +
		len(registry.UniversalInputs.GeneratedDirectoryComponents) +
		len(registry.UniversalInputs.RootExactBasenames)
	seen := make(map[string]bool, len(registry.Languages))
	for _, language := range registry.Languages {
		if language.Language == "" || seen[language.Language] ||
			len(language.RecursiveSuffixes) > 256 ||
			len(language.RecursiveExactBasenames) > 256 ||
			len(language.RootExactBasenames) > 256 ||
			len(language.RootVariableSuffixes) > 256 ||
			len(language.RootExactRelativePaths) > 256 ||
			len(language.PackageExactInputs) > 64 ||
			len(language.CaseAliasGroups) > 32 ||
			len(language.ScopedInputs) > 64 {
			return fmt.Errorf("invalid language source-input registry")
		}
		seen[language.Language] = true
		selectorCount += len(language.RecursiveSuffixes) + len(language.RecursiveExactBasenames) +
			len(language.RootExactBasenames) + len(language.RootVariableSuffixes) +
			len(language.RootExactRelativePaths)
		for _, group := range language.CaseAliasGroups {
			if len(group) < 2 || len(group) > 8 {
				return fmt.Errorf("invalid language source-input registry")
			}
			selectorCount += len(group)
		}
		for _, exact := range language.PackageExactInputs {
			if exact.ID == "" || exact.PackageRoot == "" || exact.Owner == "" || exact.Reason == "" || len(exact.Paths) == 0 || len(exact.Paths) > 256 {
				return fmt.Errorf("invalid language source-input registry")
			}
			selectorCount += len(exact.Paths)
		}
		for _, scoped := range language.ScopedInputs {
			if scoped.ID == "" || scoped.Owner == "" || scoped.Reason == "" || scoped.Decision != "include" ||
				(scoped.Role != "native_companion" && scoped.Role != "resource") ||
				(scoped.Scope != "root" && scoped.Scope != "subtree") ||
				(scoped.Scope == "root" && scoped.PathPrefix != "") ||
				(scoped.Scope == "subtree" && scoped.PathPrefix == "") ||
				len(scoped.Suffixes) > 256 || len(scoped.ExactBasenames) > 256 ||
				len(scoped.Suffixes)+len(scoped.ExactBasenames) == 0 {
				return fmt.Errorf("invalid language source-input registry")
			}
			selectorCount += len(scoped.Suffixes) + len(scoped.ExactBasenames)
		}
	}
	if selectorCount > 4096 {
		return fmt.Errorf("invalid language source-input registry")
	}
	return nil
}

func indexLanguageSourceInputRegistry(registry languageSourceInputRegistryDocument) map[string]languageSourceInputSelectors {
	indexed := make(map[string]languageSourceInputSelectors, len(registry.Languages))
	for _, language := range registry.Languages {
		indexed[language.Language] = language
	}
	return indexed
}

func generatedDirectoryComponents() []string {
	return append([]string(nil), languageSourceInputRegistry.UniversalInputs.GeneratedDirectoryComponents...)
}

func matchesSourceInput(
	language languageSourceInputSelectors,
	packageRoot string,
	relativePath string,
	basename string,
	isRoot bool,
	declaredMode bool,
	declaredPatterns []string,
) bool {
	universal := languageSourceInputRegistry.UniversalInputs
	if containsString(universal.BuildFilenames, basename) ||
		(isRoot && containsString(universal.RootExactBasenames, basename)) ||
		(isRoot && containsString(language.RootExactBasenames, basename)) ||
		(isRoot && hasAnySuffix(basename, language.RootVariableSuffixes)) ||
		containsString(language.RootExactRelativePaths, relativePath) ||
		matchesPackageExactInput(language.PackageExactInputs, packageRoot, relativePath) {
		return true
	}
	if declaredMode {
		for _, pattern := range declaredPatterns {
			if globMatchPath(pattern, relativePath) {
				return true
			}
		}
		return false
	}
	return containsString(language.RecursiveExactBasenames, basename) ||
		hasAnySuffix(basename, language.RecursiveSuffixes) ||
		matchesScopedInput(language.ScopedInputs, relativePath, basename)
}

func matchesPackageExactInput(rules []packageExactSourceInput, packageRoot string, relativePath string) bool {
	for _, rule := range rules {
		if rule.PackageRoot == packageRoot && containsString(rule.Paths, relativePath) {
			return true
		}
	}
	return false
}

func matchesScopedInput(rules []scopedSourceInput, relativePath string, basename string) bool {
	for _, rule := range rules {
		inScope := rule.Scope == "root" && !strings.Contains(relativePath, "/")
		if rule.Scope == "subtree" {
			inScope = strings.HasPrefix(relativePath, rule.PathPrefix+"/")
		}
		if inScope && (containsString(rule.ExactBasenames, basename) || hasAnySuffix(basename, rule.Suffixes)) {
			return true
		}
	}
	return false
}

func hasAnySuffix(value string, suffixes []string) bool {
	for _, suffix := range suffixes {
		if strings.HasSuffix(value, suffix) {
			return true
		}
	}
	return false
}

func containsString(values []string, target string) bool {
	for _, value := range values {
		if value == target {
			return true
		}
	}
	return false
}

func globMatchPath(pattern string, relativePath string) bool {
	return globmatch.MatchPath(pattern, relativePath)
}
