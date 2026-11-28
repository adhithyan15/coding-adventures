# Changelog

## 0.3.0

- Add the versioned HTTP/1.0 request-line subset with exact framing, three
  closed methods, and a restricted ASCII path alphabet.

## 0.2.0

- Add the strict `html2-paragraphs-1995` text-only P/H1 profile, with omitted P
  end handling and explicit H1 closure based on RFC 1866.

## 0.1.0

- Encode and parse HTTP/0.9 request lines and decode completed ASCII responses.
- Parse the archived HTML Tags vocabulary, including omitted list-item ends,
  paragraph markers, early named entities, and literal text sections.
- Verify the wire codec and markup trees using shared JSON vectors.
- Add bounded VER05c/VER06d assurance with independent wire and omitted-`LI`
  oracles, negative controls, and a 2,048-iteration public-export fuzz run.
