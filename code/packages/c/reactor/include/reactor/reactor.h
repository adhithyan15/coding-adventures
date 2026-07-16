/*
 * reactor/reactor.h — wait for many sockets at once (readiness notification).
 * ===========================================================================
 *
 * CCPP02 Phase 3, the companion to `net`. Blocking one thread per connection
 * does not scale; a *reactor* lets a single thread watch many descriptors and
 * wake only for the ones that are ready to read or write. That readiness query
 * is an OS service with no ISO C form:
 *
 *      mechanism   macOS / Linux    Windows
 *      ──────────  ───────────────  ──────────
 *      readiness   poll()           WSAPoll()
 *
 * SCOPE. This first cut uses poll()/WSAPoll — the portable readiness primitive,
 * with identical semantics on every OS and (unlike epoll/kqueue/IOCP) verifiable
 * on a developer machine. The scalable, edge-triggered backends the plan names —
 * epoll (Linux), kqueue (macOS), IOCP (Windows) — are a drop-in follow-up behind
 * this same interface; poll() is O(n) per wait but correct and universal.
 *
 * MODEL. Register descriptors with osp_reactor_add (each carries an interest
 * mask and an opaque token), then call osp_reactor_wait, which blocks up to a
 * timeout and returns the tokens of the ready descriptors and what they are
 * ready for. osp_reactor_del stops watching one. Destroy frees the reactor.
 *
 * DESCRIPTORS. The watched descriptor is the OS-native type — an int fd on
 * POSIX, a SOCKET (pointer-width) on Windows — captured by the osp_fd typedef
 * below (the single place the two platforms' descriptor types are reconciled).
 *
 * BUILD. Compiled by platform-harness; POSIX links no extra library, Windows
 * links ws2_32 (WSAPoll). Per-OS source selection is done by the BUILD.
 */
#ifndef REACTOR_REACTOR_H
#define REACTOR_REACTOR_H

#include <stdint.h> /* uintptr_t */

#include "os_platform/status.h" /* osp_status */

#ifdef __cplusplus
extern "C" {
#endif

/* The OS-native descriptor: an int file descriptor on POSIX, a SOCKET (which is
 * pointer-width) on Windows. This is the one spot the platforms' descriptor
 * types are unified. */
#ifdef _WIN32
typedef uintptr_t osp_fd;
#else
typedef int osp_fd;
#endif

/* Interest / readiness bits — combine with bitwise OR. */
enum {
    OSP_READABLE = 1, /* ready to read (or accept), POLLIN  */
    OSP_WRITABLE = 2  /* ready to write,             POLLOUT */
};

/* One ready descriptor, returned by osp_reactor_wait. */
typedef struct {
    void *token; /* the token registered for this descriptor */
    int events;  /* OSP_READABLE | OSP_WRITABLE bits that are ready */
} osp_event;

/* Opaque reactor. Created by osp_reactor_create, freed by osp_reactor_destroy. */
typedef struct osp_reactor osp_reactor;

/* Create an empty reactor. OSP_ERR_INVAL / OSP_ERR_NOMEM. */
osp_status osp_reactor_create(osp_reactor **out);

/*
 * osp_reactor_add — watch `fd` for `interest` (OSP_READABLE|OSP_WRITABLE),
 * associating the opaque `token`. Re-adding a descriptor already present updates
 * its interest and token. OSP_ERR_INVAL / OSP_ERR_NOMEM.
 */
osp_status osp_reactor_add(osp_reactor *r, osp_fd fd, int interest, void *token);

/* Stop watching `fd`. OSP_ERR_INVAL; succeeds even if fd was not registered. */
osp_status osp_reactor_del(osp_reactor *r, osp_fd fd);

/*
 * osp_reactor_wait — block up to `timeout_ms` (negative = forever) until at
 * least one watched descriptor is ready, writing up to `max_events` results into
 * `out_events` and the count into *out_count (0 on timeout). OSP_ERR_INVAL /
 * OSP_ERR_OS.
 */
osp_status osp_reactor_wait(osp_reactor *r, osp_event *out_events, int max_events,
                            int timeout_ms, int *out_count);

/* Destroy the reactor and free it. OSP_ERR_INVAL if r is NULL. */
osp_status osp_reactor_destroy(osp_reactor *r);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* REACTOR_REACTOR_H */
