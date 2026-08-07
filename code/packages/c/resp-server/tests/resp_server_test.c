/*
 * resp_server_test.c — drive the RESP server with a real loopback client.
 * ===========================================================================
 *
 * Stands up a real resp_server (tcp_runtime + reactor + net) and speaks RESP to
 * it over an actual TCP connection, single-threaded by stepping the server with
 * resp_server_poll between client actions. It sends literal RESP request frames
 * and asserts the exact RESP reply bytes — proving the whole stack (net →
 * reactor → tcp-runtime → resp-protocol) serves a real wire protocol:
 *
 *     PING            → +PONG
 *     ECHO hello      → $5 hello
 *     SET foo bar     → +OK        (and GET foo → $3 bar; overwrite works)
 *     GET missing     → $-1        (null bulk)
 *     NOPE            → -ERR …      (RESP error)
 */
#include "iso_test.h"

#include "net/tcp.h"
#include "resp_server/resp_server.h"

#include <stddef.h> /* NULL */
#include <string.h> /* memcmp */

/* Send `req`, step the server, recv the reply, and assert it equals `want`. */
static void expect_reply(resp_server *srv, osp_socket *cli, const char *req,
                         size_t req_len, const char *want, size_t want_len,
                         const char *label) {
    size_t n = 0;
    int handled = 0;
    char buf[256];
    ISO_CHECK(osp_socket_send(cli, req, req_len, &n) == OSP_OK);
    ISO_CHECK(resp_server_poll(srv, 500, &handled) == OSP_OK);
    n = 0;
    ISO_CHECK(osp_socket_recv(cli, buf, sizeof(buf), &n) == OSP_OK);
    ISO_CHECK_EQ_UINT(n, want_len);
    ISO_CHECK_MSG(n == want_len && memcmp(buf, want, want_len) == 0, label);
}

int main(void) {
    resp_server *srv = NULL;
    resp_server *tmp = NULL;
    osp_socket *cli = NULL;
    unsigned short port = 0;
    int handled = 0;

    ISO_CHECK(resp_server_bind(&srv, "127.0.0.1", 0) == OSP_OK);
    ISO_CHECK(resp_server_local_port(srv, &port) == OSP_OK);
    ISO_CHECK_MSG(port != 0, "an ephemeral port should have been assigned");

    ISO_CHECK(osp_tcp_connect(&cli, "127.0.0.1", port) == OSP_OK);
    ISO_CHECK(resp_server_poll(srv, 500, &handled) == OSP_OK); /* accept */

    /* PING → +PONG */
    expect_reply(srv, cli, "*1\r\n$4\r\nPING\r\n", 14, "+PONG\r\n", 7, "PING");
    /* ECHO hello → $5 hello */
    expect_reply(srv, cli, "*2\r\n$4\r\nECHO\r\n$5\r\nhello\r\n", 25, "$5\r\nhello\r\n",
                 11, "ECHO");
    /* SET foo bar → +OK, then GET foo → $3 bar */
    expect_reply(srv, cli, "*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n", 31,
                 "+OK\r\n", 5, "SET");
    expect_reply(srv, cli, "*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n", 22, "$3\r\nbar\r\n", 9,
                 "GET hit");
    /* GET missing → $-1 (null bulk) */
    expect_reply(srv, cli, "*2\r\n$3\r\nGET\r\n$4\r\nnope\r\n", 23, "$-1\r\n", 5,
                 "GET miss");
    /* overwrite: SET foo baz, then GET foo → $3 baz */
    expect_reply(srv, cli, "*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbaz\r\n", 31,
                 "+OK\r\n", 5, "SET overwrite");
    expect_reply(srv, cli, "*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n", 22, "$3\r\nbaz\r\n", 9,
                 "GET after overwrite");

    /* unknown command → a RESP error (begins with '-') */
    {
        size_t n = 0;
        int h = 0;
        char buf[256];
        ISO_CHECK(osp_socket_send(cli, "*1\r\n$4\r\nNOPE\r\n", 14, &n) == OSP_OK);
        ISO_CHECK(resp_server_poll(srv, 500, &h) == OSP_OK);
        n = 0;
        ISO_CHECK(osp_socket_recv(cli, buf, sizeof(buf), &n) == OSP_OK);
        ISO_CHECK_MSG(n > 0 && buf[0] == '-', "unknown command → RESP error");
    }

    ISO_CHECK(osp_socket_close(cli) == OSP_OK);
    cli = NULL;
    ISO_CHECK(resp_server_poll(srv, 500, &handled) == OSP_OK); /* reap */

    /* stop before serve → serve returns immediately */
    resp_server_stop(srv);
    ISO_CHECK(resp_server_serve(srv) == OSP_OK);

    /* argument validation */
    ISO_CHECK(resp_server_bind(NULL, "127.0.0.1", 0) == OSP_ERR_INVAL);
    ISO_CHECK(resp_server_bind(&tmp, NULL, 0) == OSP_ERR_INVAL);
    ISO_CHECK(resp_server_local_port(NULL, &port) == OSP_ERR_INVAL);
    ISO_CHECK(resp_server_poll(NULL, 0, &handled) == OSP_ERR_INVAL);
    ISO_CHECK(resp_server_serve(NULL) == OSP_ERR_INVAL);
    ISO_CHECK(resp_server_destroy(NULL) == OSP_ERR_INVAL);
    resp_server_stop(NULL); /* no-op; must not crash */

    ISO_CHECK(resp_server_destroy(srv) == OSP_OK);
    return ISO_TEST_RESULT();
}
