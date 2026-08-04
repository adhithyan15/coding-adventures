// Tests for the C++ csv-parser, using the iso_test.h harness. Cases are taken
// from the Rust crate's own tests (RFC 4180 behaviours).
#include "iso_test.h"

#include <string>
#include <vector>

#include "csv_parser.hpp"

namespace csv = ca::csv;

int main() {
    // Simple three-column table.
    {
        auto rows = csv::parse("name,age,city\nAlice,30,New York\nBob,25,London\n");
        ISO_CHECK_EQ_UINT(rows.size(), 2u);
        ISO_CHECK(rows[0].at("name") == "Alice");
        ISO_CHECK(rows[0].at("age") == "30");
        ISO_CHECK(rows[0].at("city") == "New York");
        ISO_CHECK(rows[1].at("name") == "Bob");
        ISO_CHECK(rows[1].at("city") == "London");
    }

    // No trailing newline.
    {
        auto rows = csv::parse("name,value\nhello,world");
        ISO_CHECK_EQ_UINT(rows.size(), 1u);
        ISO_CHECK(rows[0].at("name") == "hello");
        ISO_CHECK(rows[0].at("value") == "world");
    }

    // Quoted field with embedded comma / newline.
    {
        auto rows = csv::parse(
            "product,price,description\nWidget,9.99,\"A small, round widget\"\n");
        ISO_CHECK(rows[0].at("description") == "A small, round widget");
    }
    {
        auto rows = csv::parse("id,note\n1,\"Line one\nLine two\"\n2,Single line\n");
        ISO_CHECK_EQ_UINT(rows.size(), 2u);
        ISO_CHECK(rows[0].at("note") == "Line one\nLine two");
        ISO_CHECK(rows[1].at("note") == "Single line");
    }

    // Escaped double-quote.
    {
        auto rows = csv::parse("id,value\n1,\"She said \"\"hello\"\"\"\n2,plain\n");
        ISO_CHECK(rows[0].at("value") == "She said \"hello\"");
        ISO_CHECK(rows[1].at("value") == "plain");
    }

    // Empty quoted field / all-empty fields.
    {
        auto rows = csv::parse("a,b,c\n1,\"\",3\n");
        ISO_CHECK(rows[0].at("a") == "1");
        ISO_CHECK(rows[0].at("b") == "");
        ISO_CHECK(rows[0].at("c") == "3");
    }
    {
        auto rows = csv::parse("a,b,c\n,,\n");
        ISO_CHECK(rows[0].at("a") == "");
        ISO_CHECK(rows[0].at("b") == "");
        ISO_CHECK(rows[0].at("c") == "");
    }

    // Ragged rows.
    {
        auto rows = csv::parse("name,age,city\nAlice,30\n");
        ISO_CHECK_EQ_UINT(rows.size(), 1u);
        ISO_CHECK(rows[0].at("city") == ""); // padded
    }
    {
        auto rows = csv::parse("a,b,c\n1,2,3,4\n");
        ISO_CHECK(rows[0].at("c") == "3");
        ISO_CHECK_EQ_UINT(rows[0].size(), 3u); // extra dropped
    }

    // Empty / header-only files.
    {
        ISO_CHECK(csv::parse("").empty());
        ISO_CHECK(csv::parse("name,age,city\n").empty());
        ISO_CHECK(csv::parse("name,age").empty());
    }

    // Line endings: CRLF and lone CR.
    {
        auto rows = csv::parse("name,age\r\nAlice,30\r\nBob,25\r\n");
        ISO_CHECK_EQ_UINT(rows.size(), 2u);
        ISO_CHECK(rows[0].at("name") == "Alice");
        ISO_CHECK(rows[1].at("name") == "Bob");
    }
    {
        auto rows = csv::parse("name,age\rAlice,30\rBob,25\r");
        ISO_CHECK_EQ_UINT(rows.size(), 2u);
        ISO_CHECK(rows[1].at("name") == "Bob");
    }

    // Alternate delimiters.
    {
        auto rows = csv::parse("name\tage\nAlice\t30\nBob\t25\n", '\t');
        ISO_CHECK_EQ_UINT(rows.size(), 2u);
        ISO_CHECK(rows[0].at("age") == "30");
    }
    {
        auto rows = csv::parse("name;age;city\nAlice;30;Paris\n", ';');
        ISO_CHECK(rows[0].at("city") == "Paris");
    }
    {
        auto rows = csv::parse("a|b|c\n1|2|3\n", '|');
        ISO_CHECK(rows[0].at("b") == "2");
    }

    // Unclosed quote throws.
    {
        bool threw = false;
        try {
            csv::parse("name,value\n1,\"unclosed\n");
        } catch (const csv::UnclosedQuote&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // Raw grid preserves the header row and order.
    {
        csv::Grid g = csv::parse_records("name,age\nAda,36\nGrace,45\n");
        ISO_CHECK_EQ_UINT(g.size(), 3u);
        ISO_CHECK((g[0] == csv::Row{"name", "age"}));
        ISO_CHECK((g[2] == csv::Row{"Grace", "45"}));
    }

    // Multibyte UTF-8 content preserved.
    {
        auto rows = csv::parse("city,note\nM\xc3\xbcnchen,caf\xc3\xa9\n");
        ISO_CHECK(rows[0].at("city") == "M\xc3\xbcnchen");
        ISO_CHECK(rows[0].at("note") == "caf\xc3\xa9");
    }

    return ISO_TEST_RESULT();
}
