/*
 * tcp_runtime_test.c — drive the reactor TCP server with real loopback clients.
 * ===========================================================================
 *
 * This is the payoff test of the port campaign: it stands up a real tcp_runtime
 * (net listener + reactor) and hits it with actual TCP clients (via net), all in
 * ONE thread by stepping the server with tcp_runtime_poll between client actions
 * — the same single-thread loopback trick net's own test uses.
 *
 * It proves the server MULTIPLEXES: two independent connections are accepted and
 * echoed on one reactor. The handler echoes bytes, and closes the connection when
 * it sees "bye" — exercising the write-and-close path (the client then sees an
 * orderly shutdown on its next recv).
 */
#include "iso_test.h"

#include "net/tcp.h"
#include "tcp_runtime/tcp_runtime.h"

#include <stddef.h> /* NULL */
#include <string.h> /* memcpy, memcmp */

/* Echo handler: reply with the bytes received; close the connection on "bye". */
static tcp_action echo_handler(uint64_t conn_id, const void *data, size_t len,
                               void *out, size_t out_cap, void *user) {
    tcp_action a;
    size_t n = (len < out_cap) ? len : out_cap;
    memcpy(out, data, n);
    a.write_len = n;
    a.close = (n == 3 && memcmp(data, "bye", 3) == 0) ? 1 : 0;
    (void)conn_id;
    (void)user;
    return a;
}

int main(void) {
    tcp_runtime *rt = NULL;
    tcp_runtime *tmp = NULL;
    osp_socket *c1 = NULL;
    osp_socket *c2 = NULL;
    unsigned short port = 0;
    size_t n = 0;
    int handled = 0;
    char buf[64];

    /* ── stand up the server on an ephemeral loopback port ──────────────── */
    ISO_CHECK(tcp_runtime_bind(&rt, "127.0.0.1", 0, echo_handler, NULL) == OSP_OK);
    ISO_CHECK(tcp_runtime_local_port(rt, &port) == OSP_OK);
    ISO_CHECK_MSG(port != 0, "an ephemeral port should have been assigned");

    /* ── two clients connect; two poll steps accept both ────────────────── */
    ISO_CHECK(osp_tcp_connect(&c1, "127.0.0.1", port) == OSP_OK);
    ISO_CHECK(osp_tcp_connect(&c2, "127.0.0.1", port) == OSP_OK);
    ISO_CHECK(tcp_runtime_poll(rt, 500, &handled) == OSP_OK);
    ISO_CHECK(tcp_runtime_poll(rt, 500, &handled) == OSP_OK);

    /* ── client 1 → server echoes back ──────────────────────────────────── */
    ISO_CHECK(osp_socket_send(c1, "hello", 5, &n) == OSP_OK);
    ISO_CHECK(tcp_runtime_poll(rt, 500, &handled) == OSP_OK);
    n = 0;
    ISO_CHECK(osp_socket_recv(c1, buf, sizeof(buf), &n) == OSP_OK);
    ISO_CHECK_EQ_UINT(n, 5u);
    ISO_CHECK_MEM_EQ(buf, "hello", 5);

    /* ── client 2 independently → server echoes (proves multiplexing) ───── */
    ISO_CHECK(osp_socket_send(c2, "world!", 6, &n) == OSP_OK);
    ISO_CHECK(tcp_runtime_poll(rt, 500, &handled) == OSP_OK);
    n = 0;
    ISO_CHECK(osp_socket_recv(c2, buf, sizeof(buf), &n) == OSP_OK);
    ISO_CHECK_EQ_UINT(n, 6u);
    ISO_CHECK_MEM_EQ(buf, "world!", 6);

    /* ── "bye": server echoes then closes; client sees the shutdown ─────── */
    ISO_CHECK(osp_socket_send(c1, "bye", 3, &n) == OSP_OK);
    ISO_CHECK(tcp_runtime_poll(rt, 500, &handled) == OSP_OK);
    n = 0;
    ISO_CHECK(osp_socket_recv(c1, buf, sizeof(buf), &n) == OSP_OK);
    ISO_CHECK_EQ_UINT(n, 3u);
    ISO_CHECK_MEM_EQ(buf, "bye", 3);
    n = 123;
    ISO_CHECK(osp_socket_recv(c1, buf, sizeof(buf), &n) == OSP_OK);
    ISO_CHECK_MSG(n == 0, "server must close the connection after 'bye'");
    ISO_CHECK(osp_socket_close(c1) == OSP_OK);
    c1 = NULL;

    /* ── client 2 disconnects; a poll step lets the server reap it ──────── */
    ISO_CHECK(osp_socket_close(c2) == OSP_OK);
    c2 = NULL;
    ISO_CHECK(tcp_runtime_poll(rt, 500, &handled) == OSP_OK);

    /* ── connection cap: max_connections = 1 refuses the 2nd client ─────── */
    {
        tcp_runtime *capped = NULL;
        osp_socket *k1 = NULL;
        osp_socket *k2 = NULL;
        unsigned short cport = 0;
        size_t nconn = 0;
        size_t cn = 0;
        int h = 0;
        char cbuf[8];

        ISO_CHECK(tcp_runtime_bind(&capped, "127.0.0.1", 0, echo_handler, NULL) ==
                  OSP_OK);
        ISO_CHECK(tcp_runtime_set_max_connections(capped, 1) == OSP_OK);
        ISO_CHECK(tcp_runtime_local_port(capped, &cport) == OSP_OK);

        ISO_CHECK(osp_tcp_connect(&k1, "127.0.0.1", cport) == OSP_OK);
        ISO_CHECK(osp_tcp_connect(&k2, "127.0.0.1", cport) == OSP_OK);
        ISO_CHECK(tcp_runtime_poll(capped, 500, &h) == OSP_OK); /* accepts k1 */
        ISO_CHECK(tcp_runtime_poll(capped, 500, &h) == OSP_OK); /* k2 → refused */

        ISO_CHECK(tcp_runtime_connection_count(capped, &nconn) == OSP_OK);
        ISO_CHECK_EQ_UINT(nconn, 1u); /* only k1 is tracked */

        /* k1 (under the cap) is served normally */
        ISO_CHECK(osp_socket_send(k1, "hi", 2, &cn) == OSP_OK);
        ISO_CHECK(tcp_runtime_poll(capped, 500, &h) == OSP_OK);
        cn = 0;
        ISO_CHECK(osp_socket_recv(k1, cbuf, sizeof(cbuf), &cn) == OSP_OK);
        ISO_CHECK_EQ_UINT(cn, 2u);

        /* k2 (over the cap) was accepted then closed → its recv sees EOF */
        cn = 123;
        ISO_CHECK(osp_socket_recv(k2, cbuf, sizeof(cbuf), &cn) == OSP_OK);
        ISO_CHECK_MSG(cn == 0, "a connection beyond the cap must be closed");

        /* NULL validation for the new accessors */
        ISO_CHECK(tcp_runtime_set_max_connections(NULL, 1) == OSP_ERR_INVAL);
        ISO_CHECK(tcp_runtime_connection_count(NULL, &nconn) == OSP_ERR_INVAL);
        ISO_CHECK(tcp_runtime_connection_count(capped, NULL) == OSP_ERR_INVAL);

        ISO_CHECK(osp_socket_close(k1) == OSP_OK);
        ISO_CHECK(osp_socket_close(k2) == OSP_OK);
        ISO_CHECK(tcp_runtime_destroy(capped) == OSP_OK);
    }

    /* ── mailbox: a "worker" posts outbound bytes by connection id ───────────
     * The mailbox bypasses the read→handler path entirely: nothing is sent BY the
     * client; the runtime drains the queued command on its next poll and writes.
     * We drive that poll directly (single-threaded), which exercises the exact
     * enqueue/drain/free machinery a real worker thread would hit. The first
     * accepted connection has id 1 (next_id starts at 1). */
    {
        tcp_runtime *mrt = NULL;
        tcp_mailbox *mb = NULL;
        osp_socket *m1 = NULL;
        unsigned short mport = 0;
        size_t mn = 0;
        int mh = 0;
        char mbuf[32];

        ISO_CHECK(tcp_runtime_bind(&mrt, "127.0.0.1", 0, echo_handler, NULL) ==
                  OSP_OK);
        ISO_CHECK(tcp_runtime_local_port(mrt, &mport) == OSP_OK);
        mb = tcp_runtime_mailbox(mrt);
        ISO_CHECK_MSG(mb != NULL, "a bound runtime must expose a mailbox");
        ISO_CHECK(tcp_runtime_mailbox(NULL) == NULL);

        ISO_CHECK(osp_tcp_connect(&m1, "127.0.0.1", mport) == OSP_OK);
        ISO_CHECK(tcp_runtime_poll(mrt, 500, &mh) == OSP_OK); /* accepts m1 (id 1) */
        ISO_CHECK(tcp_runtime_connection_count(mrt, &mn) == OSP_OK);
        ISO_CHECK_EQ_UINT(mn, 1u);

        /* post bytes to conn 1; the next poll delivers them (client sent nothing) */
        ISO_CHECK(tcp_mailbox_send(mb, 1, "ping", 4) == OSP_OK);
        ISO_CHECK(tcp_runtime_poll(mrt, 200, &mh) == OSP_OK); /* drains the mailbox */
        mn = 0;
        ISO_CHECK(osp_socket_recv(m1, mbuf, sizeof(mbuf), &mn) == OSP_OK);
        ISO_CHECK_EQ_UINT(mn, 4u);
        ISO_CHECK_MEM_EQ(mbuf, "ping", 4);

        /* send_and_close: bytes delivered, then the connection is closed */
        ISO_CHECK(tcp_mailbox_send_and_close(mb, 1, "bye!", 4) == OSP_OK);
        ISO_CHECK(tcp_runtime_poll(mrt, 200, &mh) == OSP_OK);
        mn = 0;
        ISO_CHECK(osp_socket_recv(m1, mbuf, sizeof(mbuf), &mn) == OSP_OK);
        ISO_CHECK_EQ_UINT(mn, 4u);
        ISO_CHECK_MEM_EQ(mbuf, "bye!", 4);
        mn = 123;
        ISO_CHECK(osp_socket_recv(m1, mbuf, sizeof(mbuf), &mn) == OSP_OK);
        ISO_CHECK_MSG(mn == 0, "send_and_close must close the connection");
        ISO_CHECK(tcp_runtime_connection_count(mrt, &mn) == OSP_OK);
        ISO_CHECK_EQ_UINT(mn, 0u); /* the close removed the connection */
        ISO_CHECK(osp_socket_close(m1) == OSP_OK);
        m1 = NULL;

        /* a command for an unknown/closed id is silently dropped (no crash) */
        ISO_CHECK(tcp_mailbox_send(mb, 999, "x", 1) == OSP_OK);
        ISO_CHECK(tcp_runtime_poll(mrt, 100, &mh) == OSP_OK);
        ISO_CHECK(tcp_runtime_connection_count(mrt, &mn) == OSP_OK);
        ISO_CHECK_EQ_UINT(mn, 0u);

        /* tcp_mailbox_close on a fresh connection (id 2) removes it, no bytes sent */
        {
            osp_socket *m2 = NULL;
            ISO_CHECK(osp_tcp_connect(&m2, "127.0.0.1", mport) == OSP_OK);
            ISO_CHECK(tcp_runtime_poll(mrt, 500, &mh) == OSP_OK); /* accepts m2 (id 2) */
            ISO_CHECK(tcp_runtime_connection_count(mrt, &mn) == OSP_OK);
            ISO_CHECK_EQ_UINT(mn, 1u);
            ISO_CHECK(tcp_mailbox_close(mb, 2) == OSP_OK);
            ISO_CHECK(tcp_runtime_poll(mrt, 200, &mh) == OSP_OK);
            ISO_CHECK(tcp_runtime_connection_count(mrt, &mn) == OSP_OK);
            ISO_CHECK_EQ_UINT(mn, 0u);
            mn = 123;
            ISO_CHECK(osp_socket_recv(m2, mbuf, sizeof(mbuf), &mn) == OSP_OK);
            ISO_CHECK_MSG(mn == 0, "tcp_mailbox_close must close the connection");
            ISO_CHECK(osp_socket_close(m2) == OSP_OK);
        }

        /* argument validation */
        ISO_CHECK(tcp_mailbox_send(NULL, 1, "x", 1) == OSP_ERR_INVAL);
        ISO_CHECK(tcp_mailbox_send(mb, 1, NULL, 4) == OSP_ERR_INVAL);
        ISO_CHECK(tcp_mailbox_send_and_close(NULL, 1, "x", 1) == OSP_ERR_INVAL);
        ISO_CHECK(tcp_mailbox_close(NULL, 1) == OSP_ERR_INVAL);

        /* a command left queued at teardown must be drained+freed by destroy
         * (checked by ASan/leaks: no leak of the node or its payload) */
        ISO_CHECK(tcp_mailbox_send(mb, 1, "left-pending", 12) == OSP_OK);
        ISO_CHECK(tcp_runtime_destroy(mrt) == OSP_OK);
    }

    /* ── stop before serve → serve returns immediately ──────────────────── */
    tcp_runtime_stop(rt);
    ISO_CHECK(tcp_runtime_serve(rt) == OSP_OK);

    /* ── argument validation ────────────────────────────────────────────── */
    ISO_CHECK(tcp_runtime_bind(NULL, "127.0.0.1", 0, echo_handler, NULL) == OSP_ERR_INVAL);
    ISO_CHECK(tcp_runtime_bind(&tmp, NULL, 0, echo_handler, NULL) == OSP_ERR_INVAL);
    ISO_CHECK(tcp_runtime_bind(&tmp, "127.0.0.1", 0, NULL, NULL) == OSP_ERR_INVAL);
    ISO_CHECK(tcp_runtime_local_port(NULL, &port) == OSP_ERR_INVAL);
    ISO_CHECK(tcp_runtime_poll(NULL, 0, &handled) == OSP_ERR_INVAL);
    ISO_CHECK(tcp_runtime_serve(NULL) == OSP_ERR_INVAL);
    ISO_CHECK(tcp_runtime_destroy(NULL) == OSP_ERR_INVAL);
    tcp_runtime_stop(NULL); /* no-op; must not crash */

    ISO_CHECK(tcp_runtime_destroy(rt) == OSP_OK);
    return ISO_TEST_RESULT();
}
