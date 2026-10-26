/* Linked against the installed archive/header, never src/bounded_json.c. */
#include <bounded_json.h>
#include <stdint.h>
#include <string.h>
int main(void) {
    const char *text = "[9007199254740991,1.0,null]";
    BJsonDoc *doc = NULL;
    int64_t integer = 0;
    if (bj_parse(text, strlen(text), &doc, NULL) != BJ_OK) return 1;
    if (!bj_integer(bj_at(bj_root(doc), 0), &integer) || integer != INT64_C(9007199254740991)) {
        bj_doc_free(&doc); return 2;
    }
    if (bj_kind(bj_at(bj_root(doc), 1)) != BJ_FLOAT || bj_kind(bj_at(bj_root(doc), 2)) != BJ_NULL) {
        bj_doc_free(&doc); return 3;
    }
    bj_doc_free(&doc);
    return 0;
}
