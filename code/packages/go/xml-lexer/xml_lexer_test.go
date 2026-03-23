package xmllexer

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
)

// =============================================================================
// Test Helpers
// =============================================================================
//
// These helpers mirror the Python test's token_pairs and token_types functions.
// They extract (TypeName, Value) pairs from the token list, filtering out EOF.

// tokenPairs tokenizes XML and returns (TypeName, Value) pairs without EOF.
func tokenPairs(t *testing.T, source string) []struct{ TypeName, Value string } {
	t.Helper()
	tokens, err := TokenizeXml(source)
	if err != nil {
		t.Fatalf("Failed to tokenize %q: %v", source, err)
	}
	var result []struct{ TypeName, Value string }
	for _, tok := range tokens {
		if tok.TypeName == "EOF" {
			continue
		}
		result = append(result, struct{ TypeName, Value string }{tok.TypeName, tok.Value})
	}
	return result
}

// tokenTypes tokenizes XML and returns just the type names without EOF.
func tokenTypes(t *testing.T, source string) []string {
	t.Helper()
	pairs := tokenPairs(t, source)
	var types []string
	for _, p := range pairs {
		types = append(types, p.TypeName)
	}
	return types
}

// assertPairs checks that actual pairs match expected (TypeName, Value) pairs.
func assertPairs(t *testing.T, actual []struct{ TypeName, Value string }, expected []struct{ TypeName, Value string }) {
	t.Helper()
	if len(actual) != len(expected) {
		t.Fatalf("Expected %d pairs, got %d.\nExpected: %v\nActual:   %v", len(expected), len(actual), expected, actual)
	}
	for i, exp := range expected {
		if actual[i].TypeName != exp.TypeName || actual[i].Value != exp.Value {
			t.Errorf("Pair %d: expected (%q, %q), got (%q, %q)",
				i, exp.TypeName, exp.Value, actual[i].TypeName, actual[i].Value)
		}
	}
}

// countType counts how many times a TypeName appears in the pairs.
func countType(pairs []struct{ TypeName, Value string }, typeName string) int {
	count := 0
	for _, p := range pairs {
		if p.TypeName == typeName {
			count++
		}
	}
	return count
}

// valuesOfType extracts all values for a given type name.
func valuesOfType(pairs []struct{ TypeName, Value string }, typeName string) []string {
	var vals []string
	for _, p := range pairs {
		if p.TypeName == typeName {
			vals = append(vals, p.Value)
		}
	}
	return vals
}

// hasType checks whether a type name appears in the pairs.
func hasType(pairs []struct{ TypeName, Value string }, typeName string) bool {
	return countType(pairs, typeName) > 0
}

// =============================================================================
// Basic Tags
// =============================================================================

// TestSimpleElement verifies tokenization of a simple element: <p>text</p>.
//
// This is the most fundamental XML construct: an opening tag, text content,
// and a closing tag. The lexer should produce:
//   - OPEN_TAG_START for `<`
//   - TAG_NAME for the element name
//   - TAG_CLOSE for `>`
//   - TEXT for text content between tags
//   - CLOSE_TAG_START for `</`
func TestSimpleElement(t *testing.T) {
	pairs := tokenPairs(t, "<p>text</p>")
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"OPEN_TAG_START", "<"},
		{"TAG_NAME", "p"},
		{"TAG_CLOSE", ">"},
		{"TEXT", "text"},
		{"CLOSE_TAG_START", "</"},
		{"TAG_NAME", "p"},
		{"TAG_CLOSE", ">"},
	})
}

// TestElementWithNamespace verifies tags with XML namespace prefixes: <ns:tag>.
//
// XML namespaces use a colon in the tag name (e.g., "ns:tag"). The TAG_NAME
// regex includes colons in its character class: [a-zA-Z_][a-zA-Z0-9_:.-]*
func TestElementWithNamespace(t *testing.T) {
	pairs := tokenPairs(t, "<ns:tag>content</ns:tag>")
	types := make([]string, len(pairs))
	for i, p := range pairs {
		types[i] = p.TypeName
	}

	expected := []string{
		"OPEN_TAG_START", "TAG_NAME", "TAG_CLOSE",
		"TEXT",
		"CLOSE_TAG_START", "TAG_NAME", "TAG_CLOSE",
	}
	if len(types) != len(expected) {
		t.Fatalf("Expected %d types, got %d: %v", len(expected), len(types), types)
	}
	for i, e := range expected {
		if types[i] != e {
			t.Errorf("Type %d: expected %q, got %q", i, e, types[i])
		}
	}

	// The tag name should include the namespace prefix
	if pairs[1].Value != "ns:tag" {
		t.Errorf("Expected TAG_NAME 'ns:tag', got %q", pairs[1].Value)
	}
}

// TestEmptyElementExplicit verifies an explicitly empty element: <div></div>.
//
// This is different from a self-closing tag (<div/>) -- it has separate
// open and close tags with no content between them.
func TestEmptyElementExplicit(t *testing.T) {
	pairs := tokenPairs(t, "<div></div>")
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"OPEN_TAG_START", "<"},
		{"TAG_NAME", "div"},
		{"TAG_CLOSE", ">"},
		{"CLOSE_TAG_START", "</"},
		{"TAG_NAME", "div"},
		{"TAG_CLOSE", ">"},
	})
}

// TestSelfClosingTag verifies self-closing tags: <br/>.
//
// Self-closing tags use `/>` instead of a separate closing tag. The lexer
// produces SELF_CLOSE instead of TAG_CLOSE.
func TestSelfClosingTag(t *testing.T) {
	pairs := tokenPairs(t, "<br/>")
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"OPEN_TAG_START", "<"},
		{"TAG_NAME", "br"},
		{"SELF_CLOSE", "/>"},
	})
}

// TestSelfClosingWithSpace verifies self-closing with a space: <br />.
//
// The space between the tag name and `/>` is consumed by the skip pattern
// inside the "tag" group.
func TestSelfClosingWithSpace(t *testing.T) {
	pairs := tokenPairs(t, "<br />")
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"OPEN_TAG_START", "<"},
		{"TAG_NAME", "br"},
		{"SELF_CLOSE", "/>"},
	})
}

// =============================================================================
// Attributes
// =============================================================================

// TestDoubleQuotedAttribute verifies attribute with double quotes.
//
// In XML, attribute values can be double-quoted. The grammar defines
// ATTR_VALUE_DQ = /"[^"]*"/ -> ATTR_VALUE, so the token type name
// is aliased to ATTR_VALUE regardless of quote style.
func TestDoubleQuotedAttribute(t *testing.T) {
	pairs := tokenPairs(t, `<div class="main">`)
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"OPEN_TAG_START", "<"},
		{"TAG_NAME", "div"},
		{"TAG_NAME", "class"},
		{"ATTR_EQUALS", "="},
		{"ATTR_VALUE", `"main"`},
		{"TAG_CLOSE", ">"},
	})
}

// TestSingleQuotedAttribute verifies attribute with single quotes.
//
// XML allows both single and double quotes for attribute values. The grammar
// defines ATTR_VALUE_SQ = /'[^']*'/ -> ATTR_VALUE, aliasing to the same
// type as double-quoted values.
func TestSingleQuotedAttribute(t *testing.T) {
	pairs := tokenPairs(t, "<div class='main'>")
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"OPEN_TAG_START", "<"},
		{"TAG_NAME", "div"},
		{"TAG_NAME", "class"},
		{"ATTR_EQUALS", "="},
		{"ATTR_VALUE", "'main'"},
		{"TAG_CLOSE", ">"},
	})
}

// TestMultipleAttributes verifies multiple attributes on one tag.
//
// Each attribute name reuses the TAG_NAME pattern (attribute names follow
// the same lexical rules as tag names in XML).
func TestMultipleAttributes(t *testing.T) {
	pairs := tokenPairs(t, `<a href="url" target="_blank">`)
	tagNames := valuesOfType(pairs, "TAG_NAME")
	if len(tagNames) != 3 || tagNames[0] != "a" || tagNames[1] != "href" || tagNames[2] != "target" {
		t.Errorf("Expected TAG_NAMEs [a, href, target], got %v", tagNames)
	}
	attrValues := valuesOfType(pairs, "ATTR_VALUE")
	if len(attrValues) != 2 || attrValues[0] != `"url"` || attrValues[1] != `"_blank"` {
		t.Errorf("Expected ATTR_VALUEs [\"url\", \"_blank\"], got %v", attrValues)
	}
}

// TestAttributeOnSelfClosing verifies attribute on a self-closing tag.
func TestAttributeOnSelfClosing(t *testing.T) {
	pairs := tokenPairs(t, `<img src="photo.jpg"/>`)
	if !hasType(pairs, "SELF_CLOSE") {
		t.Error("Expected SELF_CLOSE token")
	}
	if !hasType(pairs, "ATTR_VALUE") {
		t.Error("Expected ATTR_VALUE token")
	}
}

// =============================================================================
// Comments
// =============================================================================

// TestSimpleComment verifies a simple comment: <!-- text -->.
//
// When `<!--` is matched, the callback pushes the "comment" group and
// disables skip. This means whitespace inside the comment is preserved
// as part of COMMENT_TEXT rather than being silently consumed.
func TestSimpleComment(t *testing.T) {
	pairs := tokenPairs(t, "<!-- hello -->")
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"COMMENT_START", "<!--"},
		{"COMMENT_TEXT", " hello "},
		{"COMMENT_END", "-->"},
	})
}

// TestCommentPreservesWhitespace verifies whitespace inside comments is
// preserved. Skip patterns are disabled in the comment group, so spaces
// and tabs appear as part of the COMMENT_TEXT token.
func TestCommentPreservesWhitespace(t *testing.T) {
	pairs := tokenPairs(t, "<!--  spaces  and\ttabs  -->")
	texts := valuesOfType(pairs, "COMMENT_TEXT")
	if len(texts) != 1 || texts[0] != "  spaces  and\ttabs  " {
		t.Errorf("Expected preserved whitespace, got %v", texts)
	}
}

// TestCommentWithDashes verifies comments can contain single dashes.
//
// The COMMENT_TEXT regex uses a negative lookahead pattern to match any
// character that isn't part of "-->": /([^-]|-(?!->))+/
func TestCommentWithDashes(t *testing.T) {
	pairs := tokenPairs(t, "<!-- a-b-c -->")
	texts := valuesOfType(pairs, "COMMENT_TEXT")
	if len(texts) != 1 || texts[0] != " a-b-c " {
		t.Errorf("Expected ' a-b-c ', got %v", texts)
	}
}

// TestCommentBetweenElements verifies a comment placed between two elements.
func TestCommentBetweenElements(t *testing.T) {
	pairs := tokenPairs(t, "<a/><!-- mid --><b/>")
	if !hasType(pairs, "COMMENT_START") {
		t.Error("Expected COMMENT_START token")
	}
	if !hasType(pairs, "COMMENT_END") {
		t.Error("Expected COMMENT_END token")
	}
}

// =============================================================================
// CDATA Sections
// =============================================================================

// TestSimpleCdata verifies a simple CDATA section.
//
// CDATA sections contain raw character data -- no entity processing, no
// tag recognition. Everything is literal text until `]]>` is seen.
func TestSimpleCdata(t *testing.T) {
	pairs := tokenPairs(t, "<![CDATA[raw text]]>")
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"CDATA_START", "<![CDATA["},
		{"CDATA_TEXT", "raw text"},
		{"CDATA_END", "]]>"},
	})
}

// TestCdataWithAngleBrackets verifies CDATA can contain < and > which
// are normally special characters that start/end tags.
func TestCdataWithAngleBrackets(t *testing.T) {
	pairs := tokenPairs(t, "<![CDATA[<not a tag>]]>")
	texts := valuesOfType(pairs, "CDATA_TEXT")
	if len(texts) != 1 || texts[0] != "<not a tag>" {
		t.Errorf("Expected '<not a tag>', got %v", texts)
	}
}

// TestCdataPreservesWhitespace verifies whitespace in CDATA is preserved.
//
// Like comments, CDATA sections have skip disabled so whitespace appears
// as part of the CDATA_TEXT token.
func TestCdataPreservesWhitespace(t *testing.T) {
	pairs := tokenPairs(t, "<![CDATA[  hello\n  world  ]]>")
	texts := valuesOfType(pairs, "CDATA_TEXT")
	if len(texts) != 1 || texts[0] != "  hello\n  world  " {
		t.Errorf("Expected preserved whitespace, got %v", texts)
	}
}

// TestCdataWithSingleBracket verifies CDATA can contain ] without ending.
//
// The CDATA_TEXT regex matches any character that isn't part of "]]>":
// /([^\]]|\](?!\]>))+/
// So a single ] does not end the CDATA section.
func TestCdataWithSingleBracket(t *testing.T) {
	pairs := tokenPairs(t, "<![CDATA[a]b]]>")
	texts := valuesOfType(pairs, "CDATA_TEXT")
	if len(texts) != 1 || texts[0] != "a]b" {
		t.Errorf("Expected 'a]b', got %v", texts)
	}
}

// =============================================================================
// Processing Instructions
// =============================================================================

// TestXmlDeclaration verifies the XML declaration: <?xml version="1.0"?>.
//
// Processing instructions (PIs) start with `<?` and end with `?>`. The
// PI_TARGET pattern matches the first name after `<?`, and PI_TEXT matches
// everything else up to `?>`.
func TestXmlDeclaration(t *testing.T) {
	pairs := tokenPairs(t, `<?xml version="1.0"?>`)
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"PI_START", "<?"},
		{"PI_TARGET", "xml"},
		{"PI_TEXT", ` version="1.0"`},
		{"PI_END", "?>"},
	})
}

// TestStylesheetPI verifies a stylesheet processing instruction.
func TestStylesheetPI(t *testing.T) {
	types := tokenTypes(t, `<?xml-stylesheet type="text/xsl"?>`)
	if len(types) < 3 {
		t.Fatalf("Expected at least 3 types, got %d", len(types))
	}
	if types[0] != "PI_START" {
		t.Errorf("Expected first type PI_START, got %q", types[0])
	}
	if types[1] != "PI_TARGET" {
		t.Errorf("Expected second type PI_TARGET, got %q", types[1])
	}
	if types[len(types)-1] != "PI_END" {
		t.Errorf("Expected last type PI_END, got %q", types[len(types)-1])
	}
}

// =============================================================================
// Entity and Character References
// =============================================================================

// TestNamedEntity verifies named entity references like &amp;.
//
// Entity references are recognized in the default group (between tags).
// The TEXT pattern matches `[^<&]+`, stopping at `&`. Then ENTITY_REF
// matches `&[a-zA-Z][a-zA-Z0-9]*;`.
func TestNamedEntity(t *testing.T) {
	pairs := tokenPairs(t, "a&amp;b")
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"TEXT", "a"},
		{"ENTITY_REF", "&amp;"},
		{"TEXT", "b"},
	})
}

// TestDecimalCharRef verifies decimal character references: &#65;.
func TestDecimalCharRef(t *testing.T) {
	pairs := tokenPairs(t, "&#65;")
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"CHAR_REF", "&#65;"},
	})
}

// TestHexCharRef verifies hexadecimal character references: &#x41;.
func TestHexCharRef(t *testing.T) {
	pairs := tokenPairs(t, "&#x41;")
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"CHAR_REF", "&#x41;"},
	})
}

// TestMultipleEntities verifies multiple entity references in text.
func TestMultipleEntities(t *testing.T) {
	types := tokenTypes(t, "&lt;hello&gt;")
	expected := []string{"ENTITY_REF", "TEXT", "ENTITY_REF"}
	if len(types) != len(expected) {
		t.Fatalf("Expected %d types, got %d: %v", len(expected), len(types), types)
	}
	for i, e := range expected {
		if types[i] != e {
			t.Errorf("Type %d: expected %q, got %q", i, e, types[i])
		}
	}
}

// =============================================================================
// Nested and Mixed Content
// =============================================================================

// TestNestedElements verifies nested elements: <a><b>text</b></a>.
//
// Nested elements require the group stack to push and pop correctly for
// each tag boundary. The lexer should handle arbitrary nesting depth.
func TestNestedElements(t *testing.T) {
	pairs := tokenPairs(t, "<a><b>text</b></a>")
	openCount := countType(pairs, "OPEN_TAG_START")
	closeCount := countType(pairs, "CLOSE_TAG_START")
	if openCount != 2 {
		t.Errorf("Expected 2 OPEN_TAG_START, got %d", openCount)
	}
	if closeCount != 2 {
		t.Errorf("Expected 2 CLOSE_TAG_START, got %d", closeCount)
	}
}

// TestMixedContent verifies text mixed with child elements.
//
// Mixed content is XML's way of interleaving text with child elements:
// <p>Hello <b>world</b>!</p>
// The TEXT tokens should capture each text segment separately.
func TestMixedContent(t *testing.T) {
	pairs := tokenPairs(t, "<p>Hello <b>world</b>!</p>")
	texts := valuesOfType(pairs, "TEXT")
	if len(texts) != 3 || texts[0] != "Hello " || texts[1] != "world" || texts[2] != "!" {
		t.Errorf("Expected [Hello , world, !], got %v", texts)
	}
}

// TestFullDocument verifies a small but complete XML document.
//
// This test exercises all major XML constructs in a single document:
// processing instruction, comment, attributes, entity references,
// nested tags, and text content.
func TestFullDocument(t *testing.T) {
	source := `<?xml version="1.0"?>` +
		`<!-- A greeting -->` +
		`<root lang="en">` +
		`<greeting>Hello &amp; welcome</greeting>` +
		`</root>`

	tokens, err := TokenizeXml(source)
	if err != nil {
		t.Fatalf("Failed to tokenize full document: %v", err)
	}

	var types []string
	for _, tok := range tokens {
		types = append(types, tok.TypeName)
	}

	// PI present
	if !contains(types, "PI_START") || !contains(types, "PI_END") {
		t.Error("Expected PI_START and PI_END in full document")
	}

	// Comment present
	if !contains(types, "COMMENT_START") || !contains(types, "COMMENT_END") {
		t.Error("Expected COMMENT_START and COMMENT_END in full document")
	}

	// Tags present: root + greeting = 2 opens, 2 closes
	openCount := countStr(types, "OPEN_TAG_START")
	closeCount := countStr(types, "CLOSE_TAG_START")
	if openCount != 2 {
		t.Errorf("Expected 2 OPEN_TAG_START, got %d", openCount)
	}
	if closeCount != 2 {
		t.Errorf("Expected 2 CLOSE_TAG_START, got %d", closeCount)
	}

	// Entity ref present
	if !contains(types, "ENTITY_REF") {
		t.Error("Expected ENTITY_REF in full document")
	}

	// Ends with EOF
	if types[len(types)-1] != "EOF" {
		t.Errorf("Expected last token EOF, got %q", types[len(types)-1])
	}
}

// TestCdataInsideElement verifies a CDATA section inside an element.
func TestCdataInsideElement(t *testing.T) {
	pairs := tokenPairs(t, "<script><![CDATA[x < y]]></script>")
	if !hasType(pairs, "CDATA_START") {
		t.Error("Expected CDATA_START token")
	}
	if !hasType(pairs, "CDATA_TEXT") {
		t.Error("Expected CDATA_TEXT token")
	}
	if !hasType(pairs, "CDATA_END") {
		t.Error("Expected CDATA_END token")
	}
}

// =============================================================================
// Edge Cases
// =============================================================================

// TestEmptyString verifies empty input produces only EOF.
func TestEmptyString(t *testing.T) {
	tokens, err := TokenizeXml("")
	if err != nil {
		t.Fatalf("Failed to tokenize empty string: %v", err)
	}
	if len(tokens) != 1 {
		t.Fatalf("Expected 1 token (EOF), got %d: %v", len(tokens), tokens)
	}
	if tokens[0].TypeName != "EOF" {
		t.Errorf("Expected EOF, got %q", tokens[0].TypeName)
	}
}

// TestTextOnly verifies plain text with no tags.
func TestTextOnly(t *testing.T) {
	pairs := tokenPairs(t, "just text")
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"TEXT", "just text"},
	})
}

// TestWhitespaceBetweenTagsSkipped verifies whitespace between tags is
// consumed by skip patterns. The XML grammar has a skip pattern for
// whitespace (WHITESPACE = /[ \t\r\n]+/), so spaces between tags are
// silently consumed -- no TEXT tokens are emitted for inter-tag whitespace.
func TestWhitespaceBetweenTagsSkipped(t *testing.T) {
	pairs := tokenPairs(t, "<a> <b> </b> </a>")
	texts := valuesOfType(pairs, "TEXT")
	if len(texts) != 0 {
		t.Errorf("Expected no TEXT tokens (whitespace consumed by skip), got %v", texts)
	}
}

// TestEofAlwaysLast verifies the last token is always EOF.
func TestEofAlwaysLast(t *testing.T) {
	tokens, err := TokenizeXml("<root/>")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}
	last := tokens[len(tokens)-1]
	if last.TypeName != "EOF" {
		t.Errorf("Expected last token EOF, got %q", last.TypeName)
	}
}

// TestCreateXmlLexer verifies the factory function returns a valid
// GrammarLexer that can be used for tokenization.
func TestCreateXmlLexer(t *testing.T) {
	xmlLexer, err := CreateXmlLexer("<p>hello</p>")
	if err != nil {
		t.Fatalf("Failed to create XML lexer: %v", err)
	}
	if xmlLexer == nil {
		t.Fatal("CreateXmlLexer returned nil")
	}

	tokens := xmlLexer.Tokenize()
	if len(tokens) < 2 {
		t.Fatalf("Expected at least 2 tokens, got %d", len(tokens))
	}

	last := tokens[len(tokens)-1]
	if last.Type != lexer.TokenEOF {
		t.Errorf("Expected last token to be EOF, got %s", last.Type)
	}
}

// TestLineAndColumn verifies tokens have correct line and column info.
//
// Position tracking is important for error reporting. The first character
// is at line 1, column 1.
func TestLineAndColumn(t *testing.T) {
	tokens, err := TokenizeXml("<p>")
	if err != nil {
		t.Fatalf("Failed to tokenize: %v", err)
	}

	// First token OPEN_TAG_START should be at line 1, column 1
	if tokens[0].Line != 1 {
		t.Errorf("Expected line 1, got %d", tokens[0].Line)
	}
	if tokens[0].Column != 1 {
		t.Errorf("Expected column 1, got %d", tokens[0].Column)
	}
}

// TestMultipleComments verifies multiple comments in sequence.
func TestMultipleComments(t *testing.T) {
	pairs := tokenPairs(t, "<!-- one --><!-- two -->")
	commentCount := countType(pairs, "COMMENT_START")
	if commentCount != 2 {
		t.Errorf("Expected 2 COMMENT_START, got %d", commentCount)
	}
	endCount := countType(pairs, "COMMENT_END")
	if endCount != 2 {
		t.Errorf("Expected 2 COMMENT_END, got %d", endCount)
	}
}

// TestDeeplyNestedElements verifies the group stack handles deep nesting.
func TestDeeplyNestedElements(t *testing.T) {
	source := "<a><b><c><d>deep</d></c></b></a>"
	pairs := tokenPairs(t, source)
	openCount := countType(pairs, "OPEN_TAG_START")
	closeCount := countType(pairs, "CLOSE_TAG_START")
	if openCount != 4 {
		t.Errorf("Expected 4 OPEN_TAG_START, got %d", openCount)
	}
	if closeCount != 4 {
		t.Errorf("Expected 4 CLOSE_TAG_START, got %d", closeCount)
	}
	texts := valuesOfType(pairs, "TEXT")
	if len(texts) != 1 || texts[0] != "deep" {
		t.Errorf("Expected [deep], got %v", texts)
	}
}

// TestEmptyComment verifies an empty comment: <!---->.
func TestEmptyComment(t *testing.T) {
	pairs := tokenPairs(t, "<!---->")
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"COMMENT_START", "<!--"},
		{"COMMENT_END", "-->"},
	})
}

// TestEmptyCdata verifies an empty CDATA section: <![CDATA[]]>.
func TestEmptyCdata(t *testing.T) {
	pairs := tokenPairs(t, "<![CDATA[]]>")
	assertPairs(t, pairs, []struct{ TypeName, Value string }{
		{"CDATA_START", "<![CDATA["},
		{"CDATA_END", "]]>"},
	})
}

// =============================================================================
// Internal helpers for string slices
// =============================================================================

func contains(slice []string, item string) bool {
	for _, s := range slice {
		if s == item {
			return true
		}
	}
	return false
}

func countStr(slice []string, item string) int {
	n := 0
	for _, s := range slice {
		if s == item {
			n++
		}
	}
	return n
}
