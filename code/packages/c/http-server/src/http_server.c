/*
 * http_server.c — a tiny HTTP/1.1 server on tcp-runtime + http-core.
 * ===========================================================================
 *
 * A tcp_runtime handler that:
 *   1. copies the read into a mutable, NUL-terminated buffer;
 *   2. parses the request line + headers in place (the minimal wire framing
 *      that http-core deliberately leaves to a parser layer);
 *   3. builds an http-core HttpRequestHead and uses http-core to interpret it
 *      (version parse, path/query splitting, header lookup);
 *   4. routes and writes a hand-formatted HTTP/1.1 response with Connection:
 *      close, so it is one request/response per connection.
 *
 * The parser is defensive: the request must arrive whole in one read, be under
 * the buffer size, and be well formed, or the client gets a 400 — no unbounded
 * reads, no reassembly (the phase-one handler is stateless).
 */
#include "http_server/http_server.h"

#include "http_core.h"
#include "tcp_runtime/tcp_runtime.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* free */
#include <string.h> /* memcpy, strcmp, strchr, strstr, strlen */

#define HTTP_SRV_REQ_MAX 8192u /* must not exceed tcp_runtime's per-read buffer */
#define HTTP_SRV_MAX_HEADERS 64u

/* Format a complete HTTP/1.1 response into `out` (bounded by out_cap) and ask
 * the runtime to close afterwards (Connection: close). */
static tcp_action respond(void *out, size_t out_cap, unsigned status,
                          const char *reason, const char *body,
                          size_t body_len) {
    tcp_action a;
    char head[256];
    int hl;
    size_t off;

    a.write_len = 0;
    a.close = 1; /* one request/response per connection */

    hl = snprintf(head, sizeof(head),
                  "HTTP/1.1 %u %s\r\n"
                  "Content-Type: text/plain\r\n"
                  "Content-Length: %lu\r\n"
                  "Connection: close\r\n"
                  "\r\n",
                  status, reason, (unsigned long)body_len);
    if (hl < 0) {
        return a;
    }
    off = (size_t)hl;
    if (off > out_cap) {
        off = out_cap;
    }
    memcpy(out, head, off);
    if (off < out_cap && body_len > 0) {
        size_t room = out_cap - off;
        size_t bn = (body_len < room) ? body_len : room;
        memcpy((char *)out + off, body, bn);
        off += bn;
    }
    a.write_len = off;
    return a;
}

/* pass a string literal as (body, length) */
#define BODY(s) (s), strlen(s)

/*
 * Parse `buf` (mutable, NUL-terminated) in place into `req` and `headers`.
 * Returns 0 on success, -1 if malformed or the header block is not complete.
 */
static int parse_request(char *buf, HttpRequestHead *req, HttpHeader *headers,
                         size_t max_headers) {
    char *p = buf;
    char *eol;
    char *sp1;
    char *sp2;
    size_t nh = 0;

    /* request line: METHOD SP TARGET SP HTTP/x.y CRLF */
    eol = strstr(p, "\r\n");
    if (eol == NULL) {
        return -1;
    }
    *eol = '\0';
    sp1 = strchr(p, ' ');
    if (sp1 == NULL) {
        return -1;
    }
    *sp1 = '\0';
    sp2 = strchr(sp1 + 1, ' ');
    if (sp2 == NULL) {
        return -1;
    }
    *sp2 = '\0';
    req->method = p;
    req->target = sp1 + 1;
    if (http_version_parse(sp2 + 1, &req->version) != 0) {
        return -1;
    }

    /* headers until a blank line */
    p = eol + 2;
    for (;;) {
        eol = strstr(p, "\r\n");
        if (eol == NULL) {
            return -1; /* header block never terminates → incomplete */
        }
        if (eol == p) {
            break; /* blank line → end of headers */
        }
        *eol = '\0';
        if (nh < max_headers) {
            char *colon = strchr(p, ':');
            if (colon != NULL) {
                char *val = colon + 1;
                *colon = '\0';
                while (*val == ' ' || *val == '\t') {
                    val++;
                }
                headers[nh].name = p;
                headers[nh].value = val;
                nh++;
            }
            /* a line without a colon is skipped (lenient) */
        }
        p = eol + 2;
    }
    req->headers = headers;
    req->nheaders = nh;
    return 0;
}

/* Route a parsed request and produce its response. */
static tcp_action route(const HttpRequestHead *req, void *out, size_t out_cap) {
    char *path = NULL;
    tcp_action a;

    if (http_request_head_path(req, &path) != 0) {
        return respond(out, out_cap, 400, "Bad Request", BODY("bad target\n"));
    }
    if (strcmp(req->method, "GET") != 0) {
        free(path);
        return respond(out, out_cap, 405, "Method Not Allowed",
                       BODY("method not allowed\n"));
    }

    if (strcmp(path, "/") == 0) {
        a = respond(out, out_cap, 200, "OK", BODY("hello from http-server\n"));
    } else if (strcmp(path, "/echo") == 0) {
        char *msg = NULL;
        int q = http_request_head_query_value(req, "msg", &msg);
        if (q == 1 && msg != NULL) {
            a = respond(out, out_cap, 200, "OK", msg, strlen(msg));
            free(msg);
        } else {
            a = respond(out, out_cap, 200, "OK", "", 0);
        }
    } else if (strcmp(path, "/headers") == 0) {
        char body[4096];
        size_t off = 0;
        size_t i;
        for (i = 0; i < req->nheaders; i++) {
            int w = snprintf(body + off, sizeof(body) - off, "%s: %s\n",
                             req->headers[i].name, req->headers[i].value);
            if (w < 0 || (size_t)w >= sizeof(body) - off) {
                break; /* out of room — stop cleanly */
            }
            off += (size_t)w;
        }
        a = respond(out, out_cap, 200, "OK", body, off);
    } else {
        a = respond(out, out_cap, 404, "Not Found", BODY("not found\n"));
    }
    free(path);
    return a;
}

static tcp_action http_handler(uint64_t conn_id, const void *data, size_t len,
                               void *out, size_t out_cap, void *user) {
    char reqbuf[HTTP_SRV_REQ_MAX];
    HttpHeader headers[HTTP_SRV_MAX_HEADERS];
    HttpRequestHead req;
    (void)conn_id;
    (void)user;

    if (len == 0 || len >= sizeof(reqbuf)) {
        return respond(out, out_cap, 400, "Bad Request", BODY("bad request\n"));
    }
    memcpy(reqbuf, data, len);
    reqbuf[len] = '\0';
    if (parse_request(reqbuf, &req, headers, HTTP_SRV_MAX_HEADERS) != 0) {
        return respond(out, out_cap, 400, "Bad Request", BODY("bad request\n"));
    }
    return route(&req, out, out_cap);
}

/* ── the server: a thin wrapper over tcp_runtime ───────────────────────── */

struct http_server {
    tcp_runtime *rt;
};

osp_status http_server_bind(http_server **out, const char *host,
                            unsigned short port) {
    struct http_server *s;
    osp_status st;
    if (out == NULL || host == NULL) {
        return OSP_ERR_INVAL;
    }
    s = (struct http_server *)malloc(sizeof(*s));
    if (s == NULL) {
        return OSP_ERR_NOMEM;
    }
    st = tcp_runtime_bind(&s->rt, host, port, http_handler, NULL);
    if (st != OSP_OK) {
        free(s);
        return st;
    }
    *out = s;
    return OSP_OK;
}

osp_status http_server_local_port(http_server *s, unsigned short *out_port) {
    if (s == NULL || out_port == NULL) {
        return OSP_ERR_INVAL;
    }
    return tcp_runtime_local_port(s->rt, out_port);
}

osp_status http_server_poll(http_server *s, int timeout_ms, int *out_handled) {
    if (s == NULL || out_handled == NULL) {
        return OSP_ERR_INVAL;
    }
    return tcp_runtime_poll(s->rt, timeout_ms, out_handled);
}

osp_status http_server_serve(http_server *s) {
    if (s == NULL) {
        return OSP_ERR_INVAL;
    }
    return tcp_runtime_serve(s->rt);
}

void http_server_stop(http_server *s) {
    if (s != NULL) {
        tcp_runtime_stop(s->rt);
    }
}

osp_status http_server_destroy(http_server *s) {
    if (s == NULL) {
        return OSP_ERR_INVAL;
    }
    tcp_runtime_destroy(s->rt);
    free(s);
    return OSP_OK;
}
