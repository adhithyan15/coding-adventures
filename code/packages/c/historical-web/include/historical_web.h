/* Historical Web profiles: CERN HTTP/0.9 and archived HTML Tags 1992. */
#ifndef CA_HISTORICAL_WEB_H
#define CA_HISTORICAL_WEB_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define HWEB_VERSION "0.1.0"
#define HWEB_NO_INDEX SIZE_MAX

typedef enum {
    HWEB_OK = 0,
    HWEB_ERR_ARGUMENT,
    HWEB_ERR_PROTOCOL,
    HWEB_ERR_UTF8,
    HWEB_ERR_NO_MEMORY,
    HWEB_ERR_OVERFLOW
} HwebStatus;

typedef struct {
    HwebStatus status;
    size_t byte_offset;
} HwebError;

/* Owned byte results are not NUL terminated. Initialize to {NULL, 0}; release
 * with hweb_bytes_free. Input buffers must remain readable and stable for the
 * call. Outputs/errors must be writable and not overlap input or each other.
 * Null input is permitted only when its explicit length is zero. */
typedef struct {
    uint8_t *data;
    size_t length;
} HwebBytes;

HwebStatus hweb_http09_encode_request(const uint8_t *address,
                                      size_t address_length,
                                      HwebBytes *out, HwebError *error);
HwebStatus hweb_http09_parse_request(const uint8_t *request,
                                     size_t request_length,
                                     HwebBytes *out, HwebError *error);
HwebStatus hweb_http09_decode_response(const uint8_t *response,
                                       size_t response_length,
                                       HwebBytes *out, HwebError *error);
void hweb_bytes_free(HwebBytes *bytes);

typedef struct {
    const char *data;
    size_t length;
} HwebStringView;

typedef enum {
    HWEB_NODE_TEXT = 0,
    HWEB_NODE_ELEMENT
} HwebNodeKind;

typedef struct {
    HwebStringView name;
    HwebStringView value;
    int has_value;
} HwebAttribute;

typedef struct {
    HwebNodeKind kind;
    size_t parent_index;
    size_t first_child_index;
    size_t next_sibling_index;
    HwebStringView tag;
    HwebStringView text;
    size_t first_attribute_index;
    size_t attribute_count;
} HwebNode;

typedef struct HwebHtmlDoc HwebHtmlDoc;

/* Source is UTF-8, may contain embedded NUL, and is not retained. Out must be
 * NULL; success owns the flat arena until doc_free. Views are length-delimited
 * and borrowed from the document, not necessarily NUL-terminated. The caller
 * must provide readable stable input and non-overlapping writable outputs. */
HwebStatus hweb_parse_html_tags_1992(const uint8_t *source,
                                     size_t source_length,
                                     HwebHtmlDoc **out, HwebError *error);
void hweb_html_doc_free(HwebHtmlDoc **document);
size_t hweb_html_node_count(const HwebHtmlDoc *document);
const HwebNode *hweb_html_node_at(const HwebHtmlDoc *document, size_t index);
size_t hweb_html_attribute_count(const HwebHtmlDoc *document);
const HwebAttribute *hweb_html_attribute_at(const HwebHtmlDoc *document,
                                            size_t index);
size_t hweb_html_root_first(const HwebHtmlDoc *document);

#ifdef __cplusplus
}
#endif

#endif /* CA_HISTORICAL_WEB_H */
