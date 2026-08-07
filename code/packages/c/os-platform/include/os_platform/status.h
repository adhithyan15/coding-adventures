/*
 * os_platform/status.h — the shared status/error vocabulary for os-platform.
 * ===========================================================================
 *
 * Every os-platform primitive (clock, thread, fs, …) reports success or failure
 * through this one small enum, so a caller learns a single convention and every
 * header agrees on the values. It lives in its own header (rather than inside any
 * one primitive's header) so that including two primitives at once — e.g. both
 * clock.h and thread.h — cannot produce a duplicate `enum osp_status` definition.
 *
 * CONVENTION
 * ----------
 * OSP_OK is 0, so the idiomatic check reads naturally:
 *
 *     if (osp_thread_spawn(&t, worker, arg)) {   // non-zero == it failed
 *         // handle error
 *     }
 *
 * All error codes are negative, leaving the entire positive range free should a
 * primitive ever want to return a small count through the same type.
 */
#ifndef OS_PLATFORM_STATUS_H
#define OS_PLATFORM_STATUS_H

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    OSP_OK = 0,          /* the call succeeded; any out-parameter is valid     */
    OSP_ERR_OS = -1,     /* an underlying OS call reported a failure            */
    OSP_ERR_INVAL = -2,  /* a caller argument was invalid (e.g. a NULL pointer) */
    OSP_ERR_NOMEM = -3   /* an allocation for an OS-object wrapper failed        */
} osp_status;

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* OS_PLATFORM_STATUS_H */
