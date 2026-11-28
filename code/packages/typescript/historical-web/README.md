# @coding-adventures/historical-web

An in-memory TypeScript implementation of two early-Web profiles from
[HWEB00](../../../specs/HWEB00-historical-web.md): CERN's 1991 HTTP/0.9 wire
rules and the W3C archive's November 3, 1992 *HTML Tags* vocabulary. The
runtime uses the standard JavaScript library only. It does not open sockets,
read files, fetch URLs, or render markup.

```ts
import {
  decodeHttp09Response,
  encodeHttp09Request,
  Http09Address,
  parseHtmlTags1992,
} from "@coding-adventures/historical-web";

const address = Http09Address.create("/index.html");
if (address.ok) {
  const request = encodeHttp09Request(address.value);
  // Uint8Array [71, 69, 84, 32, 47, ... , 13, 10]
}

const body = decodeHttp09Response(new TextEncoder().encode("Hello\r\n"));
if (body.ok) console.log(body.value); // preserves CRLF

const parsed = parseHtmlTags1992("<UL><LI>One<LI>Two</UL>");
if (parsed.ok) {
  const list = parsed.value.nodes[parsed.value.roots[0]];
  // The flat node arena stores child indices instead of recursive objects.
}
```

The HTML scanner uses an explicit stack and tag-indexed open scopes. Its arena
avoids recursive build and drop for deeply nested documents, and text runs
coalesce through fragment arrays. Tag and attribute folding is ASCII-only.
HTML source strings retain JavaScript's UTF-16 code-unit behavior; this parser
does not normalize Unicode or validate scalar values.

## Three historical tree models

The archived 1992 *HTML Tags* vocabulary is an SGML-era language and documents
omitted list and glossary end tags. Thus `<UL><LI>One<LI>Two</UL>` has two
list items here. XML 1.0 is a later, case-sensitive language with explicit end
tags, so this source is not well-formed XML. Compare the repository's
[XML parser](../../../specs/XML01-xml-parser.md). Venture's current browser
parser implements the WHATWG HTML tokenizer and modern tree construction and
error recovery, a substantially larger model than this historical profile.

See HWEB04 for the public API, UTF-16 input domain, recovery rules, and
conformance limits. Shared vectors live in
`code/specs/fixtures/historical-web-v1/vectors.json`.
