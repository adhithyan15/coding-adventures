/*
 * net_test.c — a loopback TCP echo round-trip for net/tcp, on every OS.
 * ===========================================================================
 *
 * This is the real integration test the CCPP02 plan calls for: open an actual
 * socket and exchange bytes. It runs in a SINGLE thread — possible because on
 * the loopback interface a blocking connect() completes the handshake before it
 * returns (the connection waits in the listener's accept queue), and a few bytes
 * always fit in the socket buffer, so send() never blocks waiting for the peer:
 *
 *     listen(127.0.0.1:0) → read back the ephemeral port
 *     connect(127.0.0.1:port)         ← handshake done here
 *     accept()                        ← dequeues the pending connection
 *     client → send;  server → recv   (bytes match)
 *     server → send (echo);  client → recv   (bytes match)
 *     close(client);  server → recv == 0   (sees the orderly shutdown)
 *
 * Then NULL / malformed-argument validation.
 */
#include "iso_test.h"

#include "net/tcp.h"

#include <stddef.h> /* NULL */

int main(void) {
    osp_socket *listener = NULL;
    osp_socket *client = NULL;
    osp_socket *conn = NULL;
    unsigned short port = 0;
    const char msg[] = "ping-echo";
    const size_t msglen = sizeof(msg) - 1; /* 9 bytes, no terminator */
    char buf[64];
    char buf2[64];
    size_t n = 0;

    ISO_CHECK(osp_net_init() == OSP_OK);

    /* ── listen on an ephemeral loopback port ───────────────────────────── */
    ISO_CHECK(osp_tcp_listen(&listener, "127.0.0.1", 0, 1) == OSP_OK);
    ISO_CHECK(osp_tcp_local_port(listener, &port) == OSP_OK);
    ISO_CHECK_MSG(port != 0, "an ephemeral port should have been assigned");

    /* ── connect, then accept the queued connection ─────────────────────── */
    ISO_CHECK(osp_tcp_connect(&client, "127.0.0.1", port) == OSP_OK);
    ISO_CHECK(osp_tcp_accept(listener, &conn) == OSP_OK);

    /* ── client → server ────────────────────────────────────────────────── */
    ISO_CHECK(osp_socket_send(client, msg, msglen, &n) == OSP_OK);
    ISO_CHECK_EQ_UINT(n, msglen);
    n = 0;
    ISO_CHECK(osp_socket_recv(conn, buf, sizeof(buf), &n) == OSP_OK);
    ISO_CHECK_EQ_UINT(n, msglen);
    ISO_CHECK_MEM_EQ(buf, msg, msglen);

    /* ── server echoes → client ─────────────────────────────────────────── */
    ISO_CHECK(osp_socket_send(conn, buf, msglen, NULL) == OSP_OK);
    n = 0;
    ISO_CHECK(osp_socket_recv(client, buf2, sizeof(buf2), &n) == OSP_OK);
    ISO_CHECK_EQ_UINT(n, msglen);
    ISO_CHECK_MEM_EQ(buf2, msg, msglen);

    /* ── orderly shutdown: closing the client makes the server's recv see 0 ── */
    ISO_CHECK(osp_socket_close(client) == OSP_OK);
    client = NULL;
    n = 123;
    ISO_CHECK(osp_socket_recv(conn, buf, sizeof(buf), &n) == OSP_OK);
    ISO_CHECK_MSG(n == 0, "recv after peer close must report 0 bytes");

    ISO_CHECK(osp_socket_close(conn) == OSP_OK);
    ISO_CHECK(osp_socket_close(listener) == OSP_OK);

    /* ── argument validation ────────────────────────────────────────────── */
    ISO_CHECK(osp_tcp_listen(NULL, "127.0.0.1", 0, 1) == OSP_ERR_INVAL);
    ISO_CHECK(osp_tcp_listen(&listener, NULL, 0, 1) == OSP_ERR_INVAL);
    ISO_CHECK(osp_tcp_listen(&listener, "not.an.ip.addr", 0, 1) == OSP_ERR_INVAL);
    ISO_CHECK(osp_tcp_connect(NULL, "127.0.0.1", 80) == OSP_ERR_INVAL);
    ISO_CHECK(osp_socket_recv(NULL, buf, sizeof(buf), &n) == OSP_ERR_INVAL);
    ISO_CHECK(osp_socket_close(NULL) == OSP_ERR_INVAL);

    ISO_CHECK(osp_net_shutdown() == OSP_OK);
    return ISO_TEST_RESULT();
}
