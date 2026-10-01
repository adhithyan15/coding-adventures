#define UNICODE
#define _UNICODE
#define WIN32_LEAN_AND_MEAN
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#include <stdio.h>
#include <string.h>
#include <wchar.h>

int wmain(int argc, wchar_t **argv) {
    const wchar_t *probe = argc > 2 ? argv[2] : L"";
    if (wcscmp(probe, L"filesystem") == 0) {
        HANDLE file = CreateFileW(L"C:\\Windows\\System32\\config\\SAM", GENERIC_READ, FILE_SHARE_READ,
            NULL, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, NULL);
        if (file != INVALID_HANDLE_VALUE) CloseHandle(file);
        return file == INVALID_HANDLE_VALUE && GetLastError() == ERROR_ACCESS_DENIED ? 0 : 70;
    }
    if (wcscmp(probe, L"network") == 0) {
        WSADATA data;
        if (WSAStartup(MAKEWORD(2, 2), &data) != 0) return 0;
        SOCKET socket_value = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
        struct sockaddr_in destination;
        ZeroMemory(&destination, sizeof(destination));
        destination.sin_family = AF_INET;
        destination.sin_port = htons(443);
        InetPtonW(AF_INET, L"1.1.1.1", &destination.sin_addr);
        int result = connect(socket_value, (struct sockaddr *)&destination, sizeof(destination));
        int error = WSAGetLastError();
        closesocket(socket_value);
        WSACleanup();
        return result == SOCKET_ERROR && error == WSAEACCES ? 0 : 71;
    }
    if (wcscmp(probe, L"process") == 0) {
        STARTUPINFOW startup;
        ZeroMemory(&startup, sizeof(startup));
        startup.cb = sizeof(startup);
        PROCESS_INFORMATION process;
        wchar_t command[] = L"cmd.exe /c exit 0";
        BOOL created = CreateProcessW(NULL, command, NULL, NULL, FALSE, CREATE_NO_WINDOW,
            NULL, NULL, &startup, &process);
        if (created) { TerminateProcess(process.hProcess, 72); CloseHandle(process.hThread); CloseHandle(process.hProcess); }
        return created ? 73 : 0;
    }
    if (wcscmp(probe, L"memory") == 0) {
        SIZE_T size = (SIZE_T)1024 * 1024 * 1024;
        volatile unsigned char *bytes = VirtualAlloc(NULL, size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
        if (bytes == NULL) return 0;
        for (SIZE_T offset = 0; offset < size; offset += 4096) bytes[offset] = 1;
        return 74;
    }
    if (wcscmp(probe, L"descriptors") == 0) {
        for (int index = 0; index < 4096; index++) {
            wchar_t name[64];
            swprintf(name, 64, L"fd-%d", index);
            CreateFileW(name, GENERIC_WRITE, FILE_SHARE_READ, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
        }
        Sleep(1000);
        return 76;
    }
    if (wcscmp(probe, L"environment") == 0) {
        wchar_t *value = NULL;
        size_t length = 0;
        int error = _wdupenv_s(&value, &length, L"FORME_SANDBOX_AMBIENT_SENTINEL");
        int isolated = error == 0 && value == NULL && length == 0;
        free(value);
        return isolated ? 0 : 78;
    }
    if (wcscmp(probe, L"cpu") == 0) {
        volatile unsigned long long value = 0;
        for (;;) value++;
    }
    if (wcscmp(probe, L"wall-clock") == 0) {
        Sleep(10000);
        return 79;
    }
    return 77;
}
