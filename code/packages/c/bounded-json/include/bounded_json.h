/* Bounded, owned JSON values in ISO C17. Views expire with their document. */
#ifndef CA_BOUNDED_JSON_H
#define CA_BOUNDED_JSON_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define BJ_MAX_BYTES ((size_t)4194304)
#define BJ_MAX_DEPTH ((size_t)128)
#define BJ_MAX_NODES ((size_t)65536)
#define BJ_MAX_ALLOC ((size_t)16777216)

typedef enum {
    BJ_OK = 0, BJ_ARGUMENT, BJ_SYNTAX, BJ_UTF8, BJ_NUMBER,
    BJ_BYTE_LIMIT, BJ_DEPTH_LIMIT, BJ_NODE_LIMIT, BJ_MEMORY_LIMIT, BJ_ALLOC
} BJsonStatus;
typedef enum {
    BJ_INVALID = 0, BJ_NULL, BJ_BOOL, BJ_INTEGER, BJ_FLOAT,
    BJ_STRING, BJ_ARRAY, BJ_OBJECT
} BJsonKind;
typedef struct BJsonDoc BJsonDoc;
typedef struct BJsonValue BJsonValue;
typedef struct { BJsonStatus kind; size_t offset; } BJsonError;
typedef void *(*BJsonAlloc)(size_t bytes, void *context);
typedef void (*BJsonFree)(void *pointer, void *context);
typedef struct {
    size_t max_bytes, max_depth, max_nodes, max_alloc_bytes;
    BJsonAlloc allocate;
    BJsonFree release;
    void *context;
} BJsonOptions;

/* Defaults are hard ceilings; options may tighten them. Allocators must return
 * suitably aligned fresh storage, and remain usable until document release.
 * out must point to a NULL handle. On error it stays NULL (an already-owned
 * out is rejected without changing it). Input must be readable and unchanged
 * throughout the call; it may be released/changed immediately afterward. */
BJsonOptions bj_options(void);
BJsonStatus bj_parse(const char *input, size_t length, BJsonDoc **out,
                     BJsonError *error);
BJsonStatus bj_parse_with_options(const char *input, size_t length,
    const BJsonOptions *options, BJsonDoc **out, BJsonError *error);
void bj_doc_free(BJsonDoc **document); /* NULL/empty/repeated release is safe. */
const BJsonValue *bj_root(const BJsonDoc *document);
BJsonKind bj_kind(const BJsonValue *value); /* NULL view => BJ_INVALID. */
int bj_boolean(const BJsonValue *value, int *out);
int bj_integer(const BJsonValue *value, int64_t *out);
int bj_number(const BJsonValue *value, double *out);
int bj_string(const BJsonValue *value, const char **out, size_t *length);
size_t bj_size(const BJsonValue *value); /* Number of array/object members. */
const BJsonValue *bj_at(const BJsonValue *array, size_t index);
/* Ordered members retain duplicate names. Names have explicit lengths and may
 * contain decoded NUL. No implicit last-wins lookup discards information. */
int bj_member(const BJsonValue *object, size_t index, const char **name,
              size_t *name_length, const BJsonValue **value);

#ifdef __cplusplus
}
#endif
#endif
