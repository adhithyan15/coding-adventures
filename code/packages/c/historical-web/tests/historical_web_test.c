#include "historical_web.h"
#include "bounded_json.h"
#include "fuzz.h"
#include "iso_test.h"
#include "pbt.h"
#include "test_util.h"

#include <stdint.h>
#include <stdlib.h>

static const BJsonValue *field(const BJsonValue *object, const char *name) {
    size_t i;
    for (i = 0; i < bj_size(object); i++) {
        const char *key;
        size_t n;
        const BJsonValue *value;
        if (bj_member(object, i, &key, &n, &value) && n == strlen(name) &&
            memcmp(key, name, n) == 0) return value;
    }
    return NULL;
}

static int json_string(const BJsonValue *value, const char **text, size_t *length) {
    return value != NULL && bj_string(value, text, length);
}

static int view_eq(HwebStringView view, const char *text) {
    size_t n = strlen(text);
    return view.length == n && (n == 0 || memcmp(view.data, text, n) == 0);
}

static int decode_hex(const char *hex, size_t hex_len, uint8_t **out, size_t *out_len) {
    size_t i;
    uint8_t *bytes;
    if ((hex_len & 1U) != 0) return 0;
    bytes = (uint8_t *)malloc(hex_len / 2 == 0 ? 1 : hex_len / 2);
    if (bytes == NULL) return 0;
    for (i = 0; i < hex_len; i += 2) {
        unsigned int a, b;
        if ((hex[i] >= '0' && hex[i] <= '9')) a = (unsigned int)(hex[i] - '0');
        else if (hex[i] >= 'a' && hex[i] <= 'f') a = (unsigned int)(hex[i] - 'a' + 10);
        else if (hex[i] >= 'A' && hex[i] <= 'F') a = (unsigned int)(hex[i] - 'A' + 10);
        else { free(bytes); return 0; }
        if (hex[i + 1] >= '0' && hex[i + 1] <= '9') b = (unsigned int)(hex[i + 1] - '0');
        else if (hex[i + 1] >= 'a' && hex[i + 1] <= 'f') b = (unsigned int)(hex[i + 1] - 'a' + 10);
        else if (hex[i + 1] >= 'A' && hex[i + 1] <= 'F') b = (unsigned int)(hex[i + 1] - 'A' + 10);
        else { free(bytes); return 0; }
        bytes[i / 2] = (uint8_t)((a << 4) | b);
    }
    *out = bytes;
    *out_len = hex_len / 2;
    return 1;
}

static const char *json_cstr(const BJsonValue *value, size_t *n) {
    const char *s = NULL;
    if (!json_string(value, &s, n) || memchr(s, '\0', *n) != NULL) return NULL;
    return s;
}

static int compare_expected_text(const BJsonValue *expected, const HwebNode *actual) {
    const char *s;
    size_t n;
    return actual->kind == HWEB_NODE_TEXT && json_string(expected, &s, &n) &&
           actual->text.length == n && (n == 0 || memcmp(actual->text.data, s, n) == 0);
}

static int compare_expected_node(const BJsonValue *expected, const HwebHtmlDoc *doc,
                                 size_t actual_index, size_t depth) {
    const BJsonValue *tag_value, *attrs, *children;
    const HwebNode *node = hweb_html_node_at(doc, actual_index);
    const char *tag;
    size_t tag_n, child_id, i, attribute_index;
    if (node == NULL || depth > 128) return 0;
    if (bj_kind(expected) == BJ_STRING) return compare_expected_text(expected, node);
    if (bj_kind(expected) != BJ_OBJECT || node->kind != HWEB_NODE_ELEMENT) return 0;
    tag_value = field(expected, "tag"); attrs = field(expected, "attributes"); children = field(expected, "children");
    if (!json_string(tag_value, &tag, &tag_n) || !attrs || !children || bj_kind(attrs) != BJ_OBJECT ||
        bj_kind(children) != BJ_ARRAY || node->tag.length != tag_n || memcmp(node->tag.data, tag, tag_n) != 0 ||
        node->attribute_count != bj_size(attrs) ||
        (node->attribute_count != 0 && node->first_attribute_index == HWEB_NO_INDEX)) return 0;
    attribute_index = node->first_attribute_index;
    for (i = 0; i < bj_size(attrs); i++) {
        const HwebAttribute *attribute;
        const char *name;
        size_t name_n;
        const BJsonValue *expected_value;
        if (!bj_member(attrs, i, &name, &name_n, &expected_value)) return 0;
        attribute = hweb_html_attribute_at(doc, attribute_index++);
        if (attribute == NULL || attribute->name.length != name_n || memcmp(attribute->name.data, name, name_n) != 0) return 0;
        if (bj_kind(expected_value) == BJ_NULL) {
            if (attribute->has_value) return 0;
        } else {
            const char *value; size_t value_n;
            if (!attribute->has_value || !json_string(expected_value, &value, &value_n) ||
                attribute->value.length != value_n || memcmp(attribute->value.data, value, value_n) != 0) return 0;
        }
    }
    child_id = node->first_child_index;
    for (i = 0; i < bj_size(children); i++) {
        if (child_id == HWEB_NO_INDEX || !compare_expected_node(bj_at(children, i), doc, child_id, depth + 1)) return 0;
        node = hweb_html_node_at(doc, child_id);
        child_id = node->next_sibling_index;
    }
    return child_id == HWEB_NO_INDEX;
}

static int compare_expected_tree(const BJsonValue *expected, const HwebHtmlDoc *doc) {
    size_t root = hweb_html_root_first(doc), i;
    if (expected == NULL || bj_kind(expected) != BJ_ARRAY) return 0;
    for (i = 0; i < bj_size(expected); i++) {
        if (root == HWEB_NO_INDEX || !compare_expected_node(bj_at(expected, i), doc, root, 0)) return 0;
        root = hweb_html_node_at(doc, root)->next_sibling_index;
    }
    return root == HWEB_NO_INDEX;
}

static int valid_utf8(const uint8_t *s, size_t n);

static void fixture_replay(void) {
    size_t length = 0, i;
    char *json = tu_read_repo_file("code/specs/fixtures/historical-web-v1/vectors.json", &length);
    BJsonDoc *fixture = NULL;
    BJsonError parse_error;
    const BJsonValue *http, *requests, *invalid_addresses, *invalid_requests, *responses, *invalid_responses, *html, *exchanges;
    ISO_CHECK_MSG(json != NULL, "shared HWEB fixture is required");
    if (json == NULL) return;
    ISO_CHECK(bj_parse(json, length, &fixture, &parse_error) == BJ_OK);
    free(json);
    if (fixture == NULL) return;
    http = field(bj_root(fixture), "http-0.9");
    requests = field(http, "requests"); invalid_addresses = field(http, "invalid_addresses");
    invalid_requests = field(http, "invalid_request_hex"); responses = field(http, "responses");
    invalid_responses = field(http, "invalid_response_hex");
    ISO_CHECK(http && requests && invalid_addresses && invalid_requests && responses && invalid_responses);
    for (i = 0; i < bj_size(requests); i++) {
        const BJsonValue *v = bj_at(requests, i); size_t address_n = 0, wire_hex_n = 0;
        const char *address = json_cstr(field(v, "address"), &address_n);
        const char *wire_hex = json_cstr(field(v, "wire_hex"), &wire_hex_n);
        uint8_t *accepted_wire = NULL; size_t accepted_wire_n = 0; uint8_t *canonical = NULL;
        size_t canonical_n = address_n + 6; HwebBytes encoded = {NULL, 0}, parsed = {NULL, 0}; HwebError error;
        ISO_CHECK(address != NULL && wire_hex != NULL && decode_hex(wire_hex, wire_hex_n, &accepted_wire, &accepted_wire_n));
        if (address == NULL || wire_hex == NULL || accepted_wire == NULL) { free(accepted_wire); continue; }
        canonical = (uint8_t *)malloc(canonical_n);
        if (canonical == NULL) { ISO_CHECK(0); free(accepted_wire); continue; }
        memcpy(canonical, "GET ", 4); memcpy(canonical + 4, address, address_n);
        canonical[canonical_n - 2] = '\r'; canonical[canonical_n - 1] = '\n';
        ISO_CHECK(hweb_http09_encode_request((const uint8_t *)address, address_n, &encoded, &error) == HWEB_OK);
        ISO_CHECK(encoded.length == canonical_n && memcmp(encoded.data, canonical, canonical_n) == 0);
        ISO_CHECK(hweb_http09_parse_request(accepted_wire, accepted_wire_n, &parsed, &error) == HWEB_OK);
        ISO_CHECK(parsed.length == address_n && memcmp(parsed.data, address, address_n) == 0);
        hweb_bytes_free(&encoded); hweb_bytes_free(&parsed); free(canonical); free(accepted_wire);
    }
    for (i = 0; i < bj_size(invalid_addresses); i++) {
        size_t n; const char *s = json_cstr(bj_at(invalid_addresses, i), &n);
        HwebBytes out = {NULL, 0}; HwebError error;
        ISO_CHECK(s != NULL);
        if (s != NULL) ISO_CHECK(hweb_http09_encode_request((const uint8_t *)s, n, &out, &error) == HWEB_ERR_PROTOCOL && out.data == NULL && out.length == 0);
    }
    for (i = 0; i < bj_size(invalid_requests); i++) {
        size_t n; const char *hex = json_cstr(bj_at(invalid_requests, i), &n); uint8_t *bytes = NULL; size_t bytes_n = 0;
        HwebBytes out = {NULL, 0}; HwebError error;
        ISO_CHECK(hex != NULL && decode_hex(hex, n, &bytes, &bytes_n));
        if (bytes != NULL) ISO_CHECK(hweb_http09_parse_request(bytes, bytes_n, &out, &error) == HWEB_ERR_PROTOCOL && out.data == NULL && out.length == 0);
        free(bytes);
    }
    for (i = 0; i < bj_size(responses); i++) {
        const BJsonValue *v = bj_at(responses, i); size_t hex_n, body_n;
        const char *hex = json_cstr(field(v, "wire_hex"), &hex_n), *body = json_cstr(field(v, "body"), &body_n);
        uint8_t *bytes = NULL; size_t bytes_n = 0; HwebBytes out = {NULL, 0}; HwebError error;
        ISO_CHECK(hex && body && decode_hex(hex, hex_n, &bytes, &bytes_n));
        if (hex && body && bytes != NULL) {
            ISO_CHECK(hweb_http09_decode_response(bytes, bytes_n, &out, &error) == HWEB_OK);
            ISO_CHECK(out.length == body_n && (body_n == 0 || memcmp(out.data, body, body_n) == 0));
        }
        hweb_bytes_free(&out); free(bytes);
    }
    for (i = 0; i < bj_size(invalid_responses); i++) {
        size_t n; const char *hex = json_cstr(bj_at(invalid_responses, i), &n); uint8_t *bytes = NULL; size_t bytes_n = 0;
        HwebBytes out = {NULL, 0}; HwebError error;
        ISO_CHECK(hex && decode_hex(hex, n, &bytes, &bytes_n));
        if (bytes != NULL) ISO_CHECK(hweb_http09_decode_response(bytes, bytes_n, &out, &error) == HWEB_ERR_PROTOCOL && out.data == NULL && out.length == 0);
        free(bytes);
    }
    html = field(bj_root(fixture), "html-tags-1992");
    ISO_CHECK(html != NULL && bj_kind(html) == BJ_ARRAY);
    for (i = 0; i < bj_size(html); i++) {
        const BJsonValue *v = bj_at(html, i); size_t source_n;
        const char *source = json_cstr(field(v, "source"), &source_n); HwebHtmlDoc *doc = NULL; HwebError error;
        ISO_CHECK(source != NULL);
        if (source != NULL) {
            ISO_CHECK(hweb_parse_html_tags_1992((const uint8_t *)source, source_n, &doc, &error) == HWEB_OK);
            ISO_CHECK(doc != NULL && compare_expected_tree(field(v, "tree"), doc));
        }
        hweb_html_doc_free(&doc);
    }
    exchanges = field(bj_root(fixture), "exchanges");
    ISO_CHECK(exchanges != NULL && bj_kind(exchanges) == BJ_ARRAY);
    for (i = 0; i < bj_size(exchanges); i++) {
        const BJsonValue *v = bj_at(exchanges, i); size_t address_n, request_hex_n, response_hex_n;
        const char *address = json_cstr(field(v, "address"), &address_n), *req_hex = json_cstr(field(v, "request_hex"), &request_hex_n);
        const char *res_hex = json_cstr(field(v, "response_hex"), &response_hex_n);
        uint8_t *req = NULL, *res = NULL; size_t req_n = 0, res_n = 0; HwebBytes encoded = {NULL,0}, body = {NULL,0}; HwebHtmlDoc *doc = NULL; HwebError error;
        ISO_CHECK(address && req_hex && res_hex && decode_hex(req_hex, request_hex_n, &req, &req_n) && decode_hex(res_hex, response_hex_n, &res, &res_n));
        if (address && req && res) {
            ISO_CHECK(hweb_http09_encode_request((const uint8_t *)address, address_n, &encoded, &error) == HWEB_OK);
            ISO_CHECK(encoded.length == req_n && memcmp(encoded.data, req, req_n) == 0);
            ISO_CHECK(hweb_http09_decode_response(res, res_n, &body, &error) == HWEB_OK);
            ISO_CHECK(hweb_parse_html_tags_1992(body.data, body.length, &doc, &error) == HWEB_OK);
        ISO_CHECK(doc && compare_expected_tree(field(v, "tree"), doc));
        }
        hweb_bytes_free(&encoded); hweb_bytes_free(&body); hweb_html_doc_free(&doc); free(req); free(res);
    }
    bj_doc_free(&fixture);
}

static int framing_oracle(const uint8_t *wire, size_t n, const uint8_t **address, size_t *address_n) {
    size_t end = n, i, terminator = 0;
    if (n < 5 || memcmp(wire, "GET ", 4) != 0) return 0;
    for (i = 4; i < n; i++) {
        if (wire[i] == '\n') { end = i; terminator = 1; break; }
        if (wire[i] == '\r') {
            if (i + 1 < n && wire[i + 1] == '\n') { end = i; terminator = 2; break; }
            return 0;
        }
    }
    if (terminator == 0 || end + terminator != n || end <= 4) return 0;
    for (i = 4; i < end; i++) if (wire[i] < 0x21 || wire[i] > 0x7e) return 0;
    *address = wire + 4; *address_n = end - 4;
    return 1;
}

static void exhaustive_http_short_wire(void) {
    unsigned int a, b, ending;
    size_t payload_length;
    size_t accepted = 0, rejected = 0;
    for (payload_length = 0; payload_length <= 2; payload_length++) {
        unsigned int a_limit = payload_length >= 1 ? 256U : 1U;
        unsigned int b_limit = payload_length >= 2 ? 256U : 1U;
        for (a = 0; a < a_limit; a++) for (b = 0; b < b_limit; b++) for (ending = 0; ending < 2; ending++) {
            uint8_t wire[9] = {'G','E','T',' ',0,0,'\n',0,0};
            size_t n, want_n = 0;
            const uint8_t *want = NULL; HwebBytes got = {NULL,0}; HwebError error;
            if (payload_length >= 1) wire[4] = (uint8_t)a;
            if (payload_length >= 2) wire[5] = (uint8_t)b;
            n = 4 + payload_length;
            if (ending == 0) wire[n++] = '\n';
            else { wire[n++] = '\r'; wire[n++] = '\n'; }
            if (framing_oracle(wire, n, &want, &want_n)) {
                accepted++;
                ISO_CHECK(hweb_http09_parse_request(wire, n, &got, &error) == HWEB_OK);
                if (got.data != NULL) ISO_CHECK(got.length == want_n && memcmp(got.data, want, want_n) == 0);
            } else {
                rejected++;
                ISO_CHECK(hweb_http09_parse_request(wire, n, &got, &error) == HWEB_ERR_PROTOCOL);
                ISO_CHECK(got.data == NULL && got.length == 0);
            }
            hweb_bytes_free(&got);
        }
    }
    ISO_CHECK_EQ_UINT(accepted + rejected, 131586);
}

static int arena_invariants(const HwebHtmlDoc *doc) {
    size_t n = hweb_html_node_count(doc), i, roots = hweb_html_root_first(doc), seen_roots = 0;
    uint8_t *seen;
    size_t *stack;
    size_t stack_len = 0;
    if (doc == NULL || (n != 0 && roots >= n)) return 0;
    seen = (uint8_t *)calloc(n == 0 ? 1 : n, 1);
    stack = (size_t *)malloc((n == 0 ? 1 : n) * sizeof *stack);
    if (seen == NULL || stack == NULL) { free(seen); free(stack); return 0; }
    for (i = 0; i < n; i++) {
        const HwebNode *node = hweb_html_node_at(doc, i);
        size_t child, steps = 0;
        if (node == NULL) { free(seen); free(stack); return 0; }
        if (node->kind == HWEB_NODE_TEXT) {
            if (node->tag.length != 0 || node->attribute_count != 0 || node->first_attribute_index != HWEB_NO_INDEX ||
                !valid_utf8((const uint8_t *)node->text.data, node->text.length)) { free(seen); free(stack); return 0; }
        } else if (node->kind == HWEB_NODE_ELEMENT) {
            if (node->text.length != 0 || node->tag.length == 0 ||
                !valid_utf8((const uint8_t *)node->tag.data, node->tag.length)) { free(seen); free(stack); return 0; }
            if (node->attribute_count != 0 && (node->first_attribute_index > hweb_html_attribute_count(doc) ||
                node->attribute_count > hweb_html_attribute_count(doc) - node->first_attribute_index)) { free(seen); free(stack); return 0; }
            for (child = 0; child < node->attribute_count; child++) {
                const HwebAttribute *attribute = hweb_html_attribute_at(doc, node->first_attribute_index + child);
                if (attribute == NULL || !valid_utf8((const uint8_t *)attribute->name.data, attribute->name.length) ||
                    (attribute->has_value && !valid_utf8((const uint8_t *)attribute->value.data, attribute->value.length))) {
                    free(seen); free(stack); return 0;
                }
            }
        } else { free(seen); free(stack); return 0; }
        if (node->parent_index != HWEB_NO_INDEX && node->parent_index >= n) { free(seen); free(stack); return 0; }
        child = node->first_child_index;
        while (child != HWEB_NO_INDEX) {
            const HwebNode *c = hweb_html_node_at(doc, child);
            if (c == NULL || c->parent_index != i || ++steps > n) { free(seen); free(stack); return 0; }
            child = c->next_sibling_index;
        }
    }
    while (roots != HWEB_NO_INDEX) {
        const HwebNode *root = hweb_html_node_at(doc, roots);
        if (root == NULL || root->parent_index != HWEB_NO_INDEX || ++seen_roots > n || seen[roots]) { free(seen); free(stack); return 0; }
        seen[roots] = 1;
        stack[stack_len++] = roots;
        roots = root->next_sibling_index;
    }
    while (stack_len != 0) {
        const HwebNode *node = hweb_html_node_at(doc, stack[--stack_len]);
        size_t child = node->first_child_index, steps = 0;
        while (child != HWEB_NO_INDEX) {
            const HwebNode *next = hweb_html_node_at(doc, child);
            if (next == NULL || seen[child] || stack_len >= n || ++steps > n) { free(seen); free(stack); return 0; }
            seen[child] = 1;
            stack[stack_len++] = child;
            child = next->next_sibling_index;
        }
    }
    for (i = 0; i < n; i++) if (!seen[i]) { free(seen); free(stack); return 0; }
    free(seen); free(stack); return 1;
}

static void ownership_and_boundaries(void) {
    static const uint8_t nul_html[] = {'a',0,'b'};
    static const uint8_t response[] = {0x41,0x00,0x7f};
    uint8_t owned_source[] = "<A HREF='original'>text</A>";
    uint8_t address[] = {'/','o','k'}; uint8_t original_address[] = {'/','o','k'};
    uint8_t canary[12]; HwebBytes bytes = {NULL,0}; HwebHtmlDoc *doc = NULL; HwebError error;
    memset(canary, 0xa5, sizeof canary);
    ISO_CHECK(hweb_http09_encode_request(address, sizeof address, &bytes, &error) == HWEB_OK);
    ISO_CHECK(memcmp(address, original_address, sizeof address) == 0);
    memset(address, 'x', sizeof address);
    ISO_CHECK(bytes.length == 9 && memcmp(bytes.data, "GET /ok\r\n", 9) == 0);
    hweb_bytes_free(&bytes); ISO_CHECK(bytes.data == NULL && bytes.length == 0);
    hweb_bytes_free(&bytes); ISO_CHECK(bytes.data == NULL && bytes.length == 0);
    hweb_bytes_free(NULL);
    ISO_CHECK(hweb_http09_encode_request((const uint8_t *)"/ok", 3, NULL, &error) == HWEB_ERR_ARGUMENT);
    ISO_CHECK(error.status == HWEB_ERR_ARGUMENT && error.byte_offset == HWEB_NO_INDEX);
    ISO_CHECK(hweb_http09_parse_request((const uint8_t *)"GET /\n", 6, NULL, &error) == HWEB_ERR_ARGUMENT);
    ISO_CHECK(hweb_http09_encode_request(NULL, 1, &bytes, &error) == HWEB_ERR_ARGUMENT);
    ISO_CHECK(bytes.data == NULL && bytes.length == 0 && error.byte_offset == HWEB_NO_INDEX);
    ISO_CHECK(hweb_http09_parse_request(NULL, 1, &bytes, &error) == HWEB_ERR_ARGUMENT);
    ISO_CHECK(bytes.data == NULL && bytes.length == 0);
    ISO_CHECK(hweb_http09_parse_request(NULL, 0, &bytes, &error) == HWEB_ERR_PROTOCOL);
    ISO_CHECK(bytes.data == NULL && bytes.length == 0);
    ISO_CHECK(hweb_http09_decode_response(response, sizeof response, NULL, &error) == HWEB_ERR_ARGUMENT);
    ISO_CHECK(hweb_http09_decode_response(NULL, 1, &bytes, &error) == HWEB_ERR_ARGUMENT);
    ISO_CHECK(bytes.data == NULL && bytes.length == 0);
    ISO_CHECK(hweb_http09_decode_response(NULL, 0, &bytes, &error) == HWEB_OK);
    ISO_CHECK(bytes.data == NULL && bytes.length == 0);
    ISO_CHECK(hweb_http09_encode_request((const uint8_t *)"/bad address", 12, &bytes, &error) == HWEB_ERR_PROTOCOL);
    ISO_CHECK(bytes.data == NULL && bytes.length == 0 && error.status == HWEB_ERR_PROTOCOL && error.byte_offset == 4);
    ISO_CHECK(hweb_http09_decode_response(response, sizeof response, &bytes, &error) == HWEB_OK);
    ISO_CHECK(bytes.length == sizeof response && memcmp(bytes.data, response, sizeof response) == 0);
    hweb_bytes_free(&bytes);
    ISO_CHECK(hweb_http09_decode_response((const uint8_t *)"\x80", 1, &bytes, &error) == HWEB_ERR_PROTOCOL);
    ISO_CHECK(bytes.data == NULL && bytes.length == 0 && error.byte_offset == 0);
    bytes.data = (uint8_t *)malloc(1); bytes.length = 1;
    if (bytes.data != NULL) {
        uint8_t *owned = bytes.data; bytes.data[0] = 0x5a;
        ISO_CHECK(hweb_http09_encode_request((const uint8_t *)"/ok", 3, &bytes, &error) == HWEB_ERR_ARGUMENT);
        ISO_CHECK(bytes.data == owned && bytes.length == 1 && bytes.data[0] == 0x5a);
        ISO_CHECK(hweb_http09_parse_request((const uint8_t *)"GET /\n", 6, &bytes, &error) == HWEB_ERR_ARGUMENT);
        ISO_CHECK(bytes.data == owned && bytes.length == 1 && bytes.data[0] == 0x5a);
        ISO_CHECK(hweb_http09_decode_response((const uint8_t *)"A", 1, &bytes, &error) == HWEB_ERR_ARGUMENT);
        ISO_CHECK(bytes.data == owned && bytes.length == 1 && bytes.data[0] == 0x5a);
        hweb_bytes_free(&bytes);
    } else ISO_CHECK(0);
    ISO_CHECK(hweb_parse_html_tags_1992((const uint8_t *)"<P>", 3, NULL, &error) == HWEB_ERR_ARGUMENT);
    ISO_CHECK(error.status == HWEB_ERR_ARGUMENT && error.byte_offset == HWEB_NO_INDEX);
    ISO_CHECK(hweb_parse_html_tags_1992(NULL, 1, &doc, &error) == HWEB_ERR_ARGUMENT);
    ISO_CHECK(doc == NULL && error.byte_offset == HWEB_NO_INDEX);
    ISO_CHECK(hweb_parse_html_tags_1992(NULL, 0, &doc, &error) == HWEB_OK);
    ISO_CHECK(doc != NULL && hweb_html_node_count(doc) == 0 && hweb_html_root_first(doc) == HWEB_NO_INDEX);
    hweb_html_doc_free(&doc);
    ISO_CHECK(hweb_parse_html_tags_1992(nul_html, sizeof nul_html, &doc, &error) == HWEB_OK);
    ISO_CHECK(doc != NULL && arena_invariants(doc) && hweb_html_node_count(doc) == 1);
    if (doc != NULL) {
        const HwebNode *n = hweb_html_node_at(doc, 0);
        ISO_CHECK(n && n->kind == HWEB_NODE_TEXT && n->text.length == 3 && memcmp(n->text.data, nul_html, 3) == 0);
        ISO_CHECK(hweb_html_node_at(doc, 1) == NULL);
        ISO_CHECK(hweb_html_attribute_at(doc, 0) == NULL);
    }
    if (doc != NULL) {
        HwebHtmlDoc *owned = doc;
        ISO_CHECK(hweb_parse_html_tags_1992(NULL, 1, &doc, &error) == HWEB_ERR_ARGUMENT);
        ISO_CHECK(doc == owned && arena_invariants(doc));
        ISO_CHECK(hweb_parse_html_tags_1992((const uint8_t *)"<P>", 3, &doc, &error) == HWEB_ERR_ARGUMENT);
        ISO_CHECK(doc == owned && arena_invariants(doc));
    }
    hweb_html_doc_free(&doc); hweb_html_doc_free(&doc); ISO_CHECK(doc == NULL);
    hweb_html_doc_free(NULL);
    ISO_CHECK(hweb_html_node_count(NULL) == 0 && hweb_html_attribute_count(NULL) == 0);
    ISO_CHECK(hweb_html_node_at(NULL, 0) == NULL && hweb_html_attribute_at(NULL, 0) == NULL);
    ISO_CHECK(hweb_parse_html_tags_1992(owned_source, sizeof owned_source - 1, &doc, &error) == HWEB_OK);
    memset(owned_source, 'x', sizeof owned_source - 1);
    if (doc != NULL) {
        const HwebNode *anchor = hweb_html_node_at(doc, hweb_html_root_first(doc));
        const HwebAttribute *attribute = anchor ? hweb_html_attribute_at(doc, anchor->first_attribute_index) : NULL;
        const HwebNode *text = anchor ? hweb_html_node_at(doc, anchor->first_child_index) : NULL;
        ISO_CHECK(anchor && view_eq(anchor->tag, "a"));
        ISO_CHECK(attribute && view_eq(attribute->name, "href") && attribute->has_value && view_eq(attribute->value, "original"));
        ISO_CHECK(text && text->kind == HWEB_NODE_TEXT && view_eq(text->text, "text"));
    }
    hweb_html_doc_free(&doc);
    ISO_CHECK(memcmp(canary, "\xa5\xa5\xa5\xa5\xa5\xa5\xa5\xa5\xa5\xa5\xa5\xa5", sizeof canary) == 0);
}

static void utf8_boundaries_and_malformed_attributes(void) {
    static const uint8_t valid[] = {'a',0xe2,0x82,0xac,0xf0,0x9f,0x98,0x80,'z'};
    static const struct { uint8_t bytes[4]; size_t length; size_t offset; } invalid[] = {
        {{0xe2,0x82,0,0}, 2, 2},       /* truncated three-byte scalar */
        {{0xf0,0x9f,0,0}, 2, 2},       /* four-byte scalar missing byte 3 */
        {{0xf0,0x9f,0x98,0}, 3, 3},    /* four-byte scalar missing byte 4 */
        {{0xe2,'A',0,0}, 2, 1},         /* bad continuation 2 */
        {{0xe2,0x82,'A',0}, 3, 2},      /* bad continuation 3 */
        {{0xf0,0x9f,'A',0x80}, 4, 2},   /* bad continuation 3 */
        {{0xf0,0x9f,0x98,'A'}, 4, 3}    /* bad continuation 4 */
    };
    static const char *const malformed[] = {"<A HREF=>text</A>", "<A HREF==x>text</A>"};
    size_t i;
    HwebHtmlDoc *doc = NULL; HwebError error;
    ISO_CHECK(hweb_parse_html_tags_1992(valid, sizeof valid, &doc, &error) == HWEB_OK);
    ISO_CHECK(doc != NULL && hweb_html_node_count(doc) == 1 && arena_invariants(doc));
    if (doc != NULL) {
        const HwebNode *text = hweb_html_node_at(doc, 0);
        ISO_CHECK(text && text->kind == HWEB_NODE_TEXT && text->text.length == sizeof valid &&
                  memcmp(text->text.data, valid, sizeof valid) == 0);
    }
    hweb_html_doc_free(&doc);
    for (i = 0; i < sizeof invalid / sizeof invalid[0]; i++) {
        doc = NULL;
        ISO_CHECK(hweb_parse_html_tags_1992(invalid[i].bytes, invalid[i].length, &doc, &error) == HWEB_ERR_UTF8);
        ISO_CHECK(doc == NULL && error.status == HWEB_ERR_UTF8 && error.byte_offset == invalid[i].offset);
    }
    for (i = 0; i < sizeof malformed / sizeof malformed[0]; i++) {
        const HwebNode *anchor, *text;
        const HwebAttribute *attribute;
        doc = NULL;
        ISO_CHECK(hweb_parse_html_tags_1992((const uint8_t *)malformed[i], strlen(malformed[i]), &doc, &error) == HWEB_OK);
        ISO_CHECK(doc != NULL && arena_invariants(doc) && hweb_html_node_count(doc) == 2);
        anchor = doc ? hweb_html_node_at(doc, hweb_html_root_first(doc)) : NULL;
        attribute = anchor ? hweb_html_attribute_at(doc, anchor->first_attribute_index) : NULL;
        text = anchor ? hweb_html_node_at(doc, anchor->first_child_index) : NULL;
        ISO_CHECK(anchor && anchor->kind == HWEB_NODE_ELEMENT && view_eq(anchor->tag, "a") && anchor->attribute_count == 1);
        ISO_CHECK(attribute && view_eq(attribute->name, "href") && !attribute->has_value && attribute->value.length == 0);
        ISO_CHECK(text && text->kind == HWEB_NODE_TEXT && view_eq(text->text, "text"));
        hweb_html_doc_free(&doc);
    }
}

static void deep_and_raw_utf8(void) {
    static const uint8_t raw[] = "<XMP>aaaaa\xc3\xa9</xMp><P>after</P>";
    const size_t nesting = 3000, open_n = sizeof("<ADDRESS>") - 1, close_n = sizeof("</ADDRESS>") - 1;
    size_t i, length = nesting * (open_n + close_n) + 30, cursor = 0, depth = 0;
    uint8_t *source = (uint8_t *)malloc(length); HwebHtmlDoc *doc = NULL; HwebError error;
    ISO_CHECK(source != NULL);
    if (source == NULL) return;
    for (i = 0; i < nesting; i++) { memcpy(source + cursor, "<ADDRESS>", open_n); cursor += open_n; }
    memcpy(source + cursor, "</A></A>plain", 13); cursor += 13;
    for (i = 0; i < nesting; i++) { memcpy(source + cursor, "</ADDRESS>", close_n); cursor += close_n; }
    ISO_CHECK(cursor <= length);
    ISO_CHECK(hweb_parse_html_tags_1992(source, cursor, &doc, &error) == HWEB_OK);
    ISO_CHECK(doc != NULL && arena_invariants(doc) && hweb_html_node_count(doc) == nesting + 1);
    if (doc != NULL) {
        size_t node = hweb_html_root_first(doc);
        while (node != HWEB_NO_INDEX && depth < nesting) {
            const HwebNode *current = hweb_html_node_at(doc, node);
            if (current == NULL || current->kind != HWEB_NODE_ELEMENT || !view_eq(current->tag, "address")) break;
            node = current->first_child_index; depth++;
        }
        ISO_CHECK_EQ_UINT(depth, nesting);
        if (node != HWEB_NO_INDEX) {
            const HwebNode *text = hweb_html_node_at(doc, node);
            ISO_CHECK(text && text->kind == HWEB_NODE_TEXT && text->text.length == 5 && memcmp(text->text.data, "plain", 5) == 0);
        }
    }
    hweb_html_doc_free(&doc); free(source);
    ISO_CHECK(hweb_parse_html_tags_1992(raw, sizeof(raw) - 1, &doc, &error) == HWEB_OK);
    ISO_CHECK(doc != NULL && arena_invariants(doc));
    if (doc != NULL) {
        const HwebNode *xmp = hweb_html_node_at(doc, hweb_html_root_first(doc));
        const HwebNode *text = xmp ? hweb_html_node_at(doc, xmp->first_child_index) : NULL;
        const HwebNode *p = xmp ? hweb_html_node_at(doc, xmp->next_sibling_index) : NULL;
        const HwebNode *following = p ? hweb_html_node_at(doc, p->next_sibling_index) : NULL;
        ISO_CHECK(xmp && view_eq(xmp->tag, "xmp") && text && text->kind == HWEB_NODE_TEXT);
        ISO_CHECK(text && text->text.length == 7 && memcmp(text->text.data, "aaaaa\xc3\xa9", 7) == 0);
        ISO_CHECK(p && view_eq(p->tag, "p") && p->first_child_index == HWEB_NO_INDEX);
        ISO_CHECK(following && following->kind == HWEB_NODE_TEXT && following->text.length == 5 && memcmp(following->text.data, "after", 5) == 0);
    }
    hweb_html_doc_free(&doc);
}

static int prop_http_matches_oracle(void *user, pbt_budget *scope, const pbt_val *value) {
    const uint8_t *data; size_t length, expected_n = 0; const uint8_t *expected = NULL;
    HwebBytes parsed = {NULL,0}, encoded = {NULL,0}; HwebError error; HwebStatus status; int holds;
    (void)user; (void)scope;
    if (value == NULL || value->kind != PBT_V_BYTES) return -1;
    data = value->u.bytes.p; length = value->u.bytes.n;
    status = hweb_http09_parse_request(data, length, &parsed, &error);
    if (framing_oracle(data, length, &expected, &expected_n)) {
        if (status != HWEB_OK || parsed.data == NULL || parsed.length != expected_n || memcmp(parsed.data, expected, expected_n) != 0) holds = 0;
        else if (hweb_http09_encode_request(parsed.data, parsed.length, &encoded, &error) != HWEB_OK) holds = 0;
        else holds = encoded.length == parsed.length + 6 && memcmp(encoded.data, "GET ", 4) == 0 &&
                     memcmp(encoded.data + 4, parsed.data, parsed.length) == 0 &&
                     encoded.data[encoded.length - 2] == '\r' && encoded.data[encoded.length - 1] == '\n';
    } else holds = status == HWEB_ERR_PROTOCOL && parsed.data == NULL && parsed.length == 0;
    hweb_bytes_free(&parsed); hweb_bytes_free(&encoded);
    return holds;
}

static int valid_utf8(const uint8_t *s, size_t n) {
    size_t i = 0;
    while (i < n) {
        uint8_t a = s[i++]; uint32_t cp; size_t extra, j;
        if (a <= 0x7f) continue;
        if (a >= 0xc2 && a <= 0xdf) { cp = a & 0x1fU; extra = 1; }
        else if (a >= 0xe0 && a <= 0xef) { cp = a & 0x0fU; extra = 2; }
        else if (a >= 0xf0 && a <= 0xf4) { cp = a & 0x07U; extra = 3; }
        else return 0;
        if (extra > n - i) return 0;
        for (j = 0; j < extra; j++) {
            uint8_t b = s[i++];
            if ((b & 0xc0U) != 0x80U) return 0;
            cp = (cp << 6) | (uint32_t)(b & 0x3fU);
        }
        if ((extra == 1 && cp < 0x80) || (extra == 2 && cp < 0x800) || (extra == 3 && cp < 0x10000) ||
            cp > 0x10ffff || (cp >= 0xd800 && cp <= 0xdfff)) return 0;
    }
    return 1;
}

static int fuzz_target(const uint8_t *data, size_t length, fz_handle *handle, fz_crash *crash) {
    const uint8_t *expected = NULL; size_t expected_n = 0; HwebBytes parsed = {NULL,0}; HwebError error;
    HwebStatus status; HwebHtmlDoc *doc = NULL; int ok = 1;
    fz_edge(handle, fz_edge_id("hweb.c.http.parse"));
    if (length != 0 && data[0] == 'G') fz_edge(handle, fz_edge_id("hweb.c.http.get-prefix"));
    status = hweb_http09_parse_request(data, length, &parsed, &error);
    if (framing_oracle(data, length, &expected, &expected_n))
        ok = status == HWEB_OK && parsed.data != NULL && parsed.length == expected_n && memcmp(parsed.data, expected, expected_n) == 0;
    else ok = status == HWEB_ERR_PROTOCOL && parsed.data == NULL && parsed.length == 0;
    hweb_bytes_free(&parsed);
    if (!ok) goto fail;
    fz_edge(handle, fz_edge_id("hweb.c.html.parse"));
    status = hweb_parse_html_tags_1992(data, length, &doc, &error);
    if (valid_utf8(data, length)) {
        ok = status == HWEB_OK && doc != NULL && arena_invariants(doc);
        if (length >= 4 && memcmp(data, "<XMP", 4) == 0) fz_edge(handle, fz_edge_id("hweb.c.html.raw-candidate"));
    } else ok = status == HWEB_ERR_UTF8 && doc == NULL;
    hweb_html_doc_free(&doc);
    if (!ok) goto fail;
    return 0;
fail:
    fz_crash_set(crash, "hweb-oracle-mismatch");
    fz_crash_push(crash, "fuzz_target");
    return 1;
}

static void property_and_fuzz(void) {
    static const uint8_t valid_lf[] = {'G','E','T',' ','/','\n'};
    static const uint8_t valid_crlf[] = {'G','E','T',' ','/','\r','\n'};
    static const uint8_t invalid[] = {'G','E','T',' ',' ','/','\n'};
    pbt_gen *arms[4], *random_bytes, *generator; pbt_val constant; pbt_verdict verdict;
    pbt_options pbt_opts = {500, 100, PBT_CI_OFF};
    size_t i;
    fz_input seeds[] = {
        {(const uint8_t *)"GET /\r\n", 7}, {(const uint8_t *)"GET /x\n", 7},
        {(const uint8_t *)"GET  /\n", 8}, {(const uint8_t *)"<XMP>a</xmp>", 12},
        {(const uint8_t *)"<A HREF='x>y'>z</A>", 19}, {(const uint8_t *)"<UL><LI>a<LI>b</UL>", 19},
        {(const uint8_t *)"<\xf0\x9f\x98\x80", 5}
    };
    fz_options fopts = fz_options_default(); fz_report report;
    random_bytes = pbt_gen_bytes(0, 32);
    pbt_val_nil(&constant); ISO_CHECK(pbt_val_set_bytes(&constant, valid_lf, sizeof valid_lf) == 0); arms[0] = pbt_gen_constant(&constant);
    pbt_val_free(&constant);
    pbt_val_nil(&constant); ISO_CHECK(pbt_val_set_bytes(&constant, valid_crlf, sizeof valid_crlf) == 0); arms[1] = pbt_gen_constant(&constant);
    pbt_val_free(&constant);
    pbt_val_nil(&constant); ISO_CHECK(pbt_val_set_bytes(&constant, invalid, sizeof invalid) == 0); arms[2] = pbt_gen_constant(&constant);
    pbt_val_free(&constant); arms[3] = random_bytes;
    ISO_CHECK(random_bytes != NULL && arms[0] != NULL && arms[1] != NULL && arms[2] != NULL);
    generator = pbt_gen_one_of(arms, 4);
    ISO_CHECK(generator != NULL);
    if (generator != NULL) {
        pbt_verdict_init(&verdict);
        ISO_CHECK(pbt_check(NULL, UINT64_C(0x485745423037), generator,
                            (pbt_prop){prop_http_matches_oracle, NULL}, &pbt_opts, NULL, &verdict) == PBT_OK);
        ISO_CHECK(verdict.passed && verdict.cases_run == pbt_opts.num_tests);
        pbt_verdict_free(&verdict); pbt_gen_release(generator);
    }
    for (i = 0; i < sizeof arms / sizeof arms[0]; i++) if (arms[i] != NULL) pbt_gen_release(arms[i]);
    fopts.seed = UINT64_C(0x485745423037); fopts.max_iterations = 600; fopts.max_input_len = 128;
    fopts.base_energy = 24; fopts.minimize_budget = 100; fopts.step_budget = UINT64_C(2000000); fopts.mem_budget = (size_t)8 << 20;
    fz_report_init(&report);
    ISO_CHECK(fz_run(seeds, sizeof seeds / sizeof seeds[0], &fopts, fuzz_target, &report) == FZ_OK);
    ISO_CHECK(fz_report_is_clean(&report));
    ISO_CHECK(report.iterations == fopts.max_iterations && report.timeouts == 0 && report.ooms == 0);
    ISO_CHECK(report.tier != NULL && strcmp(report.tier, FZ_TIER_CALLER) == 0);
    fz_report_free(&report);
}

int main(void) {
    fixture_replay();
    exhaustive_http_short_wire();
    ownership_and_boundaries();
    utf8_boundaries_and_malformed_attributes();
    deep_and_raw_utf8();
    property_and_fuzz();
    return ISO_TEST_RESULT();
}
