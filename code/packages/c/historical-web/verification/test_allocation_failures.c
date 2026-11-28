/*
 * Opt-in host allocation-failure sweep for the public HTTP and HTML APIs.
 *
 * From code/packages/c/historical-web after both source files are present:
 *
 *   gcc -std=c17 -pedantic-errors -Wall -Wextra -Werror \
 *     -fsanitize=address,undefined -fno-omit-frame-pointer -Iinclude \
 *     -Wl,--wrap=malloc -Wl,--wrap=calloc -Wl,--wrap=realloc \
 *     src/http09.c src/html_tags_1992.c verification/test_allocation_failures.c \
 *     -o /tmp/hweb-allocation-failures
 *   ASAN_OPTIONS=detect_leaks=1 /tmp/hweb-allocation-failures
 *
 * GNU ld wraps only the named allocation calls. The wrapper fails one call at
 * a time after a successful control run; it does not manufacture invalid
 * pointers or alter production code. This is optional host verification, not
 * a dependency of the ISO C17 or Windows package builds.
 */
#include "historical_web.h"

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void *__real_malloc(size_t size);
void *__real_calloc(size_t count, size_t size);
void *__real_realloc(void *pointer, size_t size);

static int allocation_failure_armed;
static size_t allocation_calls;
static size_t allocation_to_fail;

void *__wrap_malloc(size_t size) {
    if (allocation_failure_armed && ++allocation_calls == allocation_to_fail) {
        return NULL;
    }
    return __real_malloc(size);
}

void *__wrap_calloc(size_t count, size_t size) {
    if (allocation_failure_armed && ++allocation_calls == allocation_to_fail) {
        return NULL;
    }
    return __real_calloc(count, size);
}

void *__wrap_realloc(void *pointer, size_t size) {
    if (allocation_failure_armed && ++allocation_calls == allocation_to_fail) {
        return NULL;
    }
    return __real_realloc(pointer, size);
}

typedef enum {
    HTTP_ENCODE,
    HTTP_PARSE,
    HTTP_RESPONSE,
    HTML_PARSE
} Scenario;

typedef struct {
    size_t controls;
    size_t failures;
} SweepResult;

static HwebStatus run_scenario(Scenario scenario, HwebBytes *bytes,
                               HwebHtmlDoc **document, HwebError *error) {
    static const uint8_t address[] = "/allocation-sweep";
    static const uint8_t request[] = {
        'G', 'E', 'T', ' ', '/', 'a', 'l', 'l', 'o', 'c', 'a', 't', 'i',
        'o', 'n', '-', 's', 'w', 'e', 'e', 'p', '\r', '\n'};
    static const uint8_t body[] = {'b', 'o', 'd', 'y', 0, '\n'};
    static const uint8_t html[] =
        "<address>left<a href='/x&amp;y' name=dest>middle</a>"
        "<xmp>raw &amp;\r\nstill raw</xmp></address>";

    switch (scenario) {
    case HTTP_ENCODE:
        return hweb_http09_encode_request(address, sizeof(address) - 1u,
                                          bytes, error);
    case HTTP_PARSE:
        return hweb_http09_parse_request(request, sizeof(request), bytes,
                                         error);
    case HTTP_RESPONSE:
        return hweb_http09_decode_response(body, sizeof(body), bytes, error);
    case HTML_PARSE:
        return hweb_parse_html_tags_1992(html, sizeof(html) - 1u, document,
                                         error);
    }
    return HWEB_ERR_ARGUMENT;
}

static void release_result(Scenario scenario, HwebBytes *bytes,
                           HwebHtmlDoc **document) {
    if (scenario == HTML_PARSE) {
        hweb_html_doc_free(document);
    } else {
        hweb_bytes_free(bytes);
    }
}

static void check_control(Scenario scenario, HwebBytes *bytes,
                          HwebHtmlDoc *document) {
    if (scenario == HTML_PARSE) {
        assert(document != NULL);
        assert(hweb_html_node_count(document) >= 6u);
        assert(hweb_html_attribute_count(document) == 2u);
    } else {
        assert(bytes->data != NULL);
        assert(bytes->length != 0u);
        if (scenario == HTTP_RESPONSE) {
            static const uint8_t expected[] = {'b', 'o', 'd', 'y', 0, '\n'};
            assert(bytes->length == sizeof(expected));
            assert(memcmp(bytes->data, expected, sizeof(expected)) == 0);
        }
    }
}

static void check_failed_output(Scenario scenario, HwebBytes *bytes,
                                HwebHtmlDoc *document) {
    if (scenario == HTML_PARSE) {
        assert(document == NULL);
    } else {
        assert(bytes->data == NULL);
        assert(bytes->length == 0u);
    }
}

static SweepResult sweep(Scenario scenario, const char *label) {
    HwebBytes bytes = {NULL, 0u};
    HwebHtmlDoc *document = NULL;
    HwebError error;
    HwebStatus status;
    SweepResult result = {0u, 0u};
    size_t allocation_count;

    allocation_calls = 0u;
    allocation_to_fail = 0u;
    allocation_failure_armed = 1;
    status = run_scenario(scenario, &bytes, &document, &error);
    allocation_failure_armed = 0;
    allocation_count = allocation_calls;
    assert(status == HWEB_OK);
    check_control(scenario, &bytes, document);
    release_result(scenario, &bytes, &document);
    assert(scenario != HTML_PARSE || document == NULL);
    result.controls = 1u;

    for (size_t target = 1u; target <= allocation_count; target++) {
        bytes.data = NULL;
        bytes.length = 0u;
        document = NULL;
        allocation_calls = 0u;
        allocation_to_fail = target;
        allocation_failure_armed = 1;
        status = run_scenario(scenario, &bytes, &document, &error);
        allocation_failure_armed = 0;
        assert(status == HWEB_ERR_NO_MEMORY);
        assert(error.status == HWEB_ERR_NO_MEMORY);
        check_failed_output(scenario, &bytes, document);
        release_result(scenario, &bytes, &document);
        assert(scenario != HTML_PARSE || document == NULL);
        result.failures++;
    }

    printf("%s: %zu successful-control allocation(s), %zu forced failure(s)\n",
           label, allocation_count, result.failures);
    return result;
}

int main(void) {
    SweepResult encode = sweep(HTTP_ENCODE, "HTTP encode");
    SweepResult parse = sweep(HTTP_PARSE, "HTTP parse");
    SweepResult response = sweep(HTTP_RESPONSE, "HTTP response");
    SweepResult html = sweep(HTML_PARSE, "HTML parse");
    assert(encode.controls + parse.controls + response.controls + html.controls == 4u);
    assert(encode.failures + parse.failures + response.failures + html.failures > 0u);
    return 0;
}
