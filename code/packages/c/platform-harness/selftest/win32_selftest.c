/*
 * win32_selftest.c — the Windows half of the platform-harness self-test.
 *
 * Includes a Win32 header and links an explicit OS-provided import library
 * (ws2_32.lib via PLATFORM_LIBS): it initialises Winsock, proving the harness
 * compiles Win32 code under /W4 /WX WITHOUT /permissive- and links the named
 * .lib. If this fails, the platform harness or its MSVC flag/link line is broken.
 */
#include "iso_test.h"

#define WIN32_LEAN_AND_MEAN
#include <winsock2.h>
#include <windows.h>

int main(void) {
    WSADATA wsa;
    int rc = WSAStartup(MAKEWORD(2, 2), &wsa);
    ISO_CHECK_EQ_INT(rc, 0);
    if (rc == 0) {
        WSACleanup();
    }
    return ISO_TEST_RESULT();
}
