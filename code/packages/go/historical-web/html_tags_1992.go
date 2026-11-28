package historicalweb

import "strings"

// Node is one text or element node in the historical tree.
type Node interface{ historicalNode() }

// TextNode stores text after the profile's single entity-decoding pass.
type TextNode struct{ Text string }

func (TextNode) historicalNode() {}

// Element stores a lowercase historical tag and ordered contents.
type Element struct {
	Tag        string
	Attributes map[string]*string
	Children   []Node
}

func (Element) historicalNode() {}

var knownTags = map[string]bool{
	"title": true, "address": true, "a": true, "dl": true, "li": true,
	"xmp": true, "listing": true, "plaintext": true, "ul": true,
	"menu": true, "dir": true, "dt": true, "dd": true, "p": true,
	"isindex": true, "h1": true, "h2": true, "h3": true, "h4": true,
	"h5": true, "h6": true,
}
var emptyTags = map[string]bool{"p": true, "isindex": true}
var listTags = map[string]bool{"ul": true, "menu": true, "dir": true}
var termTags = map[string]bool{"dt": true, "dd": true}

type htmlToken struct {
	name, attrs string
	closing     bool
	end         int
}

func isSpace(c byte) bool     { return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' }
func isNameStart(c byte) bool { return c >= 'A' && c <= 'Z' || c >= 'a' && c <= 'z' }
func isNameChar(c byte) bool  { return isNameStart(c) || c >= '0' && c <= '9' || c == '-' }
func lowerASCII(s string) string {
	b := strings.Builder{}
	b.Grow(len(s))
	for i := 0; i < len(s); i++ {
		c := s[i]
		if c >= 'A' && c <= 'Z' {
			c += 'a' - 'A'
		}
		b.WriteByte(c)
	}
	return b.String()
}

func tagAt(source string, start int) (htmlToken, bool) {
	if start+2 > len(source) || source[start] != '<' {
		return htmlToken{}, false
	}
	i, closing := start+1, false
	if source[i] == '/' {
		closing = true
		i++
	}
	if i >= len(source) || !isNameStart(source[i]) {
		return htmlToken{}, false
	}
	nameStart := i
	for i < len(source) && isNameChar(source[i]) {
		i++
	}
	nameEnd := i
	if i < len(source) && source[i] != '>' && !isSpace(source[i]) {
		return htmlToken{}, false
	}
	quote := byte(0)
	for i < len(source) {
		c := source[i]
		if c == '<' {
			return htmlToken{}, false
		}
		if quote != 0 {
			if c == quote {
				quote = 0
			}
		} else if c == '\'' || c == '"' {
			quote = c
		} else if c == '>' {
			return htmlToken{lowerASCII(source[nameStart:nameEnd]), source[nameEnd:i], closing, i + 1}, true
		}
		i++
	}
	return htmlToken{}, false
}

func decodeEntities(s string) string {
	return strings.ReplaceAll(strings.ReplaceAll(strings.ReplaceAll(s, "&lt;", "<"), "&gt;", ">"), "&amp;", "&")
}

func anchorAttrs(source string) map[string]*string {
	result := make(map[string]*string)
	for i := 0; i < len(source); {
		spaceStart := i
		for i < len(source) && isSpace(source[i]) {
			i++
		}
		if i == spaceStart || i >= len(source) || !isNameStart(source[i]) {
			break
		}
		start := i
		for i < len(source) && isNameChar(source[i]) {
			i++
		}
		name := lowerASCII(source[start:i])
		for i < len(source) && isSpace(source[i]) {
			i++
		}
		var value *string
		if i < len(source) && source[i] == '=' {
			i++
			for i < len(source) && isSpace(source[i]) {
				i++
			}
			value = attrValue(source, &i)
		}
		if name == "href" || name == "name" || name == "type" {
			if _, exists := result[name]; exists {
				continue
			}
			result[name] = value
		}
	}
	return result
}

func attrValue(source string, cursor *int) *string {
	i := *cursor
	if i >= len(source) {
		return nil
	}
	if source[i] == '\'' || source[i] == '"' {
		quote := source[i]
		i++
		start := i
		for i < len(source) && source[i] != quote {
			i++
		}
		if i >= len(source) {
			*cursor = i
			return nil
		}
		value := decodeEntities(source[start:i])
		i++
		*cursor = i
		return &value
	}
	start := i
	for i < len(source) && !isSpace(source[i]) && !strings.ContainsRune("\"'=<>", rune(source[i])) {
		i++
	}
	if start == i {
		return nil
	}
	value := decodeEntities(source[start:i])
	*cursor = i
	return &value
}

func appendText(nodes *[]Node, text string) {
	if text == "" {
		return
	}
	*nodes = append(*nodes, TextNode{Text: text})
}

func closeIndex(positions map[string][]int, names, boundaries map[string]bool) int {
	match, boundary := -1, -1
	for name := range names {
		if values := positions[name]; len(values) > 0 && values[len(values)-1] > match {
			match = values[len(values)-1]
		}
	}
	for name := range boundaries {
		if values := positions[name]; len(values) > 0 && values[len(values)-1] > boundary {
			boundary = values[len(values)-1]
		}
	}
	if match > boundary {
		return match
	}
	return -1
}
func truncateStack(stack []*Element, positions map[string][]int, at int) []*Element {
	for len(stack) > at {
		tag := stack[len(stack)-1].Tag
		stack = stack[:len(stack)-1]
		values := positions[tag]
		positions[tag] = values[:len(values)-1]
	}
	return stack
}
func indexFold(source, needle string, from int) int {
	for i := from; i+len(needle) <= len(source); i++ {
		if asciiEqualFold(source[i:i+len(needle)], needle) {
			return i
		}
	}
	return -1
}
func asciiEqualFold(left, right string) bool {
	if len(left) != len(right) {
		return false
	}
	for i := range left {
		a, b := left[i], right[i]
		if a >= 'A' && a <= 'Z' {
			a += 'a' - 'A'
		}
		if b >= 'A' && b <= 'Z' {
			b += 'a' - 'A'
		}
		if a != b {
			return false
		}
	}
	return true
}

// ParseHTMLTags1992 builds the HWEB00 teaching profile tree.
func ParseHTMLTags1992(source string) ([]Node, error) {
	// Python's source normalization happens before tokenization, including inside
	// quoted attributes. Normalize once so every scanner sees the same input.
	source = strings.ReplaceAll(source, "\r\n", "\n")
	var root []Node
	stack := make([]*Element, 0)
	positions := make(map[string][]int)
	textStart := 0
	flush := func(end int) {
		if end <= textStart {
			return
		}
		text := decodeEntities(strings.ReplaceAll(source[textStart:end], "\r\n", "\n"))
		if len(stack) == 0 {
			appendText(&root, text)
		} else {
			appendText(&stack[len(stack)-1].Children, text)
		}
		textStart = end
	}
	for i := 0; i < len(source); {
		rel := strings.IndexByte(source[i:], '<')
		if rel < 0 {
			flush(len(source))
			break
		}
		start := i + rel
		if start > textStart {
			flush(start)
		}
		if strings.HasPrefix(source[start:], "<!--") {
			end := strings.Index(source[start+4:], "-->")
			if end < 0 {
				return publicNodes(root), nil
			}
			i = start + 4 + end + 3
			textStart = i
			continue
		}
		token, ok := tagAt(source, start)
		if !ok {
			target := &root
			if len(stack) > 0 {
				target = &stack[len(stack)-1].Children
			}
			appendText(target, "<")
			i = start + 1
			textStart = i
			continue
		}
		name := token.name
		if !knownTags[name] {
			i = token.end
			textStart = i
			continue
		}
		if !token.closing && name == "plaintext" {
			e := &Element{Tag: name, Attributes: map[string]*string{}}
			appendElement(&root, stack, e)
			appendText(&e.Children, strings.ReplaceAll(source[token.end:], "\r\n", "\n"))
			return publicNodes(root), nil
		}
		if !token.closing && (name == "xmp" || name == "listing") {
			e := &Element{Tag: name, Attributes: map[string]*string{}}
			appendElement(&root, stack, e)
			needle := "</" + name + ">"
			end := indexFold(source, needle, token.end)
			if end < 0 {
				appendText(&e.Children, strings.ReplaceAll(source[token.end:], "\r\n", "\n"))
				return publicNodes(root), nil
			}
			appendText(&e.Children, strings.ReplaceAll(source[token.end:end], "\r\n", "\n"))
			i = end + len(needle)
			textStart = i
			continue
		}
		if token.closing {
			if !emptyTags[name] {
				if at := closeIndex(positions, map[string]bool{name: true}, nil); at >= 0 {
					stack = truncateStack(stack, positions, at)
				}
			}
			i = token.end
			textStart = i
			continue
		}
		if emptyTags[name] {
			appendElement(&root, stack, &Element{Tag: name, Attributes: map[string]*string{}})
			i = token.end
			textStart = i
			continue
		}
		names, boundaries := map[string]bool{}, map[string]bool{}
		if name == "li" {
			names["li"], boundaries = true, listTags
		}
		if termTags[name] {
			names["dt"], names["dd"], boundaries["dl"] = true, true, true
		}
		if name >= "h1" && name <= "h6" {
			for j := 1; j <= 6; j++ {
				names["h"+string(rune('0'+j))] = true
			}
		}
		if at := closeIndex(positions, names, boundaries); at >= 0 {
			stack = truncateStack(stack, positions, at)
		}
		e := &Element{Tag: name, Attributes: map[string]*string{}}
		if name == "a" {
			e.Attributes = anchorAttrs(token.attrs)
		}
		appendElement(&root, stack, e)
		positions[name] = append(positions[name], len(stack))
		stack = append(stack, e)
		i = token.end
		textStart = i
	}
	if textStart < len(source) {
		flush(len(source))
	}
	return publicNodes(root), nil
}

func appendElement(root *[]Node, stack []*Element, element *Element) {
	if len(stack) == 0 {
		*root = append(*root, element)
	} else {
		stack[len(stack)-1].Children = append(stack[len(stack)-1].Children, element)
	}
}

func publicNodes(nodes []Node) []Node {
	// Build a postorder list without recursion; deeply nested input must not use
	// the Go call stack. Text fragments are coalesced with one builder per list.
	type frame struct {
		source []Node
		index  int
	}
	order := make([]*Element, 0)
	stack := []frame{{source: nodes}}
	for len(stack) > 0 {
		f := &stack[len(stack)-1]
		if f.index == len(f.source) {
			stack = stack[:len(stack)-1]
			continue
		}
		n, ok := f.source[f.index].(*Element)
		f.index++
		if ok {
			order = append(order, n)
			stack = append(stack, frame{source: n.Children})
		}
	}
	converted := make(map[*Element]Element, len(order))
	for i := len(order) - 1; i >= 0; i-- {
		e := order[i]
		converted[e] = Element{Tag: e.Tag, Attributes: e.Attributes, Children: convertList(e.Children, converted)}
	}
	return convertList(nodes, converted)
}

func convertList(nodes []Node, converted map[*Element]Element) []Node {
	out := make([]Node, 0, len(nodes))
	var pending strings.Builder
	flush := func() {
		if pending.Len() > 0 {
			out = append(out, TextNode{Text: pending.String()})
			pending.Reset()
		}
	}
	for _, n := range nodes {
		switch v := n.(type) {
		case TextNode:
			pending.WriteString(v.Text)
		case *Element:
			flush()
			out = append(out, converted[v])
		case Element:
			flush()
			v.Children = convertList(v.Children, converted)
			out = append(out, v)
		}
	}
	flush()
	return out
}
