// win32_selftest.cpp — the Windows C++ half of the platform-harness self-test.
//
// Same proof as win32_selftest.c (Win32 header + explicit ws2_32.lib link under
// /W4 /WX without /permissive-), compiled as C++17.
#include "iso_test.h"

#define WIN32_LEAN_AND_MEAN
#include <winsock2.h>
#include <windows.h>

int main() {
    WSADATA wsa;
    int rc = WSAStartup(MAKEWORD(2, 2), &wsa);
    ISO_CHECK_EQ_INT(rc, 0);
    if (rc == 0) {
        WSACleanup();
    }
    return ISO_TEST_RESULT();
}
