# Changelog

## 0.1.0

- Encode and parse HTTP/0.9 request lines and decode completed ASCII responses.
- Parse the archived HTML Tags vocabulary, including omitted list-item ends,
  paragraph markers, early named entities, and literal text sections.
- Verify the wire codec and markup trees using shared JSON vectors.
- Add bounded VER05c/VER06d assurance with independent wire and omitted-`LI`
  oracles, negative controls, and a 2,048-iteration public-export fuzz run.
