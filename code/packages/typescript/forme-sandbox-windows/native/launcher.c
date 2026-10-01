#define UNICODE
#define _UNICODE
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <bcrypt.h>
#include <aclapi.h>
#include <sddl.h>
#include <userenv.h>
#include <io.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <wchar.h>

#ifndef PROCESS_CREATION_MITIGATION_POLICY_IMAGE_LOAD_NO_REMOTE_ALWAYS_ON
#define PROCESS_CREATION_MITIGATION_POLICY_IMAGE_LOAD_NO_REMOTE_ALWAYS_ON (0x00000001ULL << 52)
#endif
#ifndef PROCESS_CREATION_MITIGATION_POLICY_IMAGE_LOAD_NO_LOW_LABEL_ALWAYS_ON
#define PROCESS_CREATION_MITIGATION_POLICY_IMAGE_LOAD_NO_LOW_LABEL_ALWAYS_ON (0x00000001ULL << 56)
#endif

#define READY_FD 3

static HANDLE verify_and_pin_sha256_file(const wchar_t *path, const wchar_t *expected) {
    if (expected == NULL || wcsncmp(expected, L"sha256:", 7) != 0 || wcslen(expected) != 71) return INVALID_HANDLE_VALUE;
    DWORD attributes = GetFileAttributesW(path);
    if (attributes == INVALID_FILE_ATTRIBUTES || (attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT))) {
        return INVALID_HANDLE_VALUE;
    }
    HANDLE file = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, NULL, OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL | FILE_FLAG_SEQUENTIAL_SCAN, NULL);
    if (file == INVALID_HANDLE_VALUE) return INVALID_HANDLE_VALUE;
    BCRYPT_ALG_HANDLE algorithm = NULL;
    BCRYPT_HASH_HANDLE hash = NULL;
    DWORD object_size = 0, copied = 0;
    PUCHAR object = NULL;
    unsigned char digest[32], buffer[16384];
    int valid = 0;
    if (BCryptOpenAlgorithmProvider(&algorithm, BCRYPT_SHA256_ALGORITHM, NULL, 0) < 0
            || BCryptGetProperty(algorithm, BCRYPT_OBJECT_LENGTH, (PUCHAR)&object_size,
                sizeof(object_size), &copied, 0) < 0) goto done;
    object = HeapAlloc(GetProcessHeap(), 0, object_size);
    if (object == NULL || BCryptCreateHash(algorithm, &hash, object, object_size, NULL, 0, 0) < 0) goto done;
    for (;;) {
        DWORD count = 0;
        if (!ReadFile(file, buffer, sizeof(buffer), &count, NULL)) goto done;
        if (count == 0) break;
        if (BCryptHashData(hash, buffer, count, 0) < 0) goto done;
    }
    if (BCryptFinishHash(hash, digest, sizeof(digest), 0) < 0) goto done;
    wchar_t actual[72] = L"sha256:";
    for (size_t index = 0; index < sizeof(digest); index++) {
        swprintf(actual + 7 + index * 2, 3, L"%02x", digest[index]);
    }
    valid = wcscmp(actual, expected) == 0;
done:
    if (hash != NULL) BCryptDestroyHash(hash);
    if (algorithm != NULL) BCryptCloseAlgorithmProvider(algorithm, 0);
    if (object != NULL) HeapFree(GetProcessHeap(), 0, object);
    if (!valid) {
        CloseHandle(file);
        return INVALID_HANDLE_VALUE;
    }
    return file;
}

static const wchar_t *argument(int argc, wchar_t **argv, const wchar_t *name) {
    size_t length = wcslen(name);
    for (int index = 1; index < argc; index++) {
        if (wcsncmp(argv[index], name, length) == 0 && argv[index][length] == L'=') return argv[index] + length + 1;
    }
    return NULL;
}

static int parse_limit(const wchar_t *text, unsigned long long *value) {
    wchar_t *end = NULL;
    if (text == NULL || *text == L'-') return -1;
    *value = _wcstoui64(text, &end, 10);
    return end == text || *end != L'\0' || *value == 0 ? -1 : 0;
}

static wchar_t *quote(const wchar_t *value) {
    size_t length = wcslen(value);
    wchar_t *result = calloc(length * 2 + 3, sizeof(wchar_t));
    if (result == NULL) return NULL;
    wchar_t *out = result;
    *out++ = L'"';
    size_t backslashes = 0;
    for (const wchar_t *cursor = value;; cursor++) {
        if (*cursor == L'\\') {
            backslashes++;
            continue;
        }
        if (*cursor == L'"' || *cursor == L'\0') {
            for (size_t count = 0; count < backslashes * 2 + (*cursor == L'"'); count++) *out++ = L'\\';
            backslashes = 0;
            if (*cursor == L'"') *out++ = L'"';
            else break;
        } else {
            for (size_t count = 0; count < backslashes; count++) *out++ = L'\\';
            backslashes = 0;
            *out++ = *cursor;
        }
    }
    *out++ = L'"';
    *out = L'\0';
    return result;
}

static int start_profile_janitor(const wchar_t *profile_name) {
    wchar_t executable[MAX_PATH];
    if (GetModuleFileNameW(NULL, executable, MAX_PATH) == 0) return -1;
    HANDLE supervisor = NULL;
    if (!DuplicateHandle(GetCurrentProcess(), GetCurrentProcess(), GetCurrentProcess(),
            &supervisor, SYNCHRONIZE, TRUE, 0)) return -1;
    wchar_t *quoted_executable = quote(executable);
    if (quoted_executable == NULL) {
        CloseHandle(supervisor);
        return -1;
    }
    size_t size = wcslen(quoted_executable) + wcslen(profile_name) + 96;
    wchar_t *command = calloc(size, sizeof(wchar_t));
    if (command == NULL) {
        CloseHandle(supervisor);
        free(quoted_executable);
        return -1;
    }
    swprintf(command, size, L"%s --cleanup-profile=%s --supervisor-handle=%llu",
        quoted_executable, profile_name, (unsigned long long)(ULONG_PTR)supervisor);
    SIZE_T attribute_size = 0;
    InitializeProcThreadAttributeList(NULL, 1, 0, &attribute_size);
    LPPROC_THREAD_ATTRIBUTE_LIST attributes = HeapAlloc(
        GetProcessHeap(), HEAP_ZERO_MEMORY, attribute_size);
    STARTUPINFOEXW startup;
    PROCESS_INFORMATION process;
    ZeroMemory(&startup, sizeof(startup));
    ZeroMemory(&process, sizeof(process));
    startup.StartupInfo.cb = sizeof(startup);
    if (attributes == NULL
            || !InitializeProcThreadAttributeList(attributes, 1, 0, &attribute_size)
            || !UpdateProcThreadAttribute(attributes, 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
                &supervisor, sizeof(supervisor), NULL, NULL)) {
        if (attributes != NULL) HeapFree(GetProcessHeap(), 0, attributes);
        CloseHandle(supervisor);
        free(command);
        free(quoted_executable);
        return -1;
    }
    startup.lpAttributeList = attributes;
    BOOL created = CreateProcessW(executable, command, NULL, NULL, TRUE,
        CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT,
        NULL, NULL, &startup.StartupInfo, &process);
    DeleteProcThreadAttributeList(attributes);
    HeapFree(GetProcessHeap(), 0, attributes);
    CloseHandle(supervisor);
    free(command);
    free(quoted_executable);
    if (!created) return -1;
    CloseHandle(process.hThread);
    CloseHandle(process.hProcess);
    return 0;
}

static int ascii(const wchar_t *source, char *target, size_t size) {
    if (source == NULL || WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, source, -1, target, (int)size, NULL, NULL) == 0) return -1;
    for (const unsigned char *cursor = (const unsigned char *)target; *cursor; cursor++) {
        if (*cursor < 0x20 || *cursor > 0x7e || *cursor == '"' || *cursor == '\\') return -1;
    }
    return 0;
}

static int add_appcontainer_acl(const wchar_t *path, PSID sid) {
    PACL old_acl = NULL;
    PSECURITY_DESCRIPTOR descriptor = NULL;
    DWORD result = GetNamedSecurityInfoW((LPWSTR)path, SE_FILE_OBJECT, DACL_SECURITY_INFORMATION,
        NULL, NULL, &old_acl, NULL, &descriptor);
    if (result != ERROR_SUCCESS) return -1;
    EXPLICIT_ACCESSW access;
    ZeroMemory(&access, sizeof(access));
    access.grfAccessPermissions = GENERIC_READ | GENERIC_WRITE | GENERIC_EXECUTE | DELETE;
    access.grfAccessMode = GRANT_ACCESS;
    access.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
    access.Trustee.TrusteeForm = TRUSTEE_IS_SID;
    access.Trustee.TrusteeType = TRUSTEE_IS_WELL_KNOWN_GROUP;
    access.Trustee.ptstrName = sid;
    PACL new_acl = NULL;
    result = SetEntriesInAclW(1, &access, old_acl, &new_acl);
    if (result == ERROR_SUCCESS) {
        result = SetNamedSecurityInfoW((LPWSTR)path, SE_FILE_OBJECT, DACL_SECURITY_INFORMATION,
            NULL, NULL, new_acl, NULL);
    }
    if (new_acl != NULL) LocalFree(new_acl);
    if (descriptor != NULL) LocalFree(descriptor);
    return result == ERROR_SUCCESS ? 0 : -1;
}

static HANDLE restricted_low_token(void) {
    HANDLE process_token = NULL;
    HANDLE restricted = NULL;
    PSID low_sid = NULL;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_ALL_ACCESS, &process_token)
            || !CreateRestrictedToken(process_token, DISABLE_MAX_PRIVILEGE, 0, NULL, 0, NULL, 0, NULL, &restricted)
            || !ConvertStringSidToSidW(L"S-1-16-4096", &low_sid)) goto fail;
    TOKEN_MANDATORY_LABEL label;
    ZeroMemory(&label, sizeof(label));
    label.Label.Attributes = SE_GROUP_INTEGRITY;
    label.Label.Sid = low_sid;
    DWORD length = sizeof(label) + GetLengthSid(low_sid);
    if (!SetTokenInformation(restricted, TokenIntegrityLevel, &label, length)) goto fail;
    CloseHandle(process_token);
    LocalFree(low_sid);
    return restricted;
fail:
    if (process_token != NULL) CloseHandle(process_token);
    if (restricted != NULL) CloseHandle(restricted);
    if (low_sid != NULL) LocalFree(low_sid);
    return NULL;
}

static HANDLE configured_job(unsigned long long memory, unsigned long long cpu_ms) {
    HANDLE job = CreateJobObjectW(NULL, NULL);
    if (job == NULL) return NULL;
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits;
    ZeroMemory(&limits, sizeof(limits));
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        | JOB_OBJECT_LIMIT_ACTIVE_PROCESS | JOB_OBJECT_LIMIT_PROCESS_MEMORY
        | JOB_OBJECT_LIMIT_JOB_MEMORY | JOB_OBJECT_LIMIT_PROCESS_TIME;
    limits.BasicLimitInformation.ActiveProcessLimit = 1;
    limits.BasicLimitInformation.PerProcessUserTimeLimit.QuadPart = (LONGLONG)cpu_ms * 10000;
    limits.ProcessMemoryLimit = (SIZE_T)memory;
    limits.JobMemoryLimit = (SIZE_T)memory;
    if (!SetInformationJobObject(job, JobObjectExtendedLimitInformation, &limits, sizeof(limits))) {
        CloseHandle(job);
        return NULL;
    }
    return job;
}

static int write_readiness(const wchar_t *manifest, const wchar_t *schema, const wchar_t *entry) {
    char manifest_utf8[512], schema_utf8[512], entry_utf8[512];
    if (ascii(manifest, manifest_utf8, sizeof(manifest_utf8)) != 0
            || ascii(schema, schema_utf8, sizeof(schema_utf8)) != 0
            || ascii(entry, entry_utf8, sizeof(entry_utf8)) != 0) return -1;
    char readiness[2048];
    int length = snprintf(readiness, sizeof(readiness),
        "{\"protocol\":1,\"provider\":\"forme-windows-v1\","
        "\"manifestHash\":\"%s\",\"configSchemaHash\":%s%s%s,\"entryHash\":\"%s\"}\n",
        manifest_utf8, strcmp(schema_utf8, "-") == 0 ? "" : "\"",
        strcmp(schema_utf8, "-") == 0 ? "null" : schema_utf8,
        strcmp(schema_utf8, "-") == 0 ? "" : "\"", entry_utf8);
    if (length <= 0 || (size_t)length >= sizeof(readiness)) return -1;
    return _write(READY_FD, readiness, (unsigned int)length) == length ? 0 : -1;
}

int wmain(int argc, wchar_t **argv) {
    const wchar_t *cleanup_profile = argument(argc, argv, L"--cleanup-profile");
    const wchar_t *supervisor_handle_text = argument(argc, argv, L"--supervisor-handle");
    if (cleanup_profile != NULL || supervisor_handle_text != NULL) {
        wchar_t *end = NULL;
        unsigned long long inherited = _wcstoui64(
            supervisor_handle_text == NULL ? L"" : supervisor_handle_text, &end, 10);
        if (cleanup_profile == NULL || end == supervisor_handle_text || *end != L'\0' || inherited == 0) return 63;
        HANDLE supervisor = (HANDLE)(ULONG_PTR)inherited;
        WaitForSingleObject(supervisor, INFINITE);
        CloseHandle(supervisor);
        for (int attempt = 0; attempt < 100; attempt++) {
            HRESULT deleted = DeleteAppContainerProfile(cleanup_profile);
            if (SUCCEEDED(deleted) || deleted == HRESULT_FROM_WIN32(ERROR_NOT_FOUND)) return 0;
            Sleep(50);
        }
        return 63;
    }
    const wchar_t *provider = argument(argc, argv, L"--provider");
    const wchar_t *manifest_hash = argument(argc, argv, L"--manifest-hash");
    const wchar_t *schema_hash = argument(argc, argv, L"--schema-hash");
    const wchar_t *entry_hash = argument(argc, argv, L"--entry-hash");
    const wchar_t *memory_text = argument(argc, argv, L"--memory-bytes");
    const wchar_t *cpu_text = argument(argc, argv, L"--cpu-ms");
    const wchar_t *wall_text = argument(argc, argv, L"--wall-clock-ms");
    const wchar_t *fd_text = argument(argc, argv, L"--fd-limit");
    const wchar_t *working_directory = argument(argc, argv, L"--working-directory");
    const wchar_t *runtime = argument(argc, argv, L"--runtime");
    const wchar_t *runtime_root = argument(argc, argv, L"--runtime-root");
    const wchar_t *entry = argument(argc, argv, L"--entry");
    const wchar_t *schema = argument(argc, argv, L"--schema");
    const wchar_t *stage = argument(argc, argv, L"--stage");
    if (provider == NULL || wcscmp(provider, L"forme-windows-v1") != 0 || manifest_hash == NULL
            || schema_hash == NULL || entry_hash == NULL || working_directory == NULL
            || runtime == NULL || runtime_root == NULL || entry == NULL || schema == NULL
            || stage == NULL) return 64;
    unsigned long long memory = 0, cpu_ms = 0, wall_ms = 0, descriptor_limit = 0;
    if (parse_limit(memory_text, &memory) != 0 || parse_limit(cpu_text, &cpu_ms) != 0
            || parse_limit(wall_text, &wall_ms) != 0
            || parse_limit(fd_text, &descriptor_limit) != 0 || memory > SIZE_MAX) return 65;
    size_t runtime_root_length = wcslen(runtime_root);
    if (_wcsnicmp(runtime, runtime_root, runtime_root_length) != 0
            || (runtime[runtime_root_length] != L'\\' && runtime[runtime_root_length] != L'/')) return 65;
    HANDLE pinned_entry = verify_and_pin_sha256_file(entry, entry_hash);
    if (pinned_entry == INVALID_HANDLE_VALUE) return 65;
    HANDLE pinned_schema = INVALID_HANDLE_VALUE;
    if (wcscmp(schema_hash, L"-") != 0) {
        pinned_schema = verify_and_pin_sha256_file(schema, schema_hash);
        if (pinned_schema == INVALID_HANDLE_VALUE) {
            CloseHandle(pinned_entry);
            return 65;
        }
    }

    unsigned char profile_nonce[16];
    if (BCryptGenRandom(NULL, profile_nonce, sizeof(profile_nonce), BCRYPT_USE_SYSTEM_PREFERRED_RNG) < 0) {
        CloseHandle(pinned_entry);
        if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 66;
    }
    wchar_t profile_name[128];
    swprintf(profile_name, 128,
        L"Forme.Plugin.%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x",
        (unsigned int)profile_nonce[0], (unsigned int)profile_nonce[1],
        (unsigned int)profile_nonce[2], (unsigned int)profile_nonce[3],
        (unsigned int)profile_nonce[4], (unsigned int)profile_nonce[5],
        (unsigned int)profile_nonce[6], (unsigned int)profile_nonce[7],
        (unsigned int)profile_nonce[8], (unsigned int)profile_nonce[9],
        (unsigned int)profile_nonce[10], (unsigned int)profile_nonce[11],
        (unsigned int)profile_nonce[12], (unsigned int)profile_nonce[13],
        (unsigned int)profile_nonce[14], (unsigned int)profile_nonce[15]);
    PSID app_sid = NULL;
    HRESULT profile = CreateAppContainerProfile(profile_name, profile_name, L"Ephemeral Forme plugin sandbox", NULL, 0, &app_sid);
    if (FAILED(profile) || app_sid == NULL || add_appcontainer_acl(working_directory, app_sid) != 0) {
        if (app_sid != NULL) FreeSid(app_sid);
        if (SUCCEEDED(profile)) DeleteAppContainerProfile(profile_name);
        CloseHandle(pinned_entry);
        if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 66;
    }
    if (start_profile_janitor(profile_name) != 0) {
        FreeSid(app_sid);
        DeleteAppContainerProfile(profile_name);
        CloseHandle(pinned_entry);
        if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 66;
    }

    HANDLE token = restricted_low_token();
    HANDLE job = configured_job(memory, cpu_ms);
    if (token == NULL || job == NULL) {
        if (job != NULL) CloseHandle(job);
        if (token != NULL) CloseHandle(token);
        FreeSid(app_sid);
        DeleteAppContainerProfile(profile_name);
        CloseHandle(pinned_entry);
        if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 67;
    }
    SECURITY_CAPABILITIES security;
    ZeroMemory(&security, sizeof(security));
    security.AppContainerSid = app_sid;

    SIZE_T attribute_size = 0;
    InitializeProcThreadAttributeList(NULL, 3, 0, &attribute_size);
    LPPROC_THREAD_ATTRIBUTE_LIST attributes = HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, attribute_size);
    STARTUPINFOEXW startup;
    ZeroMemory(&startup, sizeof(startup));
    startup.StartupInfo.cb = sizeof(startup);
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.StartupInfo.hStdInput = GetStdHandle(STD_INPUT_HANDLE);
    startup.StartupInfo.hStdOutput = GetStdHandle(STD_OUTPUT_HANDLE);
    startup.StartupInfo.hStdError = GetStdHandle(STD_ERROR_HANDLE);
    HANDLE inherited[] = { startup.StartupInfo.hStdInput, startup.StartupInfo.hStdOutput, startup.StartupInfo.hStdError };
    DWORD64 mitigations = PROCESS_CREATION_MITIGATION_POLICY_DEP_ENABLE
        | PROCESS_CREATION_MITIGATION_POLICY_BOTTOM_UP_ASLR_ALWAYS_ON
        | PROCESS_CREATION_MITIGATION_POLICY_HIGH_ENTROPY_ASLR_ALWAYS_ON
        | PROCESS_CREATION_MITIGATION_POLICY_CONTROL_FLOW_GUARD_ALWAYS_ON
        | PROCESS_CREATION_MITIGATION_POLICY_IMAGE_LOAD_NO_REMOTE_ALWAYS_ON
        | PROCESS_CREATION_MITIGATION_POLICY_IMAGE_LOAD_NO_LOW_LABEL_ALWAYS_ON;
    if (attributes == NULL || !InitializeProcThreadAttributeList(attributes, 3, 0, &attribute_size)
            || !UpdateProcThreadAttribute(attributes, 0, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
                &security, sizeof(security), NULL, NULL)
            || !UpdateProcThreadAttribute(attributes, 0, PROC_THREAD_ATTRIBUTE_MITIGATION_POLICY,
                &mitigations, sizeof(mitigations), NULL, NULL)
            || !UpdateProcThreadAttribute(attributes, 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
                inherited, sizeof(inherited), NULL, NULL)) {
        if (attributes != NULL) HeapFree(GetProcessHeap(), 0, attributes);
        CloseHandle(job);
        CloseHandle(token);
        FreeSid(app_sid);
        DeleteAppContainerProfile(profile_name);
        CloseHandle(pinned_entry);
        if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 68;
    }
    startup.lpAttributeList = attributes;

    wchar_t *quoted_runtime = quote(runtime), *quoted_entry = quote(entry), *quoted_stage = quote(stage), *quoted_schema = quote(schema_hash);
    if (quoted_runtime == NULL || quoted_entry == NULL || quoted_stage == NULL || quoted_schema == NULL) {
        free(quoted_runtime); free(quoted_entry); free(quoted_stage); free(quoted_schema);
        DeleteProcThreadAttributeList(attributes); HeapFree(GetProcessHeap(), 0, attributes);
        CloseHandle(job); CloseHandle(token); FreeSid(app_sid); DeleteAppContainerProfile(profile_name);
        CloseHandle(pinned_entry); if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 69;
    }
    size_t command_size = wcslen(quoted_runtime) + wcslen(quoted_entry) + wcslen(quoted_stage) + wcslen(quoted_schema) + 8;
    wchar_t *command = calloc(command_size, sizeof(wchar_t));
    if (command == NULL) {
        free(quoted_runtime); free(quoted_entry); free(quoted_stage); free(quoted_schema);
        DeleteProcThreadAttributeList(attributes); HeapFree(GetProcessHeap(), 0, attributes);
        CloseHandle(job); CloseHandle(token); FreeSid(app_sid); DeleteAppContainerProfile(profile_name);
        CloseHandle(pinned_entry); if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 69;
    }
    swprintf(command, command_size, L"%s %s %s %s", quoted_runtime, quoted_entry, quoted_stage, quoted_schema);

    PROCESS_INFORMATION process;
    ZeroMemory(&process, sizeof(process));
    DWORD flags = CREATE_SUSPENDED | CREATE_NO_WINDOW | EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT;
    /* The trusted host already started this supervisor in working_directory.
     * Inherit that cwd instead of asking CreateProcessAsUserW to reselect a
     * drive whose hidden `=X:` environment entry was deliberately removed by
     * the host's minimal environment. */
    BOOL created = CreateProcessAsUserW(token, runtime, command, NULL, NULL, TRUE, flags, NULL,
        NULL, &startup.StartupInfo, &process);
    DWORD create_error = created ? ERROR_SUCCESS : GetLastError();
    free(quoted_runtime); free(quoted_entry); free(quoted_stage); free(quoted_schema); free(command);
    if (!created) {
        fwprintf(stderr, L"CreateProcessAsUserW failed: %lu\n", (unsigned long)create_error);
        DeleteProcThreadAttributeList(attributes); HeapFree(GetProcessHeap(), 0, attributes);
        CloseHandle(job); CloseHandle(token); FreeSid(app_sid); DeleteAppContainerProfile(profile_name);
        CloseHandle(pinned_entry); if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 70;
    }
    HANDLE control = (HANDLE)_get_osfhandle(4);
    if (!AssignProcessToJobObject(job, process.hProcess)
            || control == INVALID_HANDLE_VALUE
            || ResumeThread(process.hThread) == (DWORD)-1) {
        TerminateProcess(process.hProcess, 71);
        CloseHandle(process.hThread); CloseHandle(process.hProcess);
        DeleteProcThreadAttributeList(attributes); HeapFree(GetProcessHeap(), 0, attributes);
        CloseHandle(job); CloseHandle(token); FreeSid(app_sid); DeleteAppContainerProfile(profile_name);
        CloseHandle(pinned_entry); if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 71;
    }
    CloseHandle(process.hThread);
    if (write_readiness(manifest_hash, schema_hash, entry_hash) != 0) {
        TerminateJobObject(job, 71);
        CloseHandle(process.hProcess);
        DeleteProcThreadAttributeList(attributes); HeapFree(GetProcessHeap(), 0, attributes);
        CloseHandle(job); CloseHandle(token); FreeSid(app_sid); DeleteAppContainerProfile(profile_name);
        CloseHandle(pinned_entry); if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 71;
    }
    _close(READY_FD);

    DWORD exit_code = STILL_ACTIVE;
    ULONGLONG started = GetTickCount64();
    for (;;) {
        DWORD wait = WaitForSingleObject(process.hProcess, 10);
        if (wait == WAIT_OBJECT_0) break;
        DWORD control_bytes = 0;
        if (!PeekNamedPipe(control, NULL, 0, NULL, &control_bytes, NULL)) {
            TerminateJobObject(job, ERROR_CANCELLED);
            break;
        }
        if (control_bytes > 0) {
            char command_byte = 0;
            _read(4, &command_byte, 1);
            TerminateJobObject(job, ERROR_CANCELLED);
            break;
        }
        DWORD handles = 0;
        if (wait == WAIT_FAILED || !GetProcessHandleCount(process.hProcess, &handles)) {
            TerminateProcess(process.hProcess, 72);
            break;
        }
        if (handles > descriptor_limit) {
            TerminateProcess(process.hProcess, ERROR_TOO_MANY_OPEN_FILES);
            break;
        }
        if (GetTickCount64() - started > wall_ms) {
            TerminateJobObject(job, ERROR_TIMEOUT);
            break;
        }
    }
    GetExitCodeProcess(process.hProcess, &exit_code);
    _close(4);
    CloseHandle(process.hProcess);
    CloseHandle(pinned_entry);
    if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
    CloseHandle(job);
    CloseHandle(token);
    DeleteProcThreadAttributeList(attributes);
    HeapFree(GetProcessHeap(), 0, attributes);
    FreeSid(app_sid);
    DeleteAppContainerProfile(profile_name);
    return (int)exit_code;
}
