/*
 * The November 1992 HTML Tags snapshot, translated as a small ISO C17 tree
 * builder. The public result is a flat arena: parent/child/sibling indexes
 * teach the same structure without C recursion or recursive destruction.
 */
#include "historical_web.h"

#include <stdlib.h>
#include <string.h>

typedef enum {
    TAG_UNKNOWN = 0,
    TAG_TITLE,
    TAG_ADDRESS,
    TAG_A,
    TAG_H1,
    TAG_H2,
    TAG_H3,
    TAG_H4,
    TAG_H5,
    TAG_H6,
    TAG_UL,
    TAG_MENU,
    TAG_DIR,
    TAG_LI,
    TAG_DL,
    TAG_DT,
    TAG_DD,
    TAG_P,
    TAG_ISINDEX,
    TAG_XMP,
    TAG_LISTING,
    TAG_PLAINTEXT,
    TAG_COUNT
} TagId;

static const char *const tag_names[TAG_COUNT] = {
    "", "title", "address", "a", "h1", "h2", "h3", "h4", "h5",
    "h6", "ul", "menu", "dir", "li", "dl", "dt", "dd", "p",
    "isindex", "xmp", "listing", "plaintext"};

typedef struct {
    size_t text_offset;
    size_t previous_open;
    size_t stack_slot;
    size_t last_child;
    TagId tag;
} NodeMeta;

typedef struct {
    size_t value_offset;
} AttributeMeta;

struct HwebHtmlDoc {
    HwebNode *nodes;
    NodeMeta *node_meta;
    size_t node_count;
    size_t node_capacity;
    size_t node_meta_capacity;

    HwebAttribute *attributes;
    AttributeMeta *attribute_meta;
    size_t attribute_count;
    size_t attribute_capacity;
    size_t attribute_meta_capacity;

    char *strings;
    size_t strings_length;
    size_t strings_capacity;

    size_t *open_stack;
    size_t open_count;
    size_t open_capacity;
    size_t latest_open[TAG_COUNT];

    size_t root_first;
    size_t root_last;
};

typedef struct {
    char *data;
    size_t length;
    size_t capacity;
} ByteBuffer;

typedef struct {
    TagId tag;
    int closing;
    size_t attributes_start;
    size_t attributes_length;
    size_t end;
} ParsedTag;

typedef struct {
    size_t name_start;
    size_t name_length;
    size_t value_start;
    size_t value_length;
    size_t next;
    int has_value;
} ParsedAttribute;

static void set_error(HwebError *error, HwebStatus status, size_t byte_offset) {
    if (error != NULL) {
        error->status = status;
        error->byte_offset = byte_offset;
    }
}

static HwebStatus fail(HwebError *error, HwebStatus status, size_t byte_offset) {
    set_error(error, status, byte_offset);
    return status;
}

/* Grow a dynamic array without wrapping its capacity or byte size. */
static HwebStatus reserve_array(void **array, size_t *capacity,
                                size_t needed, size_t item_size) {
    size_t next;
    void *grown;

    if (needed <= *capacity) {
        return HWEB_OK;
    }
    if (item_size == 0) {
        return HWEB_ERR_OVERFLOW;
    }
    next = *capacity == 0 ? 8u : *capacity;
    while (next < needed) {
        if (next > SIZE_MAX / 2u) {
            next = needed;
            break;
        }
        next *= 2u;
    }
    if (next > SIZE_MAX / item_size) {
        return HWEB_ERR_OVERFLOW;
    }
    grown = realloc(*array, next * item_size);
    if (grown == NULL) {
        return HWEB_ERR_NO_MEMORY;
    }
    *array = grown;
    *capacity = next;
    return HWEB_OK;
}

static HwebStatus buffer_append(ByteBuffer *buffer, const char *bytes,
                                size_t length) {
    size_t needed;
    HwebStatus status;
    void *storage;

    if (length == 0) {
        return HWEB_OK;
    }
    if (length > SIZE_MAX - buffer->length) {
        return HWEB_ERR_OVERFLOW;
    }
    needed = buffer->length + length;
    storage = buffer->data;
    status = reserve_array(&storage, &buffer->capacity, needed,
                           sizeof(*buffer->data));
    if (status != HWEB_OK) {
        return status;
    }
    buffer->data = (char *)storage;
    memcpy(buffer->data + buffer->length, bytes, length);
    buffer->length = needed;
    return HWEB_OK;
}

static HwebStatus strings_append(HwebHtmlDoc *document, const char *bytes,
                                 size_t length, size_t *offset_out) {
    size_t needed;
    HwebStatus status;
    void *storage;

    *offset_out = document->strings_length;
    if (length == 0) {
        return HWEB_OK;
    }
    if (length > SIZE_MAX - document->strings_length) {
        return HWEB_ERR_OVERFLOW;
    }
    needed = document->strings_length + length;
    storage = document->strings;
    status = reserve_array(&storage, &document->strings_capacity, needed,
                           sizeof(*document->strings));
    if (status != HWEB_OK) {
        return status;
    }
    document->strings = (char *)storage;
    memcpy(document->strings + document->strings_length, bytes, length);
    document->strings_length = needed;
    return HWEB_OK;
}

static void document_release(HwebHtmlDoc *document) {
    if (document == NULL) {
        return;
    }
    free(document->nodes);
    free(document->node_meta);
    free(document->attributes);
    free(document->attribute_meta);
    free(document->strings);
    free(document->open_stack);
    free(document);
}

static int is_continuation(uint8_t byte) {
    return byte >= 0x80u && byte <= 0xbfu;
}

/* Validate UTF-8 without reading past the caller's explicit byte span. */
static int valid_utf8(const uint8_t *source, size_t length,
                      size_t *bad_offset) {
    size_t cursor = 0;

    while (cursor < length) {
        uint8_t first = source[cursor];
        size_t remaining = length - cursor;
        if (first <= 0x7fu) {
            cursor++;
        } else if (first >= 0xc2u && first <= 0xdfu) {
            if (remaining < 2u) {
                *bad_offset = length;
                return 0;
            }
            if (!is_continuation(source[cursor + 1u])) {
                *bad_offset = cursor + 1u;
                return 0;
            }
            cursor += 2u;
        } else if (first >= 0xe0u && first <= 0xefu) {
            uint8_t second;
            if (remaining < 2u) {
                *bad_offset = length;
                return 0;
            }
            second = source[cursor + 1u];
            if ((first == 0xe0u && (second < 0xa0u || second > 0xbfu)) ||
                (first == 0xedu && (second < 0x80u || second > 0x9fu)) ||
                ((first != 0xe0u && first != 0xedu) &&
                 !is_continuation(second))) {
                *bad_offset = cursor + 1u;
                return 0;
            }
            if (remaining < 3u) {
                *bad_offset = length;
                return 0;
            }
            if (!is_continuation(source[cursor + 2u])) {
                *bad_offset = cursor + 2u;
                return 0;
            }
            cursor += 3u;
        } else if (first >= 0xf0u && first <= 0xf4u) {
            uint8_t second;
            if (remaining < 2u) {
                *bad_offset = length;
                return 0;
            }
            second = source[cursor + 1u];
            if ((first == 0xf0u && (second < 0x90u || second > 0xbfu)) ||
                (first == 0xf4u && (second < 0x80u || second > 0x8fu)) ||
                ((first != 0xf0u && first != 0xf4u) &&
                 !is_continuation(second))) {
                *bad_offset = cursor + 1u;
                return 0;
            }
            if (remaining < 3u) {
                *bad_offset = length;
                return 0;
            }
            if (!is_continuation(source[cursor + 2u])) {
                *bad_offset = cursor + 2u;
                return 0;
            }
            if (remaining < 4u) {
                *bad_offset = length;
                return 0;
            }
            if (!is_continuation(source[cursor + 3u])) {
                *bad_offset = cursor + 3u;
                return 0;
            }
            cursor += 4u;
        } else {
            *bad_offset = cursor;
            return 0;
        }
    }
    return 1;
}

static int ascii_alpha(uint8_t byte) {
    return (byte >= (uint8_t)'A' && byte <= (uint8_t)'Z') ||
           (byte >= (uint8_t)'a' && byte <= (uint8_t)'z');
}

static int ascii_digit(uint8_t byte) {
    return byte >= (uint8_t)'0' && byte <= (uint8_t)'9';
}

static int html_space(uint8_t byte) {
    return byte == (uint8_t)' ' || byte == (uint8_t)'\t' ||
           byte == (uint8_t)'\n' || byte == (uint8_t)'\r' ||
           byte == (uint8_t)'\f';
}

static int name_character(uint8_t byte) {
    return ascii_alpha(byte) || ascii_digit(byte) || byte == (uint8_t)'-';
}

static uint8_t ascii_lower(uint8_t byte) {
    if (byte >= (uint8_t)'A' && byte <= (uint8_t)'Z') {
        return (uint8_t)(byte + ((uint8_t)'a' - (uint8_t)'A'));
    }
    return byte;
}

/* Return one decoded byte and the number of source bytes consumed. An entity
 * replacement is always ASCII; zero means the ampersand stays literal. */
static uint8_t entity_at(const uint8_t *source, size_t remaining,
                         size_t *consumed) {
    *consumed = 0;
    if (remaining >= 4u && source[0] == 0x26u && source[1] == 0x6cu &&
        source[2] == 0x74u && source[3] == 0x3bu) {
        *consumed = 4u;
        return 0x3cu;
    }
    if (remaining >= 4u && source[0] == 0x26u && source[1] == 0x67u &&
        source[2] == 0x74u && source[3] == 0x3bu) {
        *consumed = 4u;
        return 0x3eu;
    }
    if (remaining >= 5u && source[0] == 0x26u && source[1] == 0x61u &&
        source[2] == 0x6du && source[3] == 0x70u && source[4] == 0x3bu) {
        *consumed = 5u;
        return 0x26u;
    }
    return 0u;
}

static HwebStatus append_entity_text(ByteBuffer *pending,
                                    const uint8_t *source, size_t length) {
    size_t cursor = 0;
    while (cursor < length) {
        size_t consumed = 0;
        uint8_t replacement = 0;
        HwebStatus status;
        if (source[cursor] == 0x26u) {
            replacement = entity_at(source + cursor, length - cursor,
                                    &consumed);
        }
        if (consumed != 0) {
            char byte = (char)replacement;
            status = buffer_append(pending, &byte, 1u);
            cursor += consumed;
        } else {
            status = buffer_append(pending,
                                   (const char *)(source + cursor), 1u);
            cursor++;
        }
        if (status != HWEB_OK) {
            return status;
        }
    }
    return HWEB_OK;
}

static int equals_ascii_ci(const uint8_t *source, size_t length,
                           const char *name) {
    size_t index = 0;
    while (index < length && name[index] != '\0') {
        if (ascii_lower(source[index]) != (uint8_t)name[index]) {
            return 0;
        }
        index++;
    }
    return index == length && name[index] == '\0';
}

static TagId identify_tag(const uint8_t *source, size_t length) {
    TagId tag;
    for (tag = TAG_TITLE; tag < TAG_COUNT; tag = (TagId)(tag + 1)) {
        size_t name_length = strlen(tag_names[tag]);
        if (name_length == length && equals_ascii_ci(source, length,
                                                     tag_names[tag])) {
            return tag;
        }
    }
    return TAG_UNKNOWN;
}

static int parse_tag_at(const uint8_t *source, size_t length, size_t start,
                        ParsedTag *tag_out) {
    size_t cursor = start + 1u;
    size_t name_start;
    size_t name_length;
    int closing = 0;

    if (cursor < length && source[cursor] == (uint8_t)'/') {
        closing = 1;
        cursor++;
    }
    if (cursor >= length || !ascii_alpha(source[cursor])) {
        return 0;
    }
    name_start = cursor++;
    while (cursor < length && name_character(source[cursor])) {
        cursor++;
    }
    name_length = cursor - name_start;
    if (cursor >= length || source[cursor] == (uint8_t)'<') {
        return 0;
    }
    if (source[cursor] == (uint8_t)'>') {
        tag_out->tag = identify_tag(source + name_start, name_length);
        tag_out->closing = closing;
        tag_out->attributes_start = cursor;
        tag_out->attributes_length = 0;
        tag_out->end = cursor + 1u;
        return 1;
    }
    if (!html_space(source[cursor])) {
        return 0;
    }

    tag_out->attributes_start = cursor;
    while (cursor < length) {
        uint8_t byte = source[cursor];
        if (byte == (uint8_t)'>') {
            tag_out->tag = identify_tag(source + name_start, name_length);
            tag_out->closing = closing;
            tag_out->attributes_length = cursor - tag_out->attributes_start;
            tag_out->end = cursor + 1u;
            return 1;
        }
        if (byte == (uint8_t)'<') {
            return 0;
        }
        if (byte == (uint8_t)'"' || byte == (uint8_t)'\'') {
            uint8_t quote = byte;
            cursor++;
            while (cursor < length && source[cursor] != quote &&
                   source[cursor] != (uint8_t)'<') {
                cursor++;
            }
            if (cursor >= length || source[cursor] == (uint8_t)'<') {
                return 0;
            }
        }
        cursor++;
    }
    return 0;
}

static int is_empty_tag(TagId tag) {
    return tag == TAG_P || tag == TAG_ISINDEX;
}

static int is_heading(TagId tag) {
    return tag >= TAG_H1 && tag <= TAG_H6;
}

static int is_term(TagId tag) {
    return tag == TAG_DT || tag == TAG_DD;
}

static size_t parent_index(const HwebHtmlDoc *document) {
    return document->open_count == 0
               ? HWEB_NO_INDEX
               : document->open_stack[document->open_count - 1u];
}

static HwebStatus append_node(HwebHtmlDoc *document, HwebNodeKind kind,
                              size_t parent, TagId tag, size_t *index_out) {
    HwebStatus status;
    size_t index;
    void *storage;

    if (document->node_count == SIZE_MAX) {
        return HWEB_ERR_OVERFLOW;
    }
    index = document->node_count + 1u;
    storage = document->nodes;
    status = reserve_array(&storage, &document->node_capacity, index,
                           sizeof(*document->nodes));
    if (status != HWEB_OK) {
        return status;
    }
    document->nodes = (HwebNode *)storage;
    storage = document->node_meta;
    status = reserve_array(&storage, &document->node_meta_capacity, index,
                           sizeof(*document->node_meta));
    if (status != HWEB_OK) {
        return status;
    }
    document->node_meta = (NodeMeta *)storage;

    index = document->node_count;
    memset(&document->nodes[index], 0, sizeof(document->nodes[index]));
    document->nodes[index].kind = kind;
    document->nodes[index].parent_index = parent;
    document->nodes[index].first_child_index = HWEB_NO_INDEX;
    document->nodes[index].next_sibling_index = HWEB_NO_INDEX;
    document->nodes[index].first_attribute_index = HWEB_NO_INDEX;
    document->node_meta[index].text_offset = HWEB_NO_INDEX;
    document->node_meta[index].previous_open = HWEB_NO_INDEX;
    document->node_meta[index].stack_slot = HWEB_NO_INDEX;
    document->node_meta[index].last_child = HWEB_NO_INDEX;
    document->node_meta[index].tag = tag;
    if (kind == HWEB_NODE_ELEMENT) {
        document->nodes[index].tag.data = tag_names[tag];
        document->nodes[index].tag.length = strlen(tag_names[tag]);
    }
    document->node_count++;

    if (parent == HWEB_NO_INDEX) {
        if (document->root_first == HWEB_NO_INDEX) {
            document->root_first = index;
        } else {
            document->nodes[document->root_last].next_sibling_index = index;
        }
        document->root_last = index;
    } else {
        if (document->nodes[parent].first_child_index == HWEB_NO_INDEX) {
            document->nodes[parent].first_child_index = index;
        } else {
            document->nodes[document->node_meta[parent].last_child]
                .next_sibling_index = index;
        }
        document->node_meta[parent].last_child = index;
    }
    *index_out = index;
    return HWEB_OK;
}

static HwebStatus flush_text(HwebHtmlDoc *document, size_t parent,
                             ByteBuffer *pending) {
    size_t last_child;
    size_t offset;
    size_t new_index;
    HwebStatus status;

    if (pending->length == 0) {
        return HWEB_OK;
    }
    last_child = parent == HWEB_NO_INDEX
                     ? document->root_last
                     : document->node_meta[parent].last_child;
    status = strings_append(document, pending->data, pending->length, &offset);
    if (status != HWEB_OK) {
        return status;
    }
    if (last_child != HWEB_NO_INDEX &&
        document->nodes[last_child].kind == HWEB_NODE_TEXT &&
        document->node_meta[last_child].text_offset <= offset &&
        document->nodes[last_child].text.length ==
            offset - document->node_meta[last_child].text_offset) {
        if (pending->length >
            SIZE_MAX - document->nodes[last_child].text.length) {
            return HWEB_ERR_OVERFLOW;
        }
        document->nodes[last_child].text.length += pending->length;
    } else {
        status = append_node(document, HWEB_NODE_TEXT, parent, TAG_UNKNOWN,
                             &new_index);
        if (status != HWEB_OK) {
            return status;
        }
        document->node_meta[new_index].text_offset = offset;
        document->nodes[new_index].text.length = pending->length;
    }
    pending->length = 0;
    return HWEB_OK;
}

static HwebStatus append_regular_text(ByteBuffer *pending,
                                      const uint8_t *source, size_t length,
                                      int raw) {
    if (raw) {
        return buffer_append(pending, (const char *)source, length);
    }
    return append_entity_text(pending, source, length);
}

static int parse_attribute_at(const uint8_t *source, size_t length,
                              size_t cursor, ParsedAttribute *out) {
    size_t beginning = cursor;
    size_t value_start;
    while (cursor < length && html_space(source[cursor])) {
        cursor++;
    }
    if (cursor == beginning || cursor >= length ||
        !ascii_alpha(source[cursor])) {
        return 0;
    }
    out->name_start = cursor++;
    while (cursor < length && name_character(source[cursor])) {
        cursor++;
    }
    out->name_length = cursor - out->name_start;
    value_start = cursor;
    while (cursor < length && html_space(source[cursor])) {
        cursor++;
    }
    if (cursor >= length || source[cursor] != (uint8_t)'=') {
        out->has_value = 0;
        out->value_start = 0;
        out->value_length = 0;
        out->next = value_start;
        return 1;
    }
    cursor++;
    while (cursor < length && html_space(source[cursor])) {
        cursor++;
    }
    if (cursor >= length) {
        out->has_value = 0;
        out->value_start = 0;
        out->value_length = 0;
        out->next = value_start;
        return 1;
    }
    if (source[cursor] == (uint8_t)'"' || source[cursor] == (uint8_t)'\'') {
        uint8_t quote = source[cursor++];
        out->value_start = cursor;
        while (cursor < length && source[cursor] != quote) {
            cursor++;
        }
        if (cursor >= length) {
            out->has_value = 0;
            out->value_start = 0;
            out->value_length = 0;
            out->next = value_start;
            return 1;
        }
        out->has_value = 1;
        out->value_length = cursor - out->value_start;
        out->next = cursor + 1u;
        return 1;
    }
    out->value_start = cursor;
    while (cursor < length && !html_space(source[cursor]) &&
           source[cursor] != (uint8_t)'"' &&
           source[cursor] != (uint8_t)'\'' &&
           source[cursor] != (uint8_t)'=' &&
           source[cursor] != (uint8_t)'<' &&
           source[cursor] != (uint8_t)'>') {
        cursor++;
    }
    if (cursor == out->value_start) {
        out->has_value = 0;
        out->value_start = 0;
        out->value_length = 0;
        out->next = value_start;
        return 1;
    }
    out->has_value = 1;
    out->value_length = cursor - out->value_start;
    out->next = cursor;
    return 1;
}

static int attribute_id(const uint8_t *name, size_t length) {
    if (equals_ascii_ci(name, length, "href")) {
        return 0;
    }
    if (equals_ascii_ci(name, length, "name")) {
        return 1;
    }
    if (equals_ascii_ci(name, length, "type")) {
        return 2;
    }
    return -1;
}

static const char *const attribute_names[] = {"href", "name", "type"};

static HwebStatus append_attribute(HwebHtmlDoc *document, int id,
                                   const uint8_t *value, size_t value_length,
                                   int has_value) {
    HwebStatus status;
    size_t needed;
    size_t index;
    size_t offset = HWEB_NO_INDEX;
    size_t cursor = 0;
    void *storage;

    if (document->attribute_count == SIZE_MAX) {
        return HWEB_ERR_OVERFLOW;
    }
    needed = document->attribute_count + 1u;
    storage = document->attributes;
    status = reserve_array(&storage, &document->attribute_capacity, needed,
                           sizeof(*document->attributes));
    if (status != HWEB_OK) {
        return status;
    }
    document->attributes = (HwebAttribute *)storage;
    storage = document->attribute_meta;
    status = reserve_array(&storage, &document->attribute_meta_capacity, needed,
                           sizeof(*document->attribute_meta));
    if (status != HWEB_OK) {
        return status;
    }
    document->attribute_meta = (AttributeMeta *)storage;
    if (has_value) {
        offset = document->strings_length;
        if (value_length > SIZE_MAX - document->strings_length) {
            return HWEB_ERR_OVERFLOW;
        }
        storage = document->strings;
        status = reserve_array(&storage, &document->strings_capacity,
                               document->strings_length + value_length,
                               sizeof(*document->strings));
        if (status != HWEB_OK) {
            return status;
        }
        document->strings = (char *)storage;
        while (cursor < value_length) {
            size_t consumed = 0;
            uint8_t replacement = value[cursor] == 0x26u
                                      ? entity_at(value + cursor,
                                                  value_length - cursor,
                                                  &consumed)
                                      : 0u;
            if (consumed != 0) {
                document->strings[document->strings_length++] =
                    (char)replacement;
                cursor += consumed;
            } else {
                memcpy(document->strings + document->strings_length,
                       value + cursor, 1u);
                document->strings_length++;
                cursor++;
            }
        }
    }
    index = document->attribute_count++;
    document->attributes[index].name.data = attribute_names[id];
    document->attributes[index].name.length = strlen(attribute_names[id]);
    document->attributes[index].value.data = NULL;
    document->attributes[index].value.length =
        has_value ? document->strings_length - offset : 0u;
    document->attributes[index].has_value = has_value;
    document->attribute_meta[index].value_offset = offset;
    return HWEB_OK;
}

static HwebStatus parse_anchor_attributes(HwebHtmlDoc *document,
                                         size_t node_index,
                                         const uint8_t *source,
                                         size_t start, size_t length) {
    size_t cursor = 0;
    unsigned int seen = 0u;
    HwebNode *node = &document->nodes[node_index];
    node->first_attribute_index = document->attribute_count;
    while (cursor < length) {
        ParsedAttribute attribute;
        int id;
        if (!parse_attribute_at(source + start, length, cursor, &attribute)) {
            break;
        }
        cursor = attribute.next;
        id = attribute_id(source + start + attribute.name_start,
                          attribute.name_length);
        if (id >= 0 && (seen & (1u << (unsigned int)id)) == 0u) {
            HwebStatus status;
            seen |= 1u << (unsigned int)id;
            status = append_attribute(
                document, id,
                attribute.has_value ? source + start + attribute.value_start
                                    : NULL,
                attribute.value_length, attribute.has_value);
            if (status != HWEB_OK) {
                return status;
            }
            node->attribute_count++;
        }
        if (cursor <= attribute.name_start) {
            break;
        }
    }
    return HWEB_OK;
}

static HwebStatus push_open(HwebHtmlDoc *document, size_t node_index,
                            TagId tag) {
    HwebStatus status;
    size_t needed;
    void *storage;
    if (document->open_count == SIZE_MAX) {
        return HWEB_ERR_OVERFLOW;
    }
    needed = document->open_count + 1u;
    storage = document->open_stack;
    status = reserve_array(&storage, &document->open_capacity, needed,
                           sizeof(*document->open_stack));
    if (status != HWEB_OK) {
        return status;
    }
    document->open_stack = (size_t *)storage;
    document->node_meta[node_index].previous_open = document->latest_open[tag];
    document->node_meta[node_index].stack_slot = document->open_count;
    document->latest_open[tag] = node_index;
    document->open_stack[document->open_count++] = node_index;
    return HWEB_OK;
}

static size_t latest_position(const HwebHtmlDoc *document, TagId tag) {
    size_t node = document->latest_open[tag];
    return node == HWEB_NO_INDEX ? HWEB_NO_INDEX
                                 : document->node_meta[node].stack_slot;
}

static size_t maximum_position(const HwebHtmlDoc *document,
                               const TagId *tags, size_t count) {
    size_t best = HWEB_NO_INDEX;
    size_t i;
    for (i = 0; i < count; i++) {
        size_t position = latest_position(document, tags[i]);
        if (position != HWEB_NO_INDEX &&
            (best == HWEB_NO_INDEX || position > best)) {
            best = position;
        }
    }
    return best;
}

static size_t heading_close_position(const HwebHtmlDoc *document) {
    static const TagId headings[] = {TAG_H1, TAG_H2, TAG_H3,
                                     TAG_H4, TAG_H5, TAG_H6};
    return maximum_position(document, headings,
                            sizeof(headings) / sizeof(headings[0]));
}

static size_t list_close_position(const HwebHtmlDoc *document) {
    static const TagId lists[] = {TAG_UL, TAG_MENU, TAG_DIR};
    size_t item = latest_position(document, TAG_LI);
    size_t boundary = maximum_position(document, lists,
                                       sizeof(lists) / sizeof(lists[0]));
    return item != HWEB_NO_INDEX &&
                   (boundary == HWEB_NO_INDEX || item > boundary)
               ? item
               : HWEB_NO_INDEX;
}

static size_t term_close_position(const HwebHtmlDoc *document) {
    static const TagId terms[] = {TAG_DT, TAG_DD};
    size_t term = maximum_position(document, terms,
                                   sizeof(terms) / sizeof(terms[0]));
    size_t boundary = latest_position(document, TAG_DL);
    return term != HWEB_NO_INDEX &&
                   (boundary == HWEB_NO_INDEX || term > boundary)
               ? term
               : HWEB_NO_INDEX;
}

static void pop_to(HwebHtmlDoc *document, size_t stack_position) {
    while (document->open_count > stack_position) {
        size_t node = document->open_stack[--document->open_count];
        TagId tag = document->node_meta[node].tag;
        document->latest_open[tag] = document->node_meta[node].previous_open;
        document->node_meta[node].stack_slot = HWEB_NO_INDEX;
    }
}

static size_t find_raw_end(const uint8_t *source, size_t length,
                           size_t cursor, TagId tag) {
    size_t name_length = strlen(tag_names[tag]);
    size_t candidate = cursor;
    while (candidate < length) {
        size_t i;
        int equal = 1;
        size_t needle_length = name_length + 3u;
        if (source[candidate] == (uint8_t)'<' &&
            length - candidate >= needle_length &&
            source[candidate + 1u] == (uint8_t)'/') {
            for (i = 0; i < name_length; i++) {
                if (ascii_lower(source[candidate + 2u + i]) !=
                    (uint8_t)tag_names[tag][i]) {
                    equal = 0;
                    break;
                }
            }
            if (equal && source[candidate + 2u + name_length] ==
                             (uint8_t)'>') {
                return candidate;
            }
        }
        candidate++;
    }
    return HWEB_NO_INDEX;
}

static HwebStatus finalize_views(HwebHtmlDoc *document) {
    size_t i;
    for (i = 0; i < document->node_count; i++) {
        if (document->nodes[i].kind == HWEB_NODE_TEXT) {
            size_t offset = document->node_meta[i].text_offset;
            if (document->strings != NULL) {
                document->nodes[i].text.data = document->strings + offset;
            }
        }
    }
    for (i = 0; i < document->attribute_count; i++) {
        size_t offset = document->attribute_meta[i].value_offset;
        if (document->attributes[i].has_value && document->strings != NULL) {
            document->attributes[i].value.data = document->strings + offset;
        }
    }
    return HWEB_OK;
}

static HwebStatus normalize_crlf(const uint8_t *source, size_t length,
                                 uint8_t **normalized_out,
                                 size_t *normalized_length_out) {
    uint8_t *normalized;
    size_t input = 0;
    size_t output = 0;

    *normalized_out = NULL;
    *normalized_length_out = 0;
    if (length == 0) {
        return HWEB_OK;
    }
    normalized = (uint8_t *)malloc(length);
    if (normalized == NULL) {
        return HWEB_ERR_NO_MEMORY;
    }
    while (input < length) {
        if (source[input] == (uint8_t)'\r' && input + 1u < length &&
            source[input + 1u] == (uint8_t)'\n') {
            normalized[output++] = (uint8_t)'\n';
            input += 2u;
        } else {
            normalized[output++] = source[input++];
        }
    }
    *normalized_out = normalized;
    *normalized_length_out = output;
    return HWEB_OK;
}

static HwebStatus process_tag(HwebHtmlDoc *document, const uint8_t *source,
                              const ParsedTag *token, ByteBuffer *pending,
                              TagId *raw_mode) {
    size_t close_position = HWEB_NO_INDEX;
    size_t node_index;
    HwebStatus status;
    TagId tag = token->tag;

    if (tag == TAG_UNKNOWN || (token->closing && is_empty_tag(tag))) {
        return HWEB_OK;
    }
    if (token->closing) {
        size_t opened = document->latest_open[tag];
        if (opened == HWEB_NO_INDEX) {
            return HWEB_OK;
        }
        close_position = document->node_meta[opened].stack_slot;
    } else if (is_heading(tag)) {
        close_position = heading_close_position(document);
    } else if (tag == TAG_LI) {
        close_position = list_close_position(document);
    } else if (is_term(tag)) {
        close_position = term_close_position(document);
    }

    status = flush_text(document, parent_index(document), pending);
    if (status != HWEB_OK) {
        return status;
    }
    if (close_position != HWEB_NO_INDEX) {
        pop_to(document, close_position);
    }
    if (token->closing) {
        return HWEB_OK;
    }

    status = append_node(document, HWEB_NODE_ELEMENT,
                         parent_index(document), tag, &node_index);
    if (status != HWEB_OK) {
        return status;
    }
    document->nodes[node_index].first_attribute_index =
        document->attribute_count;
    if (tag == TAG_A) {
        status = parse_anchor_attributes(
            document, node_index, source, token->attributes_start,
            token->attributes_length);
        if (status != HWEB_OK) {
            return status;
        }
    }
    if (!is_empty_tag(tag)) {
        status = push_open(document, node_index, tag);
        if (status != HWEB_OK) {
            return status;
        }
        if (tag == TAG_XMP || tag == TAG_LISTING || tag == TAG_PLAINTEXT) {
            *raw_mode = tag;
        }
    }
    return HWEB_OK;
}

static HwebStatus parse_tokens(HwebHtmlDoc *document,
                               const uint8_t *source, size_t length) {
    ByteBuffer pending = {NULL, 0, 0};
    size_t cursor = 0;
    TagId raw_mode = TAG_UNKNOWN;
    HwebStatus status = HWEB_OK;

    while (cursor < length) {
        size_t opening;
        if (raw_mode != TAG_UNKNOWN) {
            if (raw_mode == TAG_PLAINTEXT) {
                status = append_regular_text(&pending, source + cursor,
                                             length - cursor, 1);
                cursor = length;
                break;
            }
            opening = find_raw_end(source, length, cursor, raw_mode);
            if (opening == HWEB_NO_INDEX) {
                status = append_regular_text(&pending, source + cursor,
                                             length - cursor, 1);
                cursor = length;
                break;
            }
            status = append_regular_text(&pending, source + cursor,
                                         opening - cursor, 1);
            if (status != HWEB_OK) {
                break;
            }
            cursor = opening;
            raw_mode = TAG_UNKNOWN;
            continue;
        }

        opening = cursor;
        while (opening < length && source[opening] != (uint8_t)'<') {
            opening++;
        }
        if (opening > cursor) {
            status = append_regular_text(&pending, source + cursor,
                                         opening - cursor, 0);
            if (status != HWEB_OK) {
                break;
            }
            cursor = opening;
        }
        if (cursor >= length) {
            break;
        }
        if (length - cursor >= 4u &&
            memcmp(source + cursor, "<!--", 4u) == 0) {
            size_t end = cursor + 4u;
            while (end < length &&
                   !(length - end >= 3u &&
                     source[end] == (uint8_t)'-' &&
                     source[end + 1u] == (uint8_t)'-' &&
                     source[end + 2u] == (uint8_t)'>')) {
                end++;
            }
            if (end >= length) {
                cursor = length;
                break;
            }
            cursor = end + 3u;
            continue;
        }
        {
            ParsedTag token;
            if (!parse_tag_at(source, length, cursor, &token)) {
                status = buffer_append(&pending, "<", 1u);
                if (status != HWEB_OK) {
                    break;
                }
                cursor++;
                continue;
            }
            status = process_tag(document, source, &token, &pending,
                                 &raw_mode);
            if (status != HWEB_OK) {
                break;
            }
            cursor = token.end;
        }
    }

    if (status == HWEB_OK) {
        status = flush_text(document, parent_index(document), &pending);
    }
    free(pending.data);
    return status;
}

HwebStatus hweb_parse_html_tags_1992(const uint8_t *source,
                                     size_t source_length,
                                     HwebHtmlDoc **out, HwebError *error) {
    HwebHtmlDoc *document;
    uint8_t *normalized = NULL;
    size_t normalized_length = 0;
    size_t bad_offset = HWEB_NO_INDEX;
    size_t i;
    HwebStatus status;

    if (out == NULL) {
        return fail(error, HWEB_ERR_ARGUMENT, HWEB_NO_INDEX);
    }
    set_error(error, HWEB_OK, HWEB_NO_INDEX);
    if (*out != NULL || (source == NULL && source_length != 0)) {
        return fail(error, HWEB_ERR_ARGUMENT, HWEB_NO_INDEX);
    }
    if (source_length != 0 && !valid_utf8(source, source_length, &bad_offset)) {
        return fail(error, HWEB_ERR_UTF8, bad_offset);
    }
    status = normalize_crlf(source, source_length, &normalized,
                            &normalized_length);
    if (status != HWEB_OK) {
        return fail(error, status, HWEB_NO_INDEX);
    }
    document = (HwebHtmlDoc *)calloc(1u, sizeof(*document));
    if (document == NULL) {
        free(normalized);
        return fail(error, HWEB_ERR_NO_MEMORY, HWEB_NO_INDEX);
    }
    document->root_first = HWEB_NO_INDEX;
    document->root_last = HWEB_NO_INDEX;
    for (i = 0; i < TAG_COUNT; i++) {
        document->latest_open[i] = HWEB_NO_INDEX;
    }
    status = parse_tokens(document, normalized, normalized_length);
    if (status == HWEB_OK) {
        status = finalize_views(document);
    }
    free(normalized);
    if (status != HWEB_OK) {
        document_release(document);
        return fail(error, status, HWEB_NO_INDEX);
    }
    free(document->node_meta);
    document->node_meta = NULL;
    free(document->attribute_meta);
    document->attribute_meta = NULL;
    free(document->open_stack);
    document->open_stack = NULL;
    *out = document;
    set_error(error, HWEB_OK, HWEB_NO_INDEX);
    return HWEB_OK;
}

void hweb_html_doc_free(HwebHtmlDoc **document) {
    if (document == NULL) {
        return;
    }
    document_release(*document);
    *document = NULL;
}

size_t hweb_html_node_count(const HwebHtmlDoc *document) {
    return document == NULL ? 0u : document->node_count;
}

const HwebNode *hweb_html_node_at(const HwebHtmlDoc *document, size_t index) {
    if (document == NULL || index >= document->node_count) {
        return NULL;
    }
    return &document->nodes[index];
}

size_t hweb_html_attribute_count(const HwebHtmlDoc *document) {
    return document == NULL ? 0u : document->attribute_count;
}

const HwebAttribute *hweb_html_attribute_at(const HwebHtmlDoc *document,
                                            size_t index) {
    if (document == NULL || index >= document->attribute_count) {
        return NULL;
    }
    return &document->attributes[index];
}

size_t hweb_html_root_first(const HwebHtmlDoc *document) {
    return document == NULL ? HWEB_NO_INDEX : document->root_first;
}
