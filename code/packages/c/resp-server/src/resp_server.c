/*
 * resp_server.c — a Redis-style RESP server on tcp-runtime + resp-protocol.
 * ===========================================================================
 *
 * A tcp_runtime handler that decodes one RESP command per read (via
 * resp-protocol), dispatches it against a shared in-memory keyspace, and writes
 * the RESP-encoded reply. The keyspace is passed as the handler's `user`
 * pointer, so every connection shares it. Single-threaded on the reactor, so the
 * keyspace needs no locking.
 */
#include "resp_server/resp_server.h"

#include "resp_protocol.h"
#include "tcp_runtime/tcp_runtime.h"

#include <ctype.h>  /* toupper */
#include <stdlib.h> /* malloc, free, calloc, realloc */
#include <string.h> /* memcpy, memcmp, strlen */

/* ── the keyspace: a small array of {key, value} byte pairs ─────────────── */

struct kv {
    unsigned char *key;
    size_t key_len;
    unsigned char *val;
    size_t val_len;
};

struct keystore {
    struct kv *entries;
    size_t count;
    size_t cap;
};

/* Copy `len` bytes (len 0 → a 1-byte allocation, never malloc(0)). */
static unsigned char *osp__memdup(const unsigned char *src, size_t len) {
    unsigned char *p = (unsigned char *)malloc(len ? len : 1);
    if (p != NULL && len != 0) {
        memcpy(p, src, len);
    }
    return p;
}

static size_t kv_find(const struct keystore *ks, const unsigned char *key,
                      size_t key_len) {
    size_t i;
    for (i = 0; i < ks->count; i++) {
        if (ks->entries[i].key_len == key_len &&
            memcmp(ks->entries[i].key, key, key_len) == 0) {
            return i;
        }
    }
    return ks->count; /* not found */
}

/* Store (or replace) key → value. Returns 0 on success, -1 on allocation
 * failure (the store is left unchanged). */
static int kv_set(struct keystore *ks, const unsigned char *key, size_t key_len,
                  const unsigned char *val, size_t val_len) {
    size_t idx = kv_find(ks, key, key_len);
    unsigned char *vdup = osp__memdup(val, val_len);
    if (vdup == NULL) {
        return -1;
    }
    if (idx < ks->count) {
        /* replace the existing value */
        free(ks->entries[idx].val);
        ks->entries[idx].val = vdup;
        ks->entries[idx].val_len = val_len;
        return 0;
    }
    /* append a new entry */
    if (ks->count == ks->cap) {
        size_t ncap = (ks->cap == 0) ? 8 : ks->cap * 2;
        struct kv *ne = (struct kv *)realloc(ks->entries, ncap * sizeof(*ne));
        if (ne == NULL) {
            free(vdup);
            return -1;
        }
        ks->entries = ne;
        ks->cap = ncap;
    }
    {
        unsigned char *kdup = osp__memdup(key, key_len);
        if (kdup == NULL) {
            free(vdup);
            return -1;
        }
        ks->entries[ks->count].key = kdup;
        ks->entries[ks->count].key_len = key_len;
        ks->entries[ks->count].val = vdup;
        ks->entries[ks->count].val_len = val_len;
        ks->count++;
    }
    return 0;
}

static int kv_get(const struct keystore *ks, const unsigned char *key,
                  size_t key_len, unsigned char **out_val, size_t *out_len) {
    size_t idx = kv_find(ks, key, key_len);
    if (idx < ks->count) {
        *out_val = ks->entries[idx].val;
        *out_len = ks->entries[idx].val_len;
        return 1;
    }
    return 0;
}

static void kv_free(struct keystore *ks) {
    size_t i;
    for (i = 0; i < ks->count; i++) {
        free(ks->entries[i].key);
        free(ks->entries[i].val);
    }
    free(ks->entries);
    ks->entries = NULL;
    ks->count = 0;
    ks->cap = 0;
}

/* ── command dispatch ──────────────────────────────────────────────────── */

/* Is bulk-string `v` equal to `name`, case-insensitively? */
static int name_eq(const RespValue *v, const char *name) {
    size_t nl = strlen(name);
    size_t i;
    if (v->type != RESP_BULK_STRING || v->as.bulk.is_null ||
        v->as.bulk.len != nl) {
        return 0;
    }
    for (i = 0; i < nl; i++) {
        if (toupper(v->as.bulk.data[i]) != toupper((unsigned char)name[i])) {
            return 0;
        }
    }
    return 1;
}

static int is_bulk(const RespValue *v) {
    return v->type == RESP_BULK_STRING && !v->as.bulk.is_null;
}

/* Turn a decoded command into a reply value (caller frees with resp_free). */
static RespValue *dispatch(struct keystore *ks, const RespValue *cmd) {
    RespValue **items;
    size_t argc;

    if (cmd->type != RESP_ARRAY || cmd->as.array.is_null ||
        cmd->as.array.count < 1) {
        return resp_error("ERR malformed command");
    }
    items = cmd->as.array.items;
    argc = cmd->as.array.count;
    if (!is_bulk(items[0])) {
        return resp_error("ERR malformed command");
    }

    if (name_eq(items[0], "PING")) {
        if (argc >= 2 && is_bulk(items[1])) {
            return resp_bulk_string(items[1]->as.bulk.data, items[1]->as.bulk.len);
        }
        return resp_simple_string("PONG");
    }
    if (name_eq(items[0], "ECHO")) {
        if (argc >= 2 && is_bulk(items[1])) {
            return resp_bulk_string(items[1]->as.bulk.data, items[1]->as.bulk.len);
        }
        return resp_error("ERR wrong number of arguments for 'echo'");
    }
    if (name_eq(items[0], "SET")) {
        if (argc >= 3 && is_bulk(items[1]) && is_bulk(items[2])) {
            if (kv_set(ks, items[1]->as.bulk.data, items[1]->as.bulk.len,
                       items[2]->as.bulk.data, items[2]->as.bulk.len) != 0) {
                return resp_error("ERR out of memory");
            }
            return resp_simple_string("OK");
        }
        return resp_error("ERR wrong number of arguments for 'set'");
    }
    if (name_eq(items[0], "GET")) {
        if (argc >= 2 && is_bulk(items[1])) {
            unsigned char *val = NULL;
            size_t vlen = 0;
            if (kv_get(ks, items[1]->as.bulk.data, items[1]->as.bulk.len, &val,
                       &vlen)) {
                return resp_bulk_string(val, vlen);
            }
            return resp_bulk_null();
        }
        return resp_error("ERR wrong number of arguments for 'get'");
    }
    return resp_error("ERR unknown command");
}

/* ── the tcp_runtime handler ───────────────────────────────────────────── */

static tcp_action resp_handler(uint64_t conn_id, const void *data, size_t len,
                               void *out, size_t out_cap, void *user) {
    struct keystore *ks = (struct keystore *)user;
    RespValue *cmd = NULL;
    RespValue *reply = NULL;
    size_t consumed = 0;
    tcp_action a;
    RespDecodeStatus ds;

    a.write_len = 0;
    a.close = 0;
    (void)conn_id;

    ds = resp_decode((const unsigned char *)data, len, &cmd, &consumed);
    if (ds == RESP_DECODE_INCOMPLETE) {
        return a; /* need more bytes; the phase-one handler cannot buffer */
    }
    if (ds == RESP_DECODE_ERROR) {
        reply = resp_error("ERR protocol error");
    } else {
        reply = dispatch(ks, cmd);
        resp_free(cmd);
    }
    if (reply != NULL) {
        unsigned char *buf = NULL;
        size_t buf_len = 0;
        if (resp_encode(reply, &buf, &buf_len) == RESP_ENCODE_OK && buf != NULL) {
            size_t n = (buf_len < out_cap) ? buf_len : out_cap;
            memcpy(out, buf, n);
            a.write_len = n;
            free(buf);
        }
        resp_free(reply);
    }
    return a;
}

/* ── the server: a thin wrapper over tcp_runtime + a keyspace ──────────── */

struct resp_server {
    tcp_runtime *rt;
    struct keystore store;
};

osp_status resp_server_bind(resp_server **out, const char *host,
                            unsigned short port) {
    struct resp_server *s;
    osp_status st;
    if (out == NULL || host == NULL) {
        return OSP_ERR_INVAL;
    }
    s = (struct resp_server *)calloc(1, sizeof(*s)); /* keyspace zeroed */
    if (s == NULL) {
        return OSP_ERR_NOMEM;
    }
    /* &s->store is stable for the runtime's life (s is not moved). */
    st = tcp_runtime_bind(&s->rt, host, port, resp_handler, &s->store);
    if (st != OSP_OK) {
        free(s);
        return st;
    }
    *out = s;
    return OSP_OK;
}

osp_status resp_server_local_port(resp_server *s, unsigned short *out_port) {
    if (s == NULL || out_port == NULL) {
        return OSP_ERR_INVAL;
    }
    return tcp_runtime_local_port(s->rt, out_port);
}

osp_status resp_server_poll(resp_server *s, int timeout_ms, int *out_handled) {
    if (s == NULL || out_handled == NULL) {
        return OSP_ERR_INVAL;
    }
    return tcp_runtime_poll(s->rt, timeout_ms, out_handled);
}

osp_status resp_server_serve(resp_server *s) {
    if (s == NULL) {
        return OSP_ERR_INVAL;
    }
    return tcp_runtime_serve(s->rt);
}

void resp_server_stop(resp_server *s) {
    if (s != NULL) {
        tcp_runtime_stop(s->rt);
    }
}

osp_status resp_server_destroy(resp_server *s) {
    if (s == NULL) {
        return OSP_ERR_INVAL;
    }
    tcp_runtime_destroy(s->rt);
    kv_free(&s->store);
    free(s);
    return OSP_OK;
}
