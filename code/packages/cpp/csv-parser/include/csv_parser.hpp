// csv_parser.hpp — an RFC 4180 CSV parser, in pure ISO C++17, header-only, in
// namespace ca::csv. A faithful port of the Rust `csv-parser` crate.
// ===========================================================================
//
// Parses comma-separated (or any single-byte delimiter) text into rows of
// string fields, honouring the awkward parts of the format:
//
//   - Quoted fields, embedded delimiters and newlines: "a, b" / "line1\nline2"
//   - Escaped quotes: "she said ""hi""" -> a "" becomes a single "
//   - Ragged rows: short rows pad with "", extra fields are dropped
//   - Line endings: \n, \r and \r\n are all recognised
//   - Optional trailing newline
//
// The only hard error is an unclosed quoted field (EOF inside "..."), reported
// by throwing ca::csv::UnclosedQuote.
//
// Two views mirror the crate:
//   parse_records — the raw grid (Grid = vector<vector<string>>) in file order,
//                   including the header row.
//   parse         — a header-mapped view: the first row is the header, and each
//                   later row becomes a map<string,string> keyed by column name
//                   (missing columns read as "", extra fields dropped).
//
// The state machine only branches on ASCII bytes; multibyte UTF-8 content is
// copied verbatim into fields. The delimiter is a single byte.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_CSV_PARSER_HPP
#define CA_CSV_PARSER_HPP

#include <cstddef>
#include <map>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace csv {

// Thrown when the input ends while still inside a quoted field.
class UnclosedQuote : public std::runtime_error {
public:
    UnclosedQuote()
        : std::runtime_error(
              "unclosed quoted field: EOF reached inside a quoted field") {}
};

using Row = std::vector<std::string>;
using Grid = std::vector<Row>;
using Record = std::map<std::string, std::string>;

namespace detail {
enum State { FieldStart, InUnquoted, InQuoted, InQuotedMaybeEnd };
}

// Parse `source` into its raw grid using `delimiter` (throws UnclosedQuote).
inline Grid parse_records(const std::string& source, char delimiter = ',') {
    using namespace detail;
    Grid rows;
    Row current_row;
    std::string field;
    State state = FieldStart;
    std::size_t i = 0, len = source.size();
    unsigned char delim = static_cast<unsigned char>(delimiter);

    auto push_field = [&]() {
        current_row.push_back(std::move(field));
        field.clear();
    };
    auto push_row = [&]() {
        rows.push_back(std::move(current_row));
        current_row.clear();
    };

    while (i < len) {
        unsigned char ch = static_cast<unsigned char>(source[i]);
        switch (state) {
            case FieldStart:
                if (ch == '"') {
                    state = InQuoted;
                } else if (ch == delim) {
                    push_field(); // empty field
                } else if (ch == '\n' || ch == '\r') {
                    if (!current_row.empty()) {
                        push_field();
                    }
                    if (ch == '\r' && i + 1 < len && source[i + 1] == '\n') {
                        ++i;
                    }
                    push_row();
                } else {
                    field.push_back(static_cast<char>(ch));
                    state = InUnquoted;
                }
                break;
            case InUnquoted:
                if (ch == delim) {
                    push_field();
                    state = FieldStart;
                } else if (ch == '\n' || ch == '\r') {
                    push_field();
                    if (ch == '\r' && i + 1 < len && source[i + 1] == '\n') {
                        ++i;
                    }
                    push_row();
                    state = FieldStart;
                } else {
                    field.push_back(static_cast<char>(ch));
                }
                break;
            case InQuoted:
                if (ch == '"') {
                    state = InQuotedMaybeEnd;
                } else {
                    field.push_back(static_cast<char>(ch));
                }
                break;
            case InQuotedMaybeEnd:
                if (ch == '"') {
                    field.push_back('"'); // escaped "" -> "
                    state = InQuoted;
                } else if (ch == delim) {
                    push_field();
                    state = FieldStart;
                } else if (ch == '\n' || ch == '\r') {
                    push_field();
                    if (ch == '\r' && i + 1 < len && source[i + 1] == '\n') {
                        ++i;
                    }
                    push_row();
                    state = FieldStart;
                } else { // lenient: closing quote then more text
                    field.push_back(static_cast<char>(ch));
                    state = InUnquoted;
                }
                break;
        }
        ++i;
    }

    // Flush the final field / row.
    if (state == InQuoted) {
        throw UnclosedQuote();
    }
    if (state == InUnquoted || state == InQuotedMaybeEnd) {
        push_field();
    }
    if (!current_row.empty()) {
        push_row();
    }
    return rows;
}

// Parse `source` into header-mapped records (throws UnclosedQuote). The first
// row is the header; each later row becomes a map keyed by column name (missing
// columns read as "", extra fields dropped; a repeated header name keeps the
// last column's value, matching the crate's HashMap).
inline std::vector<Record> parse(const std::string& source,
                                 char delimiter = ',') {
    Grid rows = parse_records(source, delimiter);
    std::vector<Record> result;
    if (rows.empty()) {
        return result;
    }
    const Row& header = rows[0];
    for (std::size_t r = 1; r < rows.size(); ++r) {
        Record record;
        for (std::size_t idx = 0; idx < header.size(); ++idx) {
            record[header[idx]] =
                idx < rows[r].size() ? rows[r][idx] : std::string();
        }
        result.push_back(std::move(record));
    }
    return result;
}

}  // namespace csv
}  // namespace ca

#endif  // CA_CSV_PARSER_HPP
