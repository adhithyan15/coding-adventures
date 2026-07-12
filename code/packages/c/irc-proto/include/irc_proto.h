/*
 * irc_proto.h — pure IRC message parsing and serialization (RFC 1459), in pure
 * ISO C17. A faithful port of the Rust `irc-proto` crate.
 * ===========================================================================
 *
 * WHAT IT IS. This is the foundation of an IRC stack: it knows nothing about
 * sockets, threads, or buffers — it only converts between the raw text lines of
 * the IRC protocol and structured `IrcMessage` values.
 *
 * THE GRAMMAR (RFC 1459):
 *
 *     message  = [ ":" prefix SPACE ] command [ params ] CRLF
 *     prefix   = servername / ( nick [ "!" user ] [ "@" host ] )
 *     command  = 1*letter / 3digit
 *     params   = 0*14( SPACE middle ) [ SPACE ":" trailing ]
 *     SPACE    = 0x20
 *
 * In practice a message carries an optional prefix, a command, and up to 15
 * parameters — the last of which may contain spaces when introduced by ':'.
 *
 * OWNERSHIP. `irc_parse` fills a caller-provided `IrcMessage` with malloc'd
 * strings; release them with `irc_message_free` (which frees the fields, not the
 * struct). `irc_serialize` returns a malloc'd byte buffer the caller frees.
 *
 * DIVERGENCE FROM RUST. The Rust `parse` returns a `Result`; this port returns
 * an `IrcStatus` and fills the message through an out-parameter. Command
 * upper-casing is ASCII-only (Rust's `to_uppercase` is Unicode-aware) — IRC
 * commands are ASCII letters/digits, so the two agree byte-for-byte in practice.
 *
 * PORTABILITY. Pure ISO C17 — no POSIX `strdup`/`strndup`, no extensions.
 * Builds clean under GCC, Clang, and MSVC with -pedantic-errors / /permissive-
 * and warnings-as-errors.
 */
#ifndef CA_IRC_PROTO_H
#define CA_IRC_PROTO_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* A single parsed IRC protocol message — a plain envelope with three slots:
 *   - prefix  : who sent it (NULL for client-originated messages);
 *   - command : what kind of message it is (ASCII-uppercased);
 *   - params  : the arguments; the trailing param's leading ':' is stripped. */
typedef struct {
    char *prefix;    /* NULL if absent; malloc'd otherwise */
    char *command;   /* malloc'd, uppercased; never NULL after a successful parse */
    char **params;   /* array of `nparams` malloc'd strings (NULL when nparams==0) */
    size_t nparams;
} IrcMessage;

/* Status of a parse. */
typedef enum {
    IRC_OK = 0,
    IRC_ERR_NOMEM,             /* allocation failed */
    IRC_ERR_EMPTY,             /* empty or whitespace-only line */
    IRC_ERR_PREFIX_NO_COMMAND, /* a ':' prefix with no command after it */
    IRC_ERR_NO_COMMAND         /* no command token could be extracted */
} IrcStatus;

/* At most 15 parameters per message (RFC 1459). */
#define IRC_MAX_PARAMS 15

/* Parse one IRC line (with its trailing CRLF already stripped) into *out. On
 * success returns IRC_OK and fills *out (release with irc_message_free); on any
 * error *out is zeroed and left safe to pass to irc_message_free. */
IrcStatus irc_parse(const char *line, IrcMessage *out);

/* Free the malloc'd fields of a parse result and zero the struct. The struct
 * itself is caller-owned (may be on the stack). Do NOT call this on a message
 * whose fields point at string literals. */
void irc_message_free(IrcMessage *msg);

/* Serialize a message to IRC wire format (CRLF-terminated). Returns a malloc'd
 * buffer of `*len_out` bytes (also NUL-terminated for convenience, past the
 * counted length), or NULL on OOM. The trailing param is introduced with ':'
 * when it contains a space, is empty, or itself begins with ':'. */
unsigned char *irc_serialize(const IrcMessage *msg, size_t *len_out);

#ifdef __cplusplus
}
#endif

#endif /* CA_IRC_PROTO_H */
