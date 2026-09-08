package hasher

import (
	"bytes"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"sort"
	"strings"
	"unicode"
	"unicode/utf8"

	"github.com/adhithyan15/coding-adventures/code/programs/go/build-tool/internal/globmatch"
)

const (
	maxLanguageSourceInputRegistryBytes = 1 << 20
	maxLanguageSourceInputTextBytes     = 4096
)

var windowsReservedBasenames = map[string]bool{
	"aux": true, "con": true, "nul": true, "prn": true,
	"com1": true, "com2": true, "com3": true, "com4": true, "com5": true, "com6": true, "com7": true, "com8": true, "com9": true,
	"lpt1": true, "lpt2": true, "lpt3": true, "lpt4": true, "lpt5": true, "lpt6": true, "lpt7": true, "lpt8": true, "lpt9": true,
}

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
	if len(data) == 0 || len(data) > maxLanguageSourceInputRegistryBytes {
		return languageSourceInputRegistryDocument{}, fmt.Errorf("invalid language source-input registry")
	}
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
	if digest, err := languageSourceInputRegistryDigestForJSON(data); err != nil || digest != languageSourceInputRegistryDigest {
		return languageSourceInputRegistryDocument{}, fmt.Errorf("invalid language source-input registry")
	}
	return registry, nil
}

func validateLanguageSourceInputRegistry(registry languageSourceInputRegistryDocument) error {
	if registry.SchemaVersion != 1 || len(registry.Languages) != 23 || len(registry.Languages) > 32 ||
		len(registry.UniversalInputs.BuildFilenames) > 256 ||
		len(registry.UniversalInputs.GeneratedDirectoryComponents) > 256 ||
		len(registry.UniversalInputs.RootExactBasenames) > 256 {
		return fmt.Errorf("invalid language source-input registry")
	}
	for _, values := range [][]string{
		registry.UniversalInputs.BuildFilenames,
		registry.UniversalInputs.GeneratedDirectoryComponents,
		registry.UniversalInputs.RootExactBasenames,
	} {
		if err := validateCanonicalRegistrySelectors(values, "basename"); err != nil {
			return err
		}
	}
	selectorCount := len(registry.UniversalInputs.BuildFilenames) +
		len(registry.UniversalInputs.GeneratedDirectoryComponents) +
		len(registry.UniversalInputs.RootExactBasenames)
	seen := make(map[string]bool, len(registry.Languages))
	previousLanguage := ""
	for _, language := range registry.Languages {
		if !isRegistryLanguage(language.Language) || seen[language.Language] || language.Language <= previousLanguage ||
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
		previousLanguage = language.Language
		aliasGroups, err := validateRegistryAliasGroups(language.CaseAliasGroups)
		if err != nil {
			return err
		}
		roleByIdentity := make(map[string]string)
		valuesByIdentity := make(map[string]map[string]bool)
		for role, values := range map[string][]string{
			"recursive_suffixes":        language.RecursiveSuffixes,
			"recursive_exact_basenames": language.RecursiveExactBasenames,
			"root_exact_basenames":      language.RootExactBasenames,
			"root_variable_suffixes":    language.RootVariableSuffixes,
			"root_exact_relative_paths": language.RootExactRelativePaths,
		} {
			kind := "basename"
			if strings.Contains(role, "suffixes") {
				kind = "suffix"
			} else if role == "root_exact_relative_paths" {
				kind = "path"
			}
			if err := validateCanonicalRegistrySelectors(values, kind); err != nil {
				return err
			}
			for _, value := range values {
				identity := strings.ToLower(value)
				if prior, ok := roleByIdentity[identity]; ok && prior != role {
					return fmt.Errorf("invalid language source-input registry")
				}
				roleByIdentity[identity] = role
				if valuesByIdentity[identity] == nil {
					valuesByIdentity[identity] = make(map[string]bool)
				}
				valuesByIdentity[identity][value] = true
			}
		}
		for identity, aliases := range aliasGroups {
			values := valuesByIdentity[identity]
			if len(values) != len(aliases) {
				return fmt.Errorf("invalid language source-input registry")
			}
			for alias := range aliases {
				if !values[alias] {
					return fmt.Errorf("invalid language source-input registry")
				}
			}
		}
		for identity, values := range valuesByIdentity {
			if len(values) > 1 && aliasGroups[identity] == nil {
				return fmt.Errorf("invalid language source-input registry")
			}
		}
		selectorCount += len(language.RecursiveSuffixes) + len(language.RecursiveExactBasenames) +
			len(language.RootExactBasenames) + len(language.RootVariableSuffixes) +
			len(language.RootExactRelativePaths)
		for _, group := range language.CaseAliasGroups {
			selectorCount += len(group)
		}
		previousPackageID := ""
		seenPackageRoots := make(map[string]bool)
		for _, exact := range language.PackageExactInputs {
			rootParts := strings.Split(exact.PackageRoot, "/")
			rootIdentity := strings.ToLower(exact.PackageRoot)
			if !isRegistryIdentifier(exact.ID) || exact.ID <= previousPackageID || !isRegistryDescription(exact.Owner) || !isRegistryDescription(exact.Reason) ||
				len(exact.Paths) == 0 || len(exact.Paths) > 256 || validateRegistryPath(exact.PackageRoot) != nil ||
				len(rootParts) < 4 || rootParts[0] != "code" || (rootParts[1] != "packages" && rootParts[1] != "programs") ||
				rootParts[2] != language.Language || seenPackageRoots[rootIdentity] || validateCanonicalRegistrySelectors(exact.Paths, "path") != nil {
				return fmt.Errorf("invalid language source-input registry")
			}
			previousPackageID = exact.ID
			seenPackageRoots[rootIdentity] = true
			for _, path := range exact.Paths {
				if registryPathEntersGeneratedComponent(path, registry.UniversalInputs.GeneratedDirectoryComponents) {
					return fmt.Errorf("invalid language source-input registry")
				}
			}
			selectorCount += len(exact.Paths)
		}
		previousScopedID := ""
		for _, scoped := range language.ScopedInputs {
			if !isRegistryIdentifier(scoped.ID) || scoped.ID <= previousScopedID || !isRegistryDescription(scoped.Owner) || !isRegistryDescription(scoped.Reason) || scoped.Decision != "include" ||
				(scoped.Role != "native_companion" && scoped.Role != "resource") ||
				(scoped.Scope != "root" && scoped.Scope != "subtree") ||
				(scoped.Scope == "root" && scoped.PathPrefix != "") ||
				(scoped.Scope == "subtree" && scoped.PathPrefix == "") ||
				len(scoped.Suffixes) > 256 || len(scoped.ExactBasenames) > 256 ||
				validateCanonicalRegistrySelectors(scoped.Suffixes, "suffix") != nil ||
				validateCanonicalRegistrySelectors(scoped.ExactBasenames, "basename") != nil ||
				len(scoped.Suffixes)+len(scoped.ExactBasenames) == 0 {
				return fmt.Errorf("invalid language source-input registry")
			}
			if scoped.PathPrefix != "" && (validateRegistryPath(scoped.PathPrefix) != nil || registryPathEntersGeneratedComponent(scoped.PathPrefix, registry.UniversalInputs.GeneratedDirectoryComponents)) {
				return fmt.Errorf("invalid language source-input registry")
			}
			previousScopedID = scoped.ID
			selectorCount += len(scoped.Suffixes) + len(scoped.ExactBasenames)
		}
	}
	if selectorCount > 4096 {
		return fmt.Errorf("invalid language source-input registry")
	}
	return nil
}

func languageSourceInputRegistryDigestForJSON(data []byte) (string, error) {
	var document any
	if err := json.Unmarshal(data, &document); err != nil {
		return "", err
	}
	canonical, err := json.Marshal(document)
	if err != nil {
		return "", err
	}
	domain := []byte("coding-adventures/build-tool-language-source-input-registry/v1\x00")
	framed := make([]byte, len(domain)+8+len(canonical))
	copy(framed, domain)
	binary.BigEndian.PutUint64(framed[len(domain):len(domain)+8], uint64(len(canonical)))
	copy(framed[len(domain)+8:], canonical)
	digest := sha256.Sum256(framed)
	return hex.EncodeToString(digest[:]), nil
}

func validateCanonicalRegistrySelectors(values []string, kind string) error {
	if !sort.StringsAreSorted(values) {
		return fmt.Errorf("invalid language source-input registry")
	}
	for index, value := range values {
		if index > 0 && values[index-1] == value {
			return fmt.Errorf("invalid language source-input registry")
		}
		var err error
		switch kind {
		case "suffix":
			err = validateRegistrySuffix(value)
		case "path":
			err = validateRegistryPath(value)
		default:
			err = validateRegistryBasename(value)
		}
		if err != nil {
			return err
		}
	}
	return nil
}

func validateRegistryAliasGroups(groups [][]string) (map[string]map[string]bool, error) {
	result := make(map[string]map[string]bool)
	previous := ""
	for _, group := range groups {
		encoded, _ := json.Marshal(group)
		if len(group) < 2 || len(group) > 8 || string(encoded) <= previous || !sort.StringsAreSorted(group) {
			return nil, fmt.Errorf("invalid language source-input registry")
		}
		previous = string(encoded)
		identity := strings.ToLower(group[0])
		if result[identity] != nil {
			return nil, fmt.Errorf("invalid language source-input registry")
		}
		aliases := make(map[string]bool, len(group))
		for index, value := range group {
			if validateRegistryBasename(value) != nil || strings.ToLower(value) != identity || aliases[value] || (index > 0 && group[index-1] >= value) {
				return nil, fmt.Errorf("invalid language source-input registry")
			}
			aliases[value] = true
		}
		result[identity] = aliases
	}
	return result, nil
}

func validateRegistryText(value string) error {
	if value == "" || len(value) > maxLanguageSourceInputTextBytes || !utf8.ValidString(value) {
		return fmt.Errorf("invalid language source-input registry")
	}
	for _, character := range value {
		if character > unicode.MaxASCII || unicode.IsControl(character) || unicode.In(character, unicode.Cf, unicode.Co, unicode.Cs) {
			return fmt.Errorf("invalid language source-input registry")
		}
	}
	return nil
}

func validateRegistrySuffix(value string) error {
	if validateRegistryText(value) != nil || len(value) < 2 || value[0] != '.' {
		return fmt.Errorf("invalid language source-input registry")
	}
	for _, character := range value[1:] {
		if !(character >= 'A' && character <= 'Z') && !(character >= 'a' && character <= 'z') && !(character >= '0' && character <= '9') && !strings.ContainsRune("._+-", character) {
			return fmt.Errorf("invalid language source-input registry")
		}
	}
	return nil
}

func validateRegistryBasename(value string) error {
	if validateRegistryText(value) != nil || strings.ContainsAny(value, "/\\:") || strings.HasSuffix(value, ".") || strings.HasSuffix(value, " ") || value == "." || value == ".." {
		return fmt.Errorf("invalid language source-input registry")
	}
	stem := strings.ToLower(strings.SplitN(value, ".", 2)[0])
	if windowsReservedBasenames[stem] {
		return fmt.Errorf("invalid language source-input registry")
	}
	return nil
}

func validateRegistryPath(value string) error {
	if validateRegistryText(value) != nil || validateRepositoryPath(value) != nil || strings.Contains(value, ":") {
		return fmt.Errorf("invalid language source-input registry")
	}
	for _, component := range strings.Split(value, "/") {
		if validateRegistryBasename(component) != nil {
			return fmt.Errorf("invalid language source-input registry")
		}
	}
	return nil
}

func registryPathEntersGeneratedComponent(path string, generated []string) bool {
	for _, component := range strings.Split(path, "/") {
		if containsString(generated, component) {
			return true
		}
	}
	return false
}

func isRegistryLanguage(value string) bool {
	if validateRegistryText(value) != nil {
		return false
	}
	for _, character := range value {
		if !(character >= 'a' && character <= 'z') && !(character >= '0' && character <= '9') && character != '-' {
			return false
		}
	}
	return true
}

func isRegistryIdentifier(value string) bool {
	return validateRegistryText(value) == nil && !strings.ContainsAny(value, "/\\:")
}

func isRegistryDescription(value string) bool {
	return validateRegistryText(value) == nil
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
