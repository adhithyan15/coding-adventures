/*
 * Tests for the C irc-proto parser/serializer, using the header-only iso_test.h
 * harness (pure ISO). Vectors mirror the Rust crate's own unit tests — prefix /
 * command / parameter parsing, the trailing-parameter rule, the 15-parameter
 * cap, error cases, serialization, and round-trips.
 */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "irc_proto.h"

/* Parse `line` expecting success; leaves the message in *m for inspection. */
static void parse_ok(const char *line, IrcMessage *m) {
    ISO_CHECK_MSG(irc_parse(line, m) == IRC_OK, line);
}

/* Assert param `i` equals `want`. */
static void chk_param(const IrcMessage *m, size_t i, const char *want) {
    ISO_CHECK(i < m->nparams);
    if (i < m->nparams) ISO_CHECK_STR_EQ(m->params[i], want);
}

/* Serialize `m` and assert the wire bytes equal `want` (a NUL-terminated
 * string; the serializer NUL-terminates past the counted length). */
static void chk_serialize(const IrcMessage *m, const char *want) {
    size_t len = 0;
    unsigned char *out = irc_serialize(m, &len);
    ISO_CHECK(out != NULL);
    if (out) {
        ISO_CHECK_EQ_UINT(len, (unsigned)strlen(want));
        ISO_CHECK_STR_EQ((const char *)out, want);
        free(out);
    }
}

int main(void) {
    IrcMessage m;

    /* ── simple command with one param ──────────────────────────────────── */
    parse_ok("NICK alice", &m);
    ISO_CHECK(m.prefix == NULL);
    ISO_CHECK_STR_EQ(m.command, "NICK");
    ISO_CHECK_EQ_UINT(m.nparams, 1u);
    chk_param(&m, 0, "alice");
    irc_message_free(&m);

    /* ── command is upper-cased ─────────────────────────────────────────── */
    parse_ok("join #general", &m);
    ISO_CHECK_STR_EQ(m.command, "JOIN");
    chk_param(&m, 0, "#general");
    irc_message_free(&m);

    /* ── server prefix + numeric + trailing ─────────────────────────────── */
    parse_ok(":irc.local 001 alice :Welcome to the network!", &m);
    ISO_CHECK_STR_EQ(m.prefix, "irc.local");
    ISO_CHECK_STR_EQ(m.command, "001");
    ISO_CHECK_EQ_UINT(m.nparams, 2u);
    chk_param(&m, 0, "alice");
    chk_param(&m, 1, "Welcome to the network!");
    irc_message_free(&m);

    /* ── user-mask prefix ───────────────────────────────────────────────── */
    parse_ok(":alice!alice@127.0.0.1 PRIVMSG #chan :hello world", &m);
    ISO_CHECK_STR_EQ(m.prefix, "alice!alice@127.0.0.1");
    ISO_CHECK_STR_EQ(m.command, "PRIVMSG");
    chk_param(&m, 0, "#chan");
    chk_param(&m, 1, "hello world");
    irc_message_free(&m);

    /* ── the trailing param preserves interior spaces verbatim ──────────── */
    parse_ok("PRIVMSG #chan :hello   world   !", &m);
    ISO_CHECK_EQ_UINT(m.nparams, 2u);
    chk_param(&m, 1, "hello   world   !");
    irc_message_free(&m);

    /* ── no params ──────────────────────────────────────────────────────── */
    parse_ok("PING", &m);
    ISO_CHECK_STR_EQ(m.command, "PING");
    ISO_CHECK_EQ_UINT(m.nparams, 0u);
    ISO_CHECK(m.params == NULL);
    irc_message_free(&m);

    /* ── USER command with several middles + trailing ───────────────────── */
    parse_ok("USER alice 0 * :Alice Smith", &m);
    ISO_CHECK_STR_EQ(m.command, "USER");
    ISO_CHECK_EQ_UINT(m.nparams, 4u);
    chk_param(&m, 0, "alice");
    chk_param(&m, 1, "0");
    chk_param(&m, 2, "*");
    chk_param(&m, 3, "Alice Smith");
    irc_message_free(&m);

    /* ── the 15-parameter cap is enforced ───────────────────────────────── */
    {
        /* "CMD x x x … x" with 16 middle tokens → capped at 15. */
        char line[64];
        char *p = line;
        memcpy(p, "CMD", 3);
        p += 3;
        for (int i = 0; i < 16; i++) {
            *p++ = ' ';
            *p++ = 'x';
        }
        *p = '\0';
        parse_ok(line, &m);
        ISO_CHECK_EQ_UINT(m.nparams, (unsigned)IRC_MAX_PARAMS);
        irc_message_free(&m);
    }

    /* ── an empty trailing param ("AWAY :") ─────────────────────────────── */
    parse_ok("AWAY :", &m);
    ISO_CHECK_EQ_UINT(m.nparams, 1u);
    chk_param(&m, 0, "");
    irc_message_free(&m);

    /* ── numeric command with a mask + trailing ─────────────────────────── */
    parse_ok(":server.local 433 * nick :Nickname is already in use", &m);
    ISO_CHECK_STR_EQ(m.command, "433");
    ISO_CHECK_EQ_UINT(m.nparams, 3u);
    chk_param(&m, 0, "*");
    chk_param(&m, 1, "nick");
    chk_param(&m, 2, "Nickname is already in use");
    irc_message_free(&m);

    /* ── error cases ────────────────────────────────────────────────────── */
    {
        IrcMessage e;
        ISO_CHECK(irc_parse("", &e) == IRC_ERR_EMPTY);
        ISO_CHECK(irc_parse("   ", &e) == IRC_ERR_EMPTY);
        ISO_CHECK(irc_parse(":irc.local", &e) == IRC_ERR_PREFIX_NO_COMMAND);
        /* A message left safe to free even after an error. */
        irc_message_free(&e);
    }

    /* ── serialization ──────────────────────────────────────────────────── */
    {
        char *params1[] = {(char *)"alice"};
        IrcMessage s = {NULL, (char *)"NICK", params1, 1};
        chk_serialize(&s, "NICK alice\r\n");
    }
    {
        char *params2[] = {(char *)"#chan", (char *)"hello world"};
        IrcMessage s = {(char *)"alice!alice@host", (char *)"PRIVMSG", params2, 2};
        chk_serialize(&s, ":alice!alice@host PRIVMSG #chan :hello world\r\n");
    }
    {
        IrcMessage s = {NULL, (char *)"PING", NULL, 0};
        chk_serialize(&s, "PING\r\n");
    }
    {
        /* An empty trailing param is re-introduced with ':'. */
        char *paramsE[] = {(char *)""};
        IrcMessage s = {NULL, (char *)"AWAY", paramsE, 1};
        chk_serialize(&s, "AWAY :\r\n");
    }

    /* ── round-trips ────────────────────────────────────────────────────── */
    parse_ok(":alice!alice@host PRIVMSG #chan :hello world", &m);
    chk_serialize(&m, ":alice!alice@host PRIVMSG #chan :hello world\r\n");
    irc_message_free(&m);

    parse_ok("NICK alice", &m);
    chk_serialize(&m, "NICK alice\r\n");
    irc_message_free(&m);

    return ISO_TEST_RESULT();
}
