/*
 * irc_proto.c — implementation of the RFC 1459 IRC message parser/serializer.
 * ===========================================================================
 *
 * Parsing is a small three-stage scan (optional prefix, command, parameters);
 * serialization joins the parts with single spaces and appends CRLF. The only
 * subtlety is the trailing parameter: a token beginning with ':' absorbs the
 * rest of the line (spaces and all), and on output the last parameter is
 * re-introduced with ':' exactly when it would otherwise be ambiguous.
 */
#include "irc_proto.h"

#include <stdlib.h>
#include <string.h>

/* ===========================================================================
 *  Small string helpers (pure ISO — no POSIX strdup/strndup)
 * =========================================================================== */

/* Duplicate the first `n` bytes of `s` as a fresh NUL-terminated string. */
static char *dup_n(const char *s, size_t n) {
    char *out = malloc(n + 1);
    if (!out) return NULL;
    memcpy(out, s, n);
    out[n] = '\0';
    return out;
}

/* Duplicate `s` in full. */
static char *dup_str(const char *s) { return dup_n(s, strlen(s)); }

/* Duplicate the first `n` bytes of `s`, ASCII-uppercasing a–z. IRC commands are
 * ASCII, so this matches Rust's Unicode `to_uppercase` on every real command;
 * any non-ASCII byte is copied verbatim (never mapped). */
static char *dup_ascii_upper(const char *s, size_t n) {
    char *out = malloc(n + 1);
    if (!out) return NULL;
    for (size_t i = 0; i < n; i++) {
        char c = s[i];
        out[i] = (c >= 'a' && c <= 'z') ? (char)(c - 'a' + 'A') : c;
    }
    out[n] = '\0';
    return out;
}

/* Is `c` one of the ASCII whitespace characters Rust's `str::trim` strips (for
 * the space/tab/newline/carriage-return/form-feed/vertical-tab set an IRC line
 * could plausibly carry)? */
static int is_ws(char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' ||
           c == '\v';
}

/* Is the whole line empty or nothing but whitespace? */
static int is_all_ws(const char *s) {
    for (; *s; s++) {
        if (!is_ws(*s)) return 0;
    }
    return 1;
}

/* ===========================================================================
 *  Parsing
 * =========================================================================== */

IrcStatus irc_parse(const char *line, IrcMessage *out) {
    out->prefix = NULL;
    out->command = NULL;
    out->params = NULL;
    out->nparams = 0;

    /* Stage 0: reject empty / whitespace-only input. */
    if (!line || is_all_ws(line)) return IRC_ERR_EMPTY;

    const char *rest = line;

    /* Stage 1: optional prefix. A leading ':' signals a prefix, ending at the
     * first space; a prefix with no following space has no command. */
    char *prefix = NULL;
    if (rest[0] == ':') {
        const char *sp = strchr(rest, ' ');
        if (!sp) return IRC_ERR_PREFIX_NO_COMMAND;
        prefix = dup_n(rest + 1, (size_t)(sp - (rest + 1)));
        if (!prefix) return IRC_ERR_NOMEM;
        rest = sp + 1;
    }

    /* Stage 2: command — the first space-delimited token, ASCII-uppercased. */
    const char *sp = strchr(rest, ' ');
    size_t cmd_len = sp ? (size_t)(sp - rest) : strlen(rest);
    char *command = dup_ascii_upper(rest, cmd_len);
    if (!command) {
        free(prefix);
        return IRC_ERR_NOMEM;
    }
    if (command[0] == '\0') {
        free(prefix);
        free(command);
        return IRC_ERR_NO_COMMAND;
    }
    rest = sp ? sp + 1 : rest + cmd_len; /* points at "" when there was no space */

    /* Stage 3: parameters (at most IRC_MAX_PARAMS). A token beginning with ':'
     * is the trailing parameter and absorbs the rest of the line. */
    char **params = malloc(IRC_MAX_PARAMS * sizeof *params);
    if (!params) {
        free(prefix);
        free(command);
        return IRC_ERR_NOMEM;
    }
    size_t nparams = 0;
    while (rest[0] != '\0') {
        if (rest[0] == ':') {
            char *t = dup_str(rest + 1); /* trailing: ':' stripped */
            if (!t) goto oom;
            params[nparams++] = t;
            break;
        }
        const char *psp = strchr(rest, ' ');
        if (!psp) {
            char *t = dup_str(rest);
            if (!t) goto oom;
            params[nparams++] = t;
            break;
        }
        char *tok = dup_n(rest, (size_t)(psp - rest));
        if (!tok) goto oom;
        params[nparams++] = tok;
        rest = psp + 1;
        if (nparams == IRC_MAX_PARAMS) break;
    }

    if (nparams == 0) {
        free(params); /* keep the no-params case as a clean NULL */
        params = NULL;
    }
    out->prefix = prefix;
    out->command = command;
    out->params = params;
    out->nparams = nparams;
    return IRC_OK;

oom:
    for (size_t i = 0; i < nparams; i++) free(params[i]);
    free(params);
    free(prefix);
    free(command);
    return IRC_ERR_NOMEM;
}

void irc_message_free(IrcMessage *msg) {
    if (!msg) return;
    free(msg->prefix);
    free(msg->command);
    if (msg->params) {
        for (size_t i = 0; i < msg->nparams; i++) free(msg->params[i]);
        free(msg->params);
    }
    msg->prefix = NULL;
    msg->command = NULL;
    msg->params = NULL;
    msg->nparams = 0;
}

/* ===========================================================================
 *  Serialization — a growable byte buffer with overflow-guarded doubling
 * =========================================================================== */

typedef struct {
    unsigned char *data;
    size_t len;
    size_t cap;
    int err; /* sticky OOM / overflow flag */
} ByteBuf;

static void bb_reserve(ByteBuf *b, size_t extra) {
    if (b->err) return;
    /* Room for `extra` more bytes plus a trailing NUL. */
    if (extra > (size_t)-1 - 1 - b->len) {
        b->err = 1;
        return;
    }
    size_t need = b->len + extra + 1;
    if (need <= b->cap) return;
    size_t cap = b->cap ? b->cap : 16;
    while (cap < need) {
        if (cap > ((size_t)-1) / 2) {
            cap = need; /* stop doubling near the ceiling; grow exactly */
            break;
        }
        cap *= 2;
    }
    unsigned char *nd = realloc(b->data, cap);
    if (!nd) {
        b->err = 1;
        return;
    }
    b->data = nd;
    b->cap = cap;
}

static void bb_putc(ByteBuf *b, char c) {
    bb_reserve(b, 1);
    if (b->err) return;
    b->data[b->len++] = (unsigned char)c;
}

static void bb_puts(ByteBuf *b, const char *s) {
    size_t n = strlen(s);
    bb_reserve(b, n);
    if (b->err) return;
    memcpy(b->data + b->len, s, n);
    b->len += n;
}

unsigned char *irc_serialize(const IrcMessage *msg, size_t *len_out) {
    ByteBuf b = {NULL, 0, 0, 0};

    /* Parts are joined with single spaces: [":"+prefix], command, params… */
    int need_space = 0;
    if (msg->prefix) {
        bb_putc(&b, ':');
        bb_puts(&b, msg->prefix);
        need_space = 1;
    }
    if (need_space) bb_putc(&b, ' ');
    bb_puts(&b, msg->command);

    for (size_t i = 0; i < msg->nparams; i++) {
        bb_putc(&b, ' ');
        const char *p = msg->params[i];
        int is_last = (i + 1 == msg->nparams);
        /* The last param is introduced with ':' when it would otherwise be
         * ambiguous — a space inside it, an empty value, or a literal ':'. */
        if (is_last && (strchr(p, ' ') != NULL || p[0] == '\0' || p[0] == ':')) {
            bb_putc(&b, ':');
        }
        bb_puts(&b, p);
    }
    bb_puts(&b, "\r\n");

    if (b.err) {
        free(b.data);
        return NULL;
    }
    b.data[b.len] = '\0'; /* NUL-terminate past the counted length (room reserved) */
    *len_out = b.len;
    return b.data;
}
