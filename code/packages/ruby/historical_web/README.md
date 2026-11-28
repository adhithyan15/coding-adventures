# coding_adventures_historical_web

This Ruby package implements two early-Web profiles described in
[HWEB00](../../../specs/HWEB00-historical-web.md): CERN's 1991 HTTP/0.9 wire
rules and the W3C archive's November 3, 1992 *HTML Tags* vocabulary. Runtime
code uses only Ruby's standard library and caller-supplied strings; it does not
open sockets, read files, fetch URLs, or render markup.

```ruby
require "coding_adventures/historical_web"

historical_web = CodingAdventures::HistoricalWeb

address_result = historical_web::Http09Address.create("/index.html")
raise address_result.error unless address_result.ok?

wire_result = historical_web.encode_http09_request(address_result.value)
raise wire_result.error unless wire_result.ok?
wire_result.value # => "GET /index.html\r\n" as ASCII-8BIT bytes

response = historical_web.decode_http09_response("Hello\r\n".b)
raise response.error unless response.ok?
response.value.encoding # => Encoding::US_ASCII

tree_result = historical_web.parse_html_tags_1992("<UL><LI>One<LI>Two</UL>")
raise tree_result.error unless tree_result.ok?
document = tree_result.value
document.nodes[document.roots.first] # => the flat `ul` element node
```

`HtmlDocument` stores nodes in one arena and represents child ownership with
integer indices. The parser's explicit stack and tag-indexed position lists
avoid recursive construction and drop. Adjacent text uses fragment buffers so
unknown markup does not trigger repeated whole-string copying. HTTP requests
and addresses are byte-oriented; HTML admits valid UTF-8 or US-ASCII and folds
only ASCII tag and attribute letters.

## SGML, XML, and today's HTML

The archived 1992 *HTML Tags* vocabulary is an SGML-era language. It documents
omitted list and glossary end tags, so `<UL><LI>One<LI>Two</UL>` contains two
list items in this profile. XML 1.0 is a later, case-sensitive language that
requires explicit end tags, so that source is not well-formed XML. Compare the
repository's [XML parser](../../../specs/XML01-xml-parser.md). Venture's
current browser parser implements the WHATWG HTML tokenizer, tree construction,
and modern error recovery, a much larger model than this historical profile.

See HWEB05 for exact result symbols, input encoding rules, scanner recovery,
and conformance limits. Shared fixtures are in
`code/specs/fixtures/historical-web-v1/vectors.json`.
