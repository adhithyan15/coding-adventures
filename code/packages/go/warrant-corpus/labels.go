package warrantcorpus

import (
	"sort"
	"strconv"
)

// # Honest coverage labels for exhaustive suites (backlog LP-012)
//
// The corpus calls every enumerated suite `kind: "exhaustive"`, and VER04i's
// manifest has no room for a finer word. But "exhaustive" over Bytes[347] is a
// statement about 347 chosen byte strings, not about every byte string, and
// adding such totals together hides that (LP-012: the same conflation inflates
// the Rust figures). So each suite also gets a label DERIVED FROM ITS DOMAIN
// DESCRIPTORS, never typed by hand:
//
//	label         meaning
//	full          the descriptor enumerates EVERY value of its type (bool, u8_all,
//	              Bytes<0>, Bytes<1>)
//	bounded-set   a fixed, deterministic set of boundary and structured values that
//	              is a strict subset of the type (u64_edge, bytes_d1, bytes_n(N) for
//	              N >= 2, nat_0_64)
//	sample        pseudo-random or otherwise unstructured draws. No portable
//	              descriptor produces one yet; the label is reserved so a future
//	              sampled suite cannot be mistaken for a bounded set
//
// A suite is `full` only if EVERY binder is `full`; otherwise it is as weak as its
// weakest binder (sample over bounded-set over full).
//
// The labels live in a sidecar, evidence/domain-labels.json, because the
// warrant-lane-evidence-v1 rows are a closed key set that the Rust validator
// (WD-004a) rejects additions to.
const (
	// LabelsSchema is the schema string of the label sidecar.
	LabelsSchema = "warrant-lane-domain-labels-v1"
	labelsNote   = "Suite-level 'exhaustive' counts conflate full, bounded-set and sample " +
		"domains until backlog LP-012 lands; use cases_by_label, not the " +
		"manifest's exhaustive total, as the headline."
)

var labelRank = map[string]int{"full": 0, "bounded-set": 1, "sample": 2}

// domainLabel is the label of one domain descriptor (see the file comment).
func domainLabel(descriptor string) string {
	switch descriptor {
	case "bool", "u8_all":
		return "full"
	case "u64_edge", "bytes_d1", "nat_0_64":
		return "bounded-set"
	}
	if match := bytesNRE.FindStringSubmatch(descriptor); match != nil {
		// Bytes<0> has one value and Bytes<1> has 256: both enumerate the type.
		if isZeroOrOne(match[1]) {
			return "full"
		}
		return "bounded-set"
	}
	fail("unsupported-domain", "no coverage label for this descriptor")
	return ""
}

// isZeroOrOne reports whether a decimal width such as "00" or "01" is 0 or 1.
func isZeroOrOne(width string) bool {
	n, err := strconv.Atoi(width)
	return err == nil && n <= 1
}

// deriveLabels is the label document for corpus, bound to one manifest by digest.
func deriveLabels(corpus *Corpus, packageName, module, manifestSha256 string) map[string]any {
	byLabel := map[string]any{}
	suites := make([]map[string]any, 0, len(corpus.Suites))
	for _, suite := range corpus.Suites {
		label := "full"
		binders := make([]any, 0, len(suite.Binders))
		for _, binder := range suite.Binders {
			binderLabel := domainLabel(binder.Descriptor)
			if labelRank[binderLabel] > labelRank[label] {
				label = binderLabel
			}
			binders = append(binders, map[string]any{"name": binder.Name, "domain": binder.Descriptor, "label": binderLabel})
		}
		total, _ := byLabel[label].(int)
		byLabel[label] = total + suite.Cases
		suites = append(suites, map[string]any{
			"id": suite.ID, "label": label, "domain": suite.DomainText, "cases": suite.Cases, "binders": binders,
		})
	}
	sort.Slice(suites, func(i, j int) bool { return suites[i]["id"].(string) < suites[j]["id"].(string) })
	return map[string]any{
		"schema":          LabelsSchema,
		"package":         packageName,
		"module":          module,
		"manifest_sha256": manifestSha256,
		"note":            labelsNote,
		"cases_by_label":  byLabel,
		"suites":          suites,
	}
}
