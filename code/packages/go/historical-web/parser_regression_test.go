package historicalweb_test

import (
	"strings"
	"testing"

	historicalweb "github.com/adhithyan15/coding-adventures/code/packages/go/historical-web"
)

func TestHTMLParserRegressionNormalizationAndAttributes(t *testing.T) {
	nodes, err := historicalweb.ParseHTMLTags1992("<a href=\"line1\r\nline2\">x</a>")
	if err != nil {
		t.Fatal(err)
	}
	e := nodes[0].(historicalweb.Element)
	if got := *e.Attributes["href"]; got != "line1\nline2" {
		t.Fatalf("CRLF in quoted attribute: %q", got)
	}
	nodes, err = historicalweb.ParseHTMLTags1992(`<a 1href=wrong href=right>x</a>`)
	if err != nil {
		t.Fatal(err)
	}
	e = nodes[0].(historicalweb.Element)
	if len(e.Attributes) != 0 {
		t.Fatalf("scanner accepted attribute after invalid prefix: %#v", e.Attributes)
	}
}

func TestHTMLParserRegressionDepthAndAdjacentText(t *testing.T) {
	const depth = 12000
	source := strings.Repeat("<address>", depth) + "x" + strings.Repeat("</address>", depth)
	nodes, err := historicalweb.ParseHTMLTags1992(source)
	if err != nil {
		t.Fatal(err)
	}
	if len(nodes) != 1 {
		t.Fatalf("got %d root nodes", len(nodes))
	}
	malformed := strings.Repeat("<1", 20000) + "tail"
	nodes, err = historicalweb.ParseHTMLTags1992(malformed)
	if err != nil {
		t.Fatal(err)
	}
	if len(nodes) != 1 {
		t.Fatalf("got %d adjacent text nodes", len(nodes))
	}
	text, ok := nodes[0].(historicalweb.TextNode)
	if !ok || text.Text != malformed {
		t.Fatalf("malformed text did not round trip")
	}
}
