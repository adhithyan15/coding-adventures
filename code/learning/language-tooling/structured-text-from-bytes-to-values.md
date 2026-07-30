<!-- learning-concepts: csv-parser, json-lexer, json-parser, json-value, json-serializer, toml-lexer, toml-parser, xml-lexer -->
# Structured Text From Bytes To Values

JSON, CSV, TOML, and XML all turn text into structure, but they do not have the
same shape. Treating every format as "split strings and fill a map" works only
until quoting, nesting, escapes, or duplicate keys appear.

The reusable pipeline is:

```text
bytes -> decoded text -> tokens or fields -> syntax tree -> semantic values
```

Not every format needs every stage. The important habit is to name the stage
whose rules you are applying.

## Bytes Are Not Yet Characters

A parser should receive text in a known character encoding. UTF-8 decoding can
fail, a byte-order mark may be permitted only at the beginning, and line ending
rules vary by format. Source positions should track enough information to point
back to the original input, commonly a byte offset plus line and column.

This distinction matters because one displayed character can occupy several
UTF-8 bytes. A byte offset is useful for slicing; a line and column are useful
for a person reading an error.

## Lexing Separates Local Decisions

A lexer recognizes local units such as punctuation, strings, numbers, names,
and comments:

```json
{"ready": true, "count": 3}
```

becomes roughly:

```text
LBRACE STRING COLON TRUE COMMA STRING COLON NUMBER RBRACE
```

String scanning is more than searching for the next quote. The scanner must
handle escapes, reject unescaped control characters, and report an unfinished
escape at its actual source position. Number scanning must implement the
format's grammar rather than whatever the host language happens to parse.

JSON benefits from a conventional lexer/parser split. TOML does too because
dates, dotted keys, arrays, inline tables, comments, and several string forms
create substantial local syntax. XML lexing changes mode between markup and
character data, so `<` means something different inside and outside a tag.

## Parsing Enforces Structure

The parser combines local tokens according to a grammar. JSON values are
recursive:

```text
value  := object | array | string | number | true | false | null
object := "{" members? "}"
array  := "[" values? "]"
```

Recursion makes nesting easy to express but dangerous to leave unbounded. A
production parser should limit source size, nesting depth, token count, and
possibly the size of any one collection.

An AST should preserve syntax when later tools need source ranges, comments, or
exact spellings. A semantic value model can be smaller:

```text
Null
Boolean
Number
String
Array<Value>
Object<String, Value>
```

Keeping AST and value conversion separate lets a formatter preserve source
details while an application consumes convenient values.

## CSV Is A State Machine, Not A Split

CSV looks flat, but quoted fields may contain delimiters and line breaks:

```csv
name,notes
Ada,"first line
second line"
```

Splitting by newline first destroys the second record. Splitting by comma then
misreads commas inside quotes. A CSV parser instead moves through states:

```text
field start -> unquoted field -> delimiter/end
            -> quoted field -> quote seen -> delimiter/end or escaped quote
```

The dialect must define delimiter, quote character, record terminator policy,
and whether whitespace outside quotes is data. Row-width validation is a
semantic choice: some consumers reject ragged rows, while others preserve them.

## TOML Adds Assignment Semantics

TOML syntax constructs a configuration tree over time. These lines:

```toml
[server]
host = "localhost"
ports = [8080, 8081]
```

do more than parse isolated values. The table header changes the destination
for later keys. Dotted keys create paths. A correct implementation must reject
conflicting redefinitions, such as treating the same path as both a scalar and
a table.

Dates and times are another reason not to lower immediately into host-language
primitives. A format-specific value can preserve whether an input represented
a local date, local time, local date-time, or offset date-time.

## XML Mixes Trees And Streams

XML elements naturally form a tree, but large documents are often processed as
events:

```text
start_element("book")
text("...")
end_element("book")
```

A tree API is convenient for random access. A streaming API bounds memory and
can begin work before the final byte arrives. The lexer must distinguish tags,
attributes, entity references, comments, CDATA, and text. The parser must match
start and end tags and enforce a single document root.

Entity expansion is a security boundary. Implementations need explicit limits
on expansion count and output size, and applications should not grant ambient
filesystem or network access to external entities.

## Serialization Is Not `toString`

A serializer chooses a valid representation for semantic values. It must:

- escape strings according to the target format;
- reject or define non-finite numbers;
- choose deterministic key ordering when reproducibility matters;
- detect cycles in host objects;
- enforce depth and output-size limits;
- distinguish absent values from explicit `null` where the format does.

Parsing and serialization are not perfect inverses when a value model discards
comments, key spelling, numeric spelling, or ordering. State the promised
round-trip:

```text
semantic round-trip: parse(serialize(value)) == value
syntax round-trip:   print(parse(source)) == source
```

Most data APIs promise the first. Formatters and source editors need the second
or a documented normalized form.

## A Practical Test Matrix

For each format, include:

1. a small valid example;
2. every escape and delimiter boundary;
3. empty values and empty collections;
4. deeply nested or very wide input at configured limits;
5. truncated input at every token boundary;
6. duplicate or conflicting keys;
7. invalid Unicode or encoding boundaries;
8. parse/serialize semantic round-trips;
9. differential cases against a trusted implementation.

The durable lesson is not that all text formats are alike. It is that decoding,
local recognition, structural parsing, semantic conversion, and serialization
are different contracts. Bugs become much easier to locate once the contracts
stop bleeding into one another.
