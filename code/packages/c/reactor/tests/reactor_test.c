/*
 * reactor_test.c — readiness-notification test for reactor, on every OS.
 * ===========================================================================
 *
 * We make a connected pair of sockets, register one end for read-readiness, and
 * prove the reactor's behaviour:
 *
 *   - with nothing written, a zero-timeout wait reports 0 ready descriptors;
 *   - after writing to the other end, a wait reports exactly our descriptor
 *     ready-to-read, and hands back the exact token we registered;
 *   - after de-registering it, a wait reports 0 again.
 *
 * The connected pair is made with socketpair() on POSIX and, since Windows has
 * no socketpair, a tiny raw-Winsock loopback connect/accept in make_pair (only
 * in this test — the reactor backends themselves have no #ifdef). Everything
 * else is identical across platforms.
 */
#include "iso_test.h"

#include "reactor/reactor.h"

#include <stddef.h> /* NULL */

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <winsock2.h>
static int pair_send(osp_fd s, const char *b, int n) { return send((SOCKET)s, b, n, 0); }
static int pair_recv(osp_fd s, char *b, int n) { return recv((SOCKET)s, b, n, 0); }
static void pair_close(osp_fd s) { closesocket((SOCKET)s); }
/* Build a connected loopback pair: *ra reads what is written to *rb. */
static int make_pair(osp_fd *ra, osp_fd *rb) {
    SOCKET lis = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    SOCKET cli, conn;
    struct sockaddr_in addr;
    int len = (int)sizeof(addr);
    if (lis == INVALID_SOCKET) { return -1; }
    ZeroMemory(&addr, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    addr.sin_port = 0;
    if (bind(lis, (struct sockaddr *)&addr, sizeof(addr)) == SOCKET_ERROR) { closesocket(lis); return -1; }
    if (listen(lis, 1) == SOCKET_ERROR) { closesocket(lis); return -1; }
    if (getsockname(lis, (struct sockaddr *)&addr, &len) == SOCKET_ERROR) { closesocket(lis); return -1; }
    cli = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (cli == INVALID_SOCKET) { closesocket(lis); return -1; }
    if (connect(cli, (struct sockaddr *)&addr, sizeof(addr)) == SOCKET_ERROR) { closesocket(lis); closesocket(cli); return -1; }
    conn = accept(lis, NULL, NULL);
    closesocket(lis);
    if (conn == INVALID_SOCKET) { closesocket(cli); return -1; }
    *ra = (osp_fd)conn;
    *rb = (osp_fd)cli;
    return 0;
}
#else
#include <sys/socket.h>
#include <unistd.h>
static int pair_send(osp_fd s, const char *b, int n) { return (int)write(s, b, (size_t)n); }
static int pair_recv(osp_fd s, char *b, int n) { return (int)read(s, b, (size_t)n); }
static void pair_close(osp_fd s) { close(s); }
static int make_pair(osp_fd *ra, osp_fd *rb) {
    int sv[2];
    if (socketpair(AF_UNIX, SOCK_STREAM, 0, sv) != 0) { return -1; }
    *ra = sv[0];
    *rb = sv[1];
    return 0;
}
#endif

int main(void) {
    osp_reactor *r = NULL;
    osp_event events[4];
    int count = -1;
    int token = 0; /* its address is the token we register */
    osp_fd a = 0, b = 0;
    char buf[8];

#ifdef _WIN32
    WSADATA wsa;
    ISO_CHECK(WSAStartup(MAKEWORD(2, 2), &wsa) == 0);
#endif

    ISO_CHECK_MSG(make_pair(&a, &b) == 0, "failed to create a connected socket pair");

    ISO_CHECK(osp_reactor_create(&r) == OSP_OK);
    ISO_CHECK(osp_reactor_add(r, a, OSP_READABLE, &token) == OSP_OK);

    /* nothing written yet → immediate wait finds nothing ready */
    count = -1;
    ISO_CHECK(osp_reactor_wait(r, events, 4, 0, &count) == OSP_OK);
    ISO_CHECK_EQ_INT(count, 0);

    /* write on the far end → our descriptor becomes readable */
    ISO_CHECK(pair_send(b, "x", 1) == 1);
    count = -1;
    ISO_CHECK(osp_reactor_wait(r, events, 4, 1000, &count) == OSP_OK);
    ISO_CHECK_EQ_INT(count, 1);
    ISO_CHECK_MSG(events[0].token == &token, "wait must return the registered token");
    ISO_CHECK_MSG((events[0].events & OSP_READABLE) != 0, "descriptor should be readable");

    /* drain the byte, de-register, and confirm nothing is ready anymore */
    ISO_CHECK(pair_recv(a, buf, (int)sizeof(buf)) == 1);
    ISO_CHECK(osp_reactor_del(r, a) == OSP_OK);
    count = -1;
    ISO_CHECK(osp_reactor_wait(r, events, 4, 0, &count) == OSP_OK);
    ISO_CHECK_EQ_INT(count, 0);

    /* ── argument validation ────────────────────────────────────────────── */
    ISO_CHECK(osp_reactor_create(NULL) == OSP_ERR_INVAL);
    ISO_CHECK(osp_reactor_add(NULL, a, OSP_READABLE, NULL) == OSP_ERR_INVAL);
    ISO_CHECK(osp_reactor_wait(NULL, events, 4, 0, &count) == OSP_ERR_INVAL);
    ISO_CHECK(osp_reactor_wait(r, NULL, 4, 0, &count) == OSP_ERR_INVAL);
    ISO_CHECK(osp_reactor_del(NULL, a) == OSP_ERR_INVAL);
    ISO_CHECK(osp_reactor_destroy(NULL) == OSP_ERR_INVAL);

    ISO_CHECK(osp_reactor_destroy(r) == OSP_OK);
    pair_close(a);
    pair_close(b);
#ifdef _WIN32
    WSACleanup();
#endif
    return ISO_TEST_RESULT();
}
