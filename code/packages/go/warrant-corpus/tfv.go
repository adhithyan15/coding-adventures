package warrantcorpus

import (
	"regexp"
	"strconv"
	"strings"
)

// # A minimal TFV v1 reader (TF01 section 2) for cases.tfv
//
// TFV is the repository's language-neutral line format. This reader is written
// from the format's rules, not copied from another lane's reader, so a bug in
// one cannot hide a bug in the other. It accepts only the value kinds the
// Warrant corpus uses and rejects the rest.
//
// A record is one line:
//
//	<id> R<count> K<name> <value> K<name> <value> ...
//
// Values are space-separated tokens:
//
//	N            null            T / F           booleans
//	I<decimal>   integer         X<hex>          bytes ("X" alone is empty)
//	S<hex>       UTF-8 text      L<n> v1 .. vn   list
//	M<n> k1 v1 .. kn vn          map
//
// Because lists and maps announce their length, the parser is a small recursive
// descent; it never has to guess where a value ends.
const (
	tfvHeader   = "tfv 1"
	tfvMaxDepth = 64
)

var (
	tfvIDRE    = regexp.MustCompile(`^[A-Za-z0-9._/@#-]+$`)
	tfvFieldRE = regexp.MustCompile(`^[a-z][a-z0-9_]*$`)
	tfvIntRE   = regexp.MustCompile(`^-?(?:0|[1-9][0-9]*)$`)
	tfvCountRE = regexp.MustCompile(`^(?:0|[1-9][0-9]*)$`)
)

// TfvRecord is one record: its id and its fields, in the order written.
type TfvRecord struct {
	ID     string
	Fields map[string]TfvValue
}

func parseTfvValue(tokens []string, start, depth int) (TfvValue, int) {
	need(start < len(tokens), "tfv", "record ends inside a value")
	need(depth <= tfvMaxDepth, "tfv", "value nests too deeply")
	token := tokens[start]
	need(token != "", "tfv", "empty token in a TFV record")
	tag, body := token[:1], token[1:]
	switch {
	case tag == "N" && body == "":
		return TfvValue{Kind: "null"}, start + 1
	case tag == "T" && body == "":
		return TfvValue{Kind: "bool", Bool: true}, start + 1
	case tag == "F" && body == "":
		return TfvValue{Kind: "bool"}, start + 1
	case tag == "I":
		need(tfvIntRE.MatchString(body) && body != "-0", "tfv", "bad integer")
		return TfvValue{Kind: "int", Text: body}, start + 1
	case tag == "X":
		need(hexRE.MatchString(body), "tfv", "bad bytes")
		return TfvValue{Kind: "bytes", Bytes: fromHex(body)}, start + 1
	case tag == "S":
		need(hexRE.MatchString(body), "tfv", "bad text")
		return TfvValue{Kind: "text", Text: decodeUTF8(fromHex(body), "tfv")}, start + 1
	case tag == "L" || tag == "M":
		need(tfvCountRE.MatchString(body), "tfv", "bad container length")
		count, err := strconv.Atoi(body)
		need(err == nil && count <= len(tokens), "tfv", "container length exceeds the record")
		at := start + 1
		if tag == "L" {
			items := make([]TfvValue, 0, count)
			for i := 0; i < count; i++ {
				var item TfvValue
				item, at = parseTfvValue(tokens, at, depth+1)
				items = append(items, item)
			}
			return TfvValue{Kind: "list", List: items}, at
		}
		pairs := make([][2]TfvValue, 0, count)
		for i := 0; i < count; i++ {
			var key, item TfvValue
			key, at = parseTfvValue(tokens, at, depth+1)
			item, at = parseTfvValue(tokens, at, depth+1)
			pairs = append(pairs, [2]TfvValue{key, item})
		}
		return TfvValue{Kind: "map", Pairs: pairs}, at
	}
	fail("tfv", "unsupported TFV value kind")
	return TfvValue{}, 0
}

// parseTFV parses a whole cases.tfv into records, in file order.
func parseTFV(text string) []TfvRecord {
	lines := strings.Split(text, "\n")
	need(lines[0] == tfvHeader, "tfv", "missing `tfv 1` header")
	need(lines[len(lines)-1] == "", "tfv", "TFV must end with a newline")
	records := []TfvRecord{}
	seen := map[string]bool{}
	for _, line := range lines[1 : len(lines)-1] {
		need(line != "" && !strings.Contains(line, "\r"), "tfv", "blank or CR line in TFV")
		tokens := strings.Split(line, " ")
		need(len(tokens) >= 2, "tfv", "record is too short")
		recordID := tokens[0]
		need(tfvIDRE.MatchString(recordID), "tfv", "bad record id")
		need(!seen[recordID], "tfv", "duplicate record id")
		seen[recordID] = true
		countToken := tokens[1]
		need(strings.HasPrefix(countToken, "R") && tfvCountRE.MatchString(countToken[1:]), "tfv", "bad field count")
		count, err := strconv.Atoi(countToken[1:])
		need(err == nil, "tfv", "bad field count")
		fields := map[string]TfvValue{}
		at := 2
		for i := 0; i < count; i++ {
			need(at < len(tokens) && strings.HasPrefix(tokens[at], "K"), "tfv", "expected a field key")
			name := tokens[at][1:]
			need(tfvFieldRE.MatchString(name), "tfv", "bad field name")
			_, duplicate := fields[name]
			need(!duplicate, "tfv", "duplicate field")
			var value TfvValue
			value, at = parseTfvValue(tokens, at+1, 0)
			fields[name] = value
		}
		need(at == len(tokens), "tfv", "trailing tokens after the last field")
		records = append(records, TfvRecord{ID: recordID, Fields: fields})
	}
	return records
}
