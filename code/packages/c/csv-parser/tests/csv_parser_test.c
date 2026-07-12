/* Tests for the C csv-parser, using the iso_test.h harness. Cases are taken
 * from the Rust crate's own tests (RFC 4180 behaviours). */
#include "iso_test.h"

#include <string.h> /* strcmp */

#include "csv_parser.h"

/* Assert that data row `r` column `col` of a parsed table equals `expected`. */
static void check_get(const CsvTable *t, size_t r, const char *col,
                      const char *expected) {
    const char *v = csv_table_get(t, r, col);
    ISO_CHECK(v != NULL);
    if (v) {
        ISO_CHECK_STR_EQ(v, expected);
    }
}

int main(void) {
    /* Simple three-column table. */
    {
        CsvTable t;
        ISO_CHECK_EQ_INT(
            (int)csv_parse("name,age,city\nAlice,30,New York\nBob,25,London\n",
                           &t),
            (int)CSV_OK);
        ISO_CHECK_EQ_UINT(t.row_count, 2u);
        check_get(&t, 0, "name", "Alice");
        check_get(&t, 0, "age", "30");
        check_get(&t, 0, "city", "New York");
        check_get(&t, 1, "name", "Bob");
        check_get(&t, 1, "city", "London");
        ISO_CHECK(csv_table_get(&t, 0, "nope") == NULL); /* no such column */
        ISO_CHECK(csv_table_get(&t, 5, "name") == NULL); /* no such row */
        csv_table_free(&t);
    }

    /* No trailing newline (last record may omit it). */
    {
        CsvTable t;
        ISO_CHECK_EQ_INT((int)csv_parse("name,value\nhello,world", &t),
                         (int)CSV_OK);
        ISO_CHECK_EQ_UINT(t.row_count, 1u);
        check_get(&t, 0, "name", "hello");
        check_get(&t, 0, "value", "world");
        csv_table_free(&t);
    }

    /* Quoted field with an embedded comma. */
    {
        CsvTable t;
        csv_parse("product,price,description\n"
                  "Widget,9.99,\"A small, round widget\"\n",
                  &t);
        check_get(&t, 0, "description", "A small, round widget");
        csv_table_free(&t);
    }

    /* Quoted field with an embedded newline. */
    {
        CsvTable t;
        csv_parse("id,note\n1,\"Line one\nLine two\"\n2,Single line\n", &t);
        ISO_CHECK_EQ_UINT(t.row_count, 2u);
        check_get(&t, 0, "note", "Line one\nLine two");
        check_get(&t, 1, "note", "Single line");
        csv_table_free(&t);
    }

    /* Escaped double-quote "" -> ". */
    {
        CsvTable t;
        csv_parse("id,value\n1,\"She said \"\"hello\"\"\"\n2,plain\n", &t);
        check_get(&t, 0, "value", "She said \"hello\"");
        check_get(&t, 1, "value", "plain");
        csv_table_free(&t);
    }

    /* Empty quoted field, and empty middle fields. */
    {
        CsvTable t;
        csv_parse("a,b,c\n1,\"\",3\n", &t);
        check_get(&t, 0, "a", "1");
        check_get(&t, 0, "b", "");
        check_get(&t, 0, "c", "3");
        csv_table_free(&t);
    }
    {
        CsvTable t;
        csv_parse("a,b,c\n,,\n", &t); /* all empty */
        check_get(&t, 0, "a", "");
        check_get(&t, 0, "b", "");
        check_get(&t, 0, "c", "");
        csv_table_free(&t);
    }

    /* Ragged rows: short padded with "", long truncated. */
    {
        CsvTable t;
        csv_parse("name,age,city\nAlice,30\n", &t);
        ISO_CHECK_EQ_UINT(t.row_count, 1u);
        check_get(&t, 0, "name", "Alice");
        check_get(&t, 0, "age", "30");
        check_get(&t, 0, "city", ""); /* padded */
        csv_table_free(&t);
    }
    {
        CsvTable t;
        csv_parse("a,b,c\n1,2,3,4\n", &t); /* extra field dropped */
        check_get(&t, 0, "a", "1");
        check_get(&t, 0, "c", "3");
        csv_table_free(&t);
    }

    /* Empty file and header-only file -> zero data rows. */
    {
        CsvTable t;
        csv_parse("", &t);
        ISO_CHECK_EQ_UINT(t.row_count, 0u);
        csv_table_free(&t);
    }
    {
        CsvTable t;
        csv_parse("name,age,city\n", &t);
        ISO_CHECK_EQ_UINT(t.row_count, 0u);
        csv_table_free(&t);
    }
    {
        CsvTable t;
        csv_parse("name,age", &t); /* header-only, no trailing newline */
        ISO_CHECK_EQ_UINT(t.row_count, 0u);
        csv_table_free(&t);
    }

    /* Line endings: CRLF and lone CR both work. */
    {
        CsvTable t;
        csv_parse("name,age\r\nAlice,30\r\nBob,25\r\n", &t);
        ISO_CHECK_EQ_UINT(t.row_count, 2u);
        check_get(&t, 0, "name", "Alice");
        check_get(&t, 1, "name", "Bob");
        csv_table_free(&t);
    }
    {
        CsvTable t;
        csv_parse("name,age\rAlice,30\rBob,25\r", &t);
        ISO_CHECK_EQ_UINT(t.row_count, 2u);
        check_get(&t, 0, "name", "Alice");
        check_get(&t, 1, "name", "Bob");
        csv_table_free(&t);
    }

    /* Alternate delimiters: TSV, semicolon, pipe. */
    {
        CsvTable t;
        csv_parse_with_delimiter("name\tage\nAlice\t30\nBob\t25\n", '\t', &t);
        ISO_CHECK_EQ_UINT(t.row_count, 2u);
        check_get(&t, 0, "name", "Alice");
        check_get(&t, 0, "age", "30");
        csv_table_free(&t);
    }
    {
        CsvTable t;
        csv_parse_with_delimiter("name;age;city\nAlice;30;Paris\n", ';', &t);
        check_get(&t, 0, "city", "Paris");
        csv_table_free(&t);
    }
    {
        CsvTable t;
        csv_parse_with_delimiter("a|b|c\n1|2|3\n", '|', &t);
        check_get(&t, 0, "b", "2");
        csv_table_free(&t);
    }

    /* Unclosed quote is an error. */
    {
        CsvTable t;
        ISO_CHECK_EQ_INT((int)csv_parse("name,value\n1,\"unclosed\n", &t),
                         (int)CSV_ERR_UNCLOSED_QUOTE);
        /* nothing to free: on error the table is zeroed */
    }

    /* Raw grid keeps the header row and column order. */
    {
        CsvGrid g;
        ISO_CHECK_EQ_INT(
            (int)csv_parse_records("name,age\nAda,36\nGrace,45\n", ',', &g),
            (int)CSV_OK);
        ISO_CHECK_EQ_UINT(g.count, 3u);
        ISO_CHECK_EQ_UINT(g.rows[0].count, 2u);
        ISO_CHECK_STR_EQ(g.rows[0].fields[0], "name");
        ISO_CHECK_STR_EQ(g.rows[0].fields[1], "age");
        ISO_CHECK_STR_EQ(g.rows[2].fields[0], "Grace");
        ISO_CHECK_STR_EQ(g.rows[2].fields[1], "45");
        csv_grid_free(&g);
    }

    /* Multibyte UTF-8 content is preserved verbatim. */
    {
        CsvTable t;
        csv_parse("city,note\nM\xc3\xbcnchen,caf\xc3\xa9\n", &t); /* München, café */
        check_get(&t, 0, "city", "M\xc3\xbcnchen");
        check_get(&t, 0, "note", "caf\xc3\xa9");
        csv_table_free(&t);
    }

    return ISO_TEST_RESULT();
}
