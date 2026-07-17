/*
 * http_server_test.c — drive the HTTP server with a real loopback client.
 * ===========================================================================
 *
 * Stands up a real http_server (tcp_runtime → reactor → net) and speaks HTTP/1.1
 * to it over an actual TCP connection, single-threaded by stepping the server
 * with http_server_poll. Because every response is Connection: close, each
 * request uses a fresh connection. It sends a raw request and asserts the status
 * line and (for 200s) the body — proving the whole stack serves HTTP:
 *
 *     GET /              → 200, "hello from http-server"
 *     GET /echo?msg=pong → 200, "pong"          (query parsed via http-core)
 *     GET /headers       → 200, "Host: test"    (echoes request headers)
 *     GET /nope          → 404
 *     POST /             → 405
 *     GARBAGE            → 400
 */
#include "iso_test.h"

#include "http_server/http_server.h"
#include "net/tcp.h"

#include <stddef.h> /* NULL */
#include <string.h> /* strlen, strncmp, strstr */

/* Send `request` over a fresh connection, step the server twice (accept, then
 * handle+respond+close), recv the response, and assert its status/body. */
static void raw_check(http_server *srv, unsigned short port, const char *request,
                      const char *want_status, const char *want_body,
                      const char *label) {
    osp_socket *cli = NULL;
    char buf[4096];
    size_t n = 0;
    int handled = 0;

    ISO_CHECK(osp_tcp_connect(&cli, "127.0.0.1", port) == OSP_OK);
    ISO_CHECK(osp_socket_send(cli, request, strlen(request), &n) == OSP_OK);
    ISO_CHECK(http_server_poll(srv, 500, &handled) == OSP_OK); /* accept */
    ISO_CHECK(http_server_poll(srv, 500, &handled) == OSP_OK); /* respond */
    n = 0;
    ISO_CHECK(osp_socket_recv(cli, buf, sizeof(buf) - 1, &n) == OSP_OK);
    buf[n] = '\0';
    ISO_CHECK_MSG(strncmp(buf, want_status, strlen(want_status)) == 0, label);
    if (want_body != NULL) {
        ISO_CHECK_MSG(strstr(buf, want_body) != NULL, label);
    }
    ISO_CHECK(osp_socket_close(cli) == OSP_OK);
    (void)http_server_poll(srv, 100, &handled); /* let the server reap it */
}

int main(void) {
    http_server *srv = NULL;
    http_server *tmp = NULL;
    unsigned short port = 0;
    int handled = 0;

    ISO_CHECK(http_server_bind(&srv, "127.0.0.1", 0) == OSP_OK);
    ISO_CHECK(http_server_local_port(srv, &port) == OSP_OK);
    ISO_CHECK_MSG(port != 0, "an ephemeral port should have been assigned");

    raw_check(srv, port, "GET / HTTP/1.1\r\nHost: test\r\n\r\n", "HTTP/1.1 200",
              "hello from http-server", "GET /");
    raw_check(srv, port, "GET /echo?msg=pong HTTP/1.1\r\nHost: test\r\n\r\n",
              "HTTP/1.1 200", "pong", "GET /echo");
    raw_check(srv, port, "GET /headers HTTP/1.1\r\nHost: test\r\n\r\n",
              "HTTP/1.1 200", "Host: test", "GET /headers");
    raw_check(srv, port, "GET /nope HTTP/1.1\r\nHost: test\r\n\r\n",
              "HTTP/1.1 404", NULL, "GET /nope → 404");
    raw_check(srv, port, "POST / HTTP/1.1\r\nHost: test\r\n\r\n", "HTTP/1.1 405",
              NULL, "POST → 405");
    raw_check(srv, port, "GARBAGE\r\n\r\n", "HTTP/1.1 400", NULL, "malformed → 400");

    /* stop before serve → serve returns immediately */
    http_server_stop(srv);
    ISO_CHECK(http_server_serve(srv) == OSP_OK);

    /* argument validation */
    ISO_CHECK(http_server_bind(NULL, "127.0.0.1", 0) == OSP_ERR_INVAL);
    ISO_CHECK(http_server_bind(&tmp, NULL, 0) == OSP_ERR_INVAL);
    ISO_CHECK(http_server_local_port(NULL, &port) == OSP_ERR_INVAL);
    ISO_CHECK(http_server_poll(NULL, 0, &handled) == OSP_ERR_INVAL);
    ISO_CHECK(http_server_serve(NULL) == OSP_ERR_INVAL);
    ISO_CHECK(http_server_destroy(NULL) == OSP_ERR_INVAL);
    http_server_stop(NULL); /* no-op; must not crash */

    ISO_CHECK(http_server_destroy(srv) == OSP_OK);
    return ISO_TEST_RESULT();
}
