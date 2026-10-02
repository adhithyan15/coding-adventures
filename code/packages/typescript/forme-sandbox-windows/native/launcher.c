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

static int revoke_appcontainer_acl(const wchar_t *path, PSID sid);

static HANDLE acquire_acl_mutex(void) {
    HANDLE mutex = CreateMutexW(NULL, FALSE, L"Local\\Forme.SandboxAcl.v1");
    if (mutex == NULL) return NULL;
    DWORD wait = WaitForSingleObject(mutex, 5000);
    if (wait != WAIT_OBJECT_0 && wait != WAIT_ABANDONED) {
        CloseHandle(mutex);
        return NULL;
    }
    return mutex;
}

static void release_acl_mutex(HANDLE mutex) {
    ReleaseMutex(mutex);
    CloseHandle(mutex);
}

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

static int start_profile_janitor(
    const wchar_t *profile_name,
    const wchar_t *runtime,
    const wchar_t *entry
) {
    wchar_t executable[MAX_PATH];
    wchar_t safe_cwd[MAX_PATH];
    DWORD executable_length = GetModuleFileNameW(NULL, executable, MAX_PATH);
    UINT safe_cwd_length = GetWindowsDirectoryW(safe_cwd, MAX_PATH);
    if (executable_length == 0 || executable_length >= MAX_PATH
            || safe_cwd_length == 0 || safe_cwd_length >= MAX_PATH) return -1;
    HANDLE supervisor = NULL;
    if (!DuplicateHandle(GetCurrentProcess(), GetCurrentProcess(), GetCurrentProcess(),
            &supervisor, SYNCHRONIZE, TRUE, 0)) return -1;
    wchar_t *quoted_executable = quote(executable);
    wchar_t *quoted_runtime = quote(wcscmp(runtime, entry) == 0 ? L"-" : runtime);
    if (quoted_executable == NULL || quoted_runtime == NULL) {
        CloseHandle(supervisor);
        free(quoted_runtime);
        free(quoted_executable);
        return -1;
    }
    size_t size = wcslen(quoted_executable) + wcslen(profile_name) + wcslen(quoted_runtime) + 128;
    wchar_t *command = calloc(size, sizeof(wchar_t));
    if (command == NULL) {
        CloseHandle(supervisor);
        free(quoted_runtime);
        free(quoted_executable);
        return -1;
    }
    swprintf(command, size,
        L"%s --cleanup-profile=%s --cleanup-runtime=%s --supervisor-handle=%llu",
        quoted_executable, profile_name, quoted_runtime,
        (unsigned long long)(ULONG_PTR)supervisor);
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
        free(quoted_runtime);
        free(quoted_executable);
        return -1;
    }
    startup.lpAttributeList = attributes;
    BOOL created = CreateProcessW(executable, command, NULL, NULL, TRUE,
        CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT,
        NULL, safe_cwd, &startup.StartupInfo, &process);
    DeleteProcThreadAttributeList(attributes);
    HeapFree(GetProcessHeap(), 0, attributes);
    CloseHandle(supervisor);
    free(command);
    free(quoted_runtime);
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

static int add_appcontainer_acl(
    const wchar_t *path,
    PSID sid,
    DWORD permissions,
    DWORD inheritance
) {
    HANDLE mutex = acquire_acl_mutex();
    if (mutex == NULL) return -1;
    PACL old_acl = NULL;
    PSECURITY_DESCRIPTOR descriptor = NULL;
    DWORD result = GetNamedSecurityInfoW((LPWSTR)path, SE_FILE_OBJECT, DACL_SECURITY_INFORMATION,
        NULL, NULL, &old_acl, NULL, &descriptor);
    if (result != ERROR_SUCCESS) {
        release_acl_mutex(mutex);
        return -1;
    }
    EXPLICIT_ACCESSW access;
    ZeroMemory(&access, sizeof(access));
    access.grfAccessPermissions = permissions;
    access.grfAccessMode = GRANT_ACCESS;
    access.grfInheritance = inheritance;
    access.Trustee.TrusteeForm = TRUSTEE_IS_SID;
    access.Trustee.TrusteeType = TRUSTEE_IS_USER;
    access.Trustee.ptstrName = sid;
    PACL new_acl = NULL;
    result = SetEntriesInAclW(1, &access, old_acl, &new_acl);
    if (result == ERROR_SUCCESS) {
        result = SetNamedSecurityInfoW((LPWSTR)path, SE_FILE_OBJECT, DACL_SECURITY_INFORMATION,
            NULL, NULL, new_acl, NULL);
    }
    if (new_acl != NULL) LocalFree(new_acl);
    if (descriptor != NULL) LocalFree(descriptor);
    release_acl_mutex(mutex);
    return result == ERROR_SUCCESS ? 0 : -1;
}

static int revoke_appcontainer_acl(const wchar_t *path, PSID sid) {
    HANDLE mutex = acquire_acl_mutex();
    if (mutex == NULL) return -1;
    PACL old_acl = NULL;
    PSECURITY_DESCRIPTOR descriptor = NULL;
    DWORD result = GetNamedSecurityInfoW((LPWSTR)path, SE_FILE_OBJECT, DACL_SECURITY_INFORMATION,
        NULL, NULL, &old_acl, NULL, &descriptor);
    if (result != ERROR_SUCCESS) {
        release_acl_mutex(mutex);
        return -1;
    }
    EXPLICIT_ACCESSW access;
    ZeroMemory(&access, sizeof(access));
    access.grfAccessMode = REVOKE_ACCESS;
    access.Trustee.TrusteeForm = TRUSTEE_IS_SID;
    access.Trustee.TrusteeType = TRUSTEE_IS_USER;
    access.Trustee.ptstrName = sid;
    PACL new_acl = NULL;
    result = SetEntriesInAclW(1, &access, old_acl, &new_acl);
    if (result == ERROR_SUCCESS) {
        result = SetNamedSecurityInfoW((LPWSTR)path, SE_FILE_OBJECT, DACL_SECURITY_INFORMATION,
            NULL, NULL, new_acl, NULL);
    }
    if (new_acl != NULL) LocalFree(new_acl);
    LocalFree(descriptor);
    release_acl_mutex(mutex);
    return result == ERROR_SUCCESS ? 0 : -1;
}

static int cleanup_appcontainer(
    const wchar_t *profile_name,
    PSID sid,
    const wchar_t *runtime,
    const wchar_t *entry
) {
    if (sid == NULL) return -1;
    int revoked = wcscmp(runtime, entry) == 0;
    for (int attempt = 0; !revoked && attempt < 100; attempt++) {
        revoked = revoke_appcontainer_acl(runtime, sid) == 0;
        if (!revoked) Sleep(50);
    }
    int deleted = 0;
    for (int attempt = 0; revoked && !deleted && attempt < 100; attempt++) {
        HRESULT result = DeleteAppContainerProfile(profile_name);
        deleted = SUCCEEDED(result) || result == HRESULT_FROM_WIN32(ERROR_NOT_FOUND);
        if (!deleted) Sleep(50);
    }
    FreeSid(sid);
    return revoked && deleted ? 0 : -1;
}

static int protect_dacl(const wchar_t *path) {
    PACL acl = NULL;
    PSECURITY_DESCRIPTOR descriptor = NULL;
    DWORD result = GetNamedSecurityInfoW((LPWSTR)path, SE_FILE_OBJECT,
        DACL_SECURITY_INFORMATION, NULL, NULL, &acl, NULL, &descriptor);
    if (result != ERROR_SUCCESS) return -1;
    result = SetNamedSecurityInfoW((LPWSTR)path, SE_FILE_OBJECT,
        DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
        NULL, NULL, acl, NULL);
    LocalFree(descriptor);
    return result == ERROR_SUCCESS ? 0 : -1;
}

static wchar_t *parent_path(const wchar_t *path) {
    wchar_t *parent = _wcsdup(path);
    if (parent == NULL) return NULL;
    wchar_t *backslash = wcsrchr(parent, L'\\');
    wchar_t *slash = wcsrchr(parent, L'/');
    wchar_t *separator = backslash == NULL ? slash
        : (slash == NULL || backslash > slash ? backslash : slash);
    if (separator == NULL || separator == parent) {
        free(parent);
        return NULL;
    }
    *separator = L'\0';
    return parent;
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

#define ACL_TREE_MAX_ENTRIES 4096
#define ACL_TREE_MAX_DEPTH 64

static int trusted_write_sid(PSID sid, PSID user, PSID administrators, PSID system_sid) {
    return EqualSid(sid, user) || EqualSid(sid, administrators) || EqualSid(sid, system_sid);
}

static PSID allowed_ace_sid(void *raw, BYTE type) {
    if (type == ACCESS_ALLOWED_ACE_TYPE || type == ACCESS_ALLOWED_CALLBACK_ACE_TYPE) {
        return (PSID)&((ACCESS_ALLOWED_ACE *)raw)->SidStart;
    }
    if (type == ACCESS_ALLOWED_OBJECT_ACE_TYPE || type == ACCESS_ALLOWED_CALLBACK_OBJECT_ACE_TYPE) {
        ACCESS_ALLOWED_OBJECT_ACE *ace = (ACCESS_ALLOWED_OBJECT_ACE *)raw;
        BYTE *cursor = (BYTE *)&ace->ObjectType;
        if (ace->Flags & ACE_OBJECT_TYPE_PRESENT) cursor += sizeof(GUID);
        if (ace->Flags & ACE_INHERITED_OBJECT_TYPE_PRESENT) cursor += sizeof(GUID);
        return (PSID)cursor;
    }
    return NULL;
}

static int verify_handle_acl(HANDLE handle, PSID user, PSID administrators, PSID system_sid) {
    PSID owner = NULL;
    PACL dacl = NULL;
    PSECURITY_DESCRIPTOR descriptor = NULL;
    DWORD result = GetSecurityInfo(handle, SE_FILE_OBJECT,
        OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
        &owner, NULL, &dacl, NULL, &descriptor);
    if (result != ERROR_SUCCESS || descriptor == NULL || owner == NULL || dacl == NULL
            || !IsValidSid(owner) || !IsValidAcl(dacl)
            || !trusted_write_sid(owner, user, administrators, system_sid)) {
        if (descriptor != NULL) LocalFree(descriptor);
        return -1;
    }
    GENERIC_MAPPING mapping = {
        FILE_GENERIC_READ,
        FILE_GENERIC_WRITE,
        FILE_GENERIC_EXECUTE,
        FILE_ALL_ACCESS,
    };
    const DWORD dangerous = FILE_WRITE_DATA | FILE_APPEND_DATA | FILE_WRITE_EA
        | FILE_WRITE_ATTRIBUTES | FILE_DELETE_CHILD | DELETE | WRITE_DAC | WRITE_OWNER;
    for (DWORD index = 0; index < dacl->AceCount; index++) {
        void *raw = NULL;
        if (!GetAce(dacl, index, &raw) || raw == NULL) {
            LocalFree(descriptor);
            return -1;
        }
        ACE_HEADER *header = (ACE_HEADER *)raw;
        if (header->AceFlags & INHERIT_ONLY_ACE) continue;
        PSID sid = allowed_ace_sid(raw, header->AceType);
        if (sid == NULL) {
            if (header->AceType == ACCESS_DENIED_ACE_TYPE
                    || header->AceType == ACCESS_DENIED_OBJECT_ACE_TYPE
                    || header->AceType == ACCESS_DENIED_CALLBACK_ACE_TYPE
                    || header->AceType == ACCESS_DENIED_CALLBACK_OBJECT_ACE_TYPE) continue;
            LocalFree(descriptor);
            return -1;
        }
        DWORD mask = ((ACCESS_ALLOWED_ACE *)raw)->Mask;
        MapGenericMask(&mask, &mapping);
        if ((mask & dangerous) != 0
                && (!IsValidSid(sid) || !trusted_write_sid(sid, user, administrators, system_sid))) {
            LocalFree(descriptor);
            return -1;
        }
    }
    LocalFree(descriptor);
    return 0;
}

static int same_file_identity(const BY_HANDLE_FILE_INFORMATION *left, const BY_HANDLE_FILE_INFORMATION *right) {
    return left->dwVolumeSerialNumber == right->dwVolumeSerialNumber
        && left->nFileIndexHigh == right->nFileIndexHigh
        && left->nFileIndexLow == right->nFileIndexLow;
}

static int verify_acl_tree(
    const wchar_t *path,
    int recurse,
    unsigned int depth,
    unsigned int *entries,
    PSID user,
    PSID administrators,
    PSID system_sid
) {
    if (depth > ACL_TREE_MAX_DEPTH || ++*entries > ACL_TREE_MAX_ENTRIES) return -1;
    HANDLE handle = CreateFileW(path, FILE_READ_ATTRIBUTES | READ_CONTROL,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, NULL, OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, NULL);
    if (handle == INVALID_HANDLE_VALUE) return -1;
    BY_HANDLE_FILE_INFORMATION before;
    if (!GetFileInformationByHandle(handle, &before)
            || (before.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT)
            || verify_handle_acl(handle, user, administrators, system_sid) != 0) {
        CloseHandle(handle);
        return -1;
    }
    int is_directory = (before.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
    if (recurse && is_directory) {
        size_t length = wcslen(path);
        if (length > 32760) {
            CloseHandle(handle);
            return -1;
        }
        wchar_t *pattern = calloc(length + 3, sizeof(wchar_t));
        if (pattern == NULL) {
            CloseHandle(handle);
            return -1;
        }
        wcscpy(pattern, path);
        if (length > 0 && path[length - 1] != L'\\' && path[length - 1] != L'/') wcscat(pattern, L"\\");
        wcscat(pattern, L"*");
        WIN32_FIND_DATAW found;
        HANDLE search = FindFirstFileW(pattern, &found);
        free(pattern);
        if (search == INVALID_HANDLE_VALUE && GetLastError() != ERROR_FILE_NOT_FOUND) {
            CloseHandle(handle);
            return -1;
        }
        if (search != INVALID_HANDLE_VALUE) {
            int valid = 1;
            do {
                if (wcscmp(found.cFileName, L".") == 0 || wcscmp(found.cFileName, L"..") == 0) continue;
                size_t child_size = length + wcslen(found.cFileName) + 2;
                wchar_t *child = calloc(child_size, sizeof(wchar_t));
                if (child == NULL) {
                    valid = 0;
                    break;
                }
                wcscpy(child, path);
                if (length > 0 && path[length - 1] != L'\\' && path[length - 1] != L'/') wcscat(child, L"\\");
                wcscat(child, found.cFileName);
                if (verify_acl_tree(child, 1, depth + 1, entries, user, administrators, system_sid) != 0) valid = 0;
                free(child);
                if (!valid) break;
            } while (FindNextFileW(search, &found));
            if (valid && GetLastError() != ERROR_NO_MORE_FILES) valid = 0;
            FindClose(search);
            if (!valid) {
                CloseHandle(handle);
                return -1;
            }
        }
    }
    HANDLE named = CreateFileW(path, FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, NULL, OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT, NULL);
    BY_HANDLE_FILE_INFORMATION after;
    int unchanged = named != INVALID_HANDLE_VALUE
        && GetFileInformationByHandle(named, &after)
        && !(after.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT)
        && same_file_identity(&before, &after);
    if (named != INVALID_HANDLE_VALUE) CloseHandle(named);
    CloseHandle(handle);
    return unchanged ? 0 : -1;
}

static int verify_install_acl(const wchar_t *path, int recurse) {
    HANDLE token = NULL;
    DWORD needed = 0;
    TOKEN_USER *token_user = NULL;
    PSID administrators = NULL, system_sid = NULL;
    int valid = 0;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token)) goto done;
    GetTokenInformation(token, TokenUser, NULL, 0, &needed);
    if (needed == 0 || GetLastError() != ERROR_INSUFFICIENT_BUFFER) goto done;
    token_user = HeapAlloc(GetProcessHeap(), 0, needed);
    if (token_user == NULL
            || !GetTokenInformation(token, TokenUser, token_user, needed, &needed)) goto done;
    SID_IDENTIFIER_AUTHORITY nt = SECURITY_NT_AUTHORITY;
    if (!AllocateAndInitializeSid(&nt, 2, SECURITY_BUILTIN_DOMAIN_RID,
            DOMAIN_ALIAS_RID_ADMINS, 0, 0, 0, 0, 0, 0, &administrators)
            || !AllocateAndInitializeSid(&nt, 1, SECURITY_LOCAL_SYSTEM_RID,
                0, 0, 0, 0, 0, 0, 0, &system_sid)) goto done;
    unsigned int entries = 0;
    valid = verify_acl_tree(path, recurse, 0, &entries, token_user->User.Sid,
        administrators, system_sid) == 0;
done:
    if (administrators != NULL) FreeSid(administrators);
    if (system_sid != NULL) FreeSid(system_sid);
    if (token_user != NULL) HeapFree(GetProcessHeap(), 0, token_user);
    if (token != NULL) CloseHandle(token);
    return valid ? 0 : 1;
}

int wmain(int argc, wchar_t **argv) {
    const wchar_t *verify_acl_path = argument(argc, argv, L"--verify-acl-path");
    const wchar_t *verify_acl_scope = argument(argc, argv, L"--verify-acl-scope");
    if (verify_acl_path != NULL || verify_acl_scope != NULL) {
        if (verify_acl_path == NULL || verify_acl_scope == NULL) return 62;
        if (wcscmp(verify_acl_scope, L"install-root") == 0) return verify_install_acl(verify_acl_path, 0);
        if (wcscmp(verify_acl_scope, L"existing-target-tree") == 0) return verify_install_acl(verify_acl_path, 1);
        return 62;
    }
    const wchar_t *cleanup_profile = argument(argc, argv, L"--cleanup-profile");
    const wchar_t *cleanup_runtime = argument(argc, argv, L"--cleanup-runtime");
    const wchar_t *supervisor_handle_text = argument(argc, argv, L"--supervisor-handle");
    if (cleanup_profile != NULL || cleanup_runtime != NULL || supervisor_handle_text != NULL) {
        wchar_t *end = NULL;
        unsigned long long inherited = _wcstoui64(
            supervisor_handle_text == NULL ? L"" : supervisor_handle_text, &end, 10);
        if (cleanup_profile == NULL || cleanup_runtime == NULL
                || end == supervisor_handle_text || *end != L'\0' || inherited == 0) return 63;
        HANDLE supervisor = (HANDLE)(ULONG_PTR)inherited;
        WaitForSingleObject(supervisor, INFINITE);
        CloseHandle(supervisor);
        PSID cleanup_sid = NULL;
        HRESULT derived = DeriveAppContainerSidFromAppContainerName(cleanup_profile, &cleanup_sid);
        int revoked = wcscmp(cleanup_runtime, L"-") == 0;
        for (int attempt = 0; attempt < 100; attempt++) {
            if (!revoked && SUCCEEDED(derived) && cleanup_sid != NULL) {
                revoked = revoke_appcontainer_acl(cleanup_runtime, cleanup_sid) == 0;
            }
            HRESULT deleted = revoked
                ? DeleteAppContainerProfile(cleanup_profile)
                : E_ACCESSDENIED;
            if (revoked && (SUCCEEDED(deleted)
                    || deleted == HRESULT_FROM_WIN32(ERROR_NOT_FOUND))) {
                if (cleanup_sid != NULL) FreeSid(cleanup_sid);
                return 0;
            }
            Sleep(50);
        }
        if (cleanup_sid != NULL) FreeSid(cleanup_sid);
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
    const wchar_t *runtime_kind = argument(argc, argv, L"--runtime-kind");
    const wchar_t *runtime = argument(argc, argv, L"--runtime");
    const wchar_t *runtime_root = argument(argc, argv, L"--runtime-root");
    const wchar_t *entry = argument(argc, argv, L"--entry");
    const wchar_t *schema = argument(argc, argv, L"--schema");
    const wchar_t *stage = argument(argc, argv, L"--stage");
    if (provider == NULL || wcscmp(provider, L"forme-windows-v1") != 0 || manifest_hash == NULL
            || schema_hash == NULL || entry_hash == NULL || working_directory == NULL
            || runtime_kind == NULL || runtime == NULL || runtime_root == NULL || entry == NULL || schema == NULL
            || stage == NULL) return 64;
    if (wcscmp(runtime_kind, L"node") != 0 && wcscmp(runtime_kind, L"deno") != 0
            && wcscmp(runtime_kind, L"bun") != 0 && wcscmp(runtime_kind, L"python") != 0
            && wcscmp(runtime_kind, L"binary") != 0) return 64;
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
    wchar_t *snapshot_directory = parent_path(entry);
    int acl_failed = FAILED(profile) || app_sid == NULL || snapshot_directory == NULL
        || protect_dacl(snapshot_directory) != 0
        || add_appcontainer_acl(snapshot_directory, app_sid,
            GENERIC_READ | GENERIC_EXECUTE, NO_INHERITANCE) != 0
        || add_appcontainer_acl(entry, app_sid,
            GENERIC_READ | GENERIC_EXECUTE, NO_INHERITANCE) != 0
        || (wcscmp(schema_hash, L"-") != 0
            && add_appcontainer_acl(schema, app_sid, GENERIC_READ, NO_INHERITANCE) != 0)
        || (wcscmp(runtime, entry) != 0
            && add_appcontainer_acl(runtime, app_sid,
                GENERIC_READ | GENERIC_EXECUTE, NO_INHERITANCE) != 0)
        || add_appcontainer_acl(working_directory, app_sid,
            GENERIC_READ | GENERIC_WRITE | GENERIC_EXECUTE | DELETE,
            SUB_CONTAINERS_AND_OBJECTS_INHERIT) != 0;
    free(snapshot_directory);
    if (FAILED(profile) || acl_failed) {
        if (SUCCEEDED(profile)) cleanup_appcontainer(profile_name, app_sid, runtime, entry);
        CloseHandle(pinned_entry);
        if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 66;
    }
    if (start_profile_janitor(profile_name, runtime, entry) != 0) {
        cleanup_appcontainer(profile_name, app_sid, runtime, entry);
        CloseHandle(pinned_entry);
        if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 66;
    }

    HANDLE job = configured_job(memory, cpu_ms);
    if (job == NULL) {
        cleanup_appcontainer(profile_name, app_sid, runtime, entry);
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
        cleanup_appcontainer(profile_name, app_sid, runtime, entry);
        CloseHandle(pinned_entry);
        if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 68;
    }
    startup.lpAttributeList = attributes;

    wchar_t *quoted_runtime = quote(runtime), *quoted_entry = quote(entry), *quoted_stage = quote(stage), *quoted_schema = quote(schema_hash);
    if (quoted_runtime == NULL || quoted_entry == NULL || quoted_stage == NULL || quoted_schema == NULL) {
        free(quoted_runtime); free(quoted_entry); free(quoted_stage); free(quoted_schema);
        DeleteProcThreadAttributeList(attributes); HeapFree(GetProcessHeap(), 0, attributes);
        CloseHandle(job); cleanup_appcontainer(profile_name, app_sid, runtime, entry);
        CloseHandle(pinned_entry); if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 69;
    }
    const wchar_t *runtime_arguments = wcscmp(runtime_kind, L"node") == 0
        ? L"--preserve-symlinks-main "
        : L"";
    size_t command_size = wcslen(quoted_runtime) + wcslen(runtime_arguments)
        + wcslen(quoted_entry) + wcslen(quoted_stage) + wcslen(quoted_schema) + 8;
    wchar_t *command = calloc(command_size, sizeof(wchar_t));
    if (command == NULL) {
        free(quoted_runtime); free(quoted_entry); free(quoted_stage); free(quoted_schema);
        DeleteProcThreadAttributeList(attributes); HeapFree(GetProcessHeap(), 0, attributes);
        CloseHandle(job); cleanup_appcontainer(profile_name, app_sid, runtime, entry);
        CloseHandle(pinned_entry); if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 69;
    }
    swprintf(command, command_size, L"%s %s%s %s %s",
        quoted_runtime, runtime_arguments, quoted_entry, quoted_stage, quoted_schema);

    PROCESS_INFORMATION process;
    ZeroMemory(&process, sizeof(process));
    DWORD flags = CREATE_SUSPENDED | CREATE_NO_WINDOW | EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT;
    /* PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES asks Windows to derive the
     * capability-free, low-integrity AppContainer token and object namespace.
     * The documented unpackaged-AppContainer flow supplies no alternate
     * primary token; Windows uses the caller while deriving the lowbox token. */
    BOOL created = CreateProcessAsUserW(NULL, runtime, command, NULL, NULL, TRUE, flags, NULL,
        NULL, &startup.StartupInfo, &process);
    DWORD create_error = created ? ERROR_SUCCESS : GetLastError();
    free(quoted_runtime); free(quoted_entry); free(quoted_stage); free(quoted_schema); free(command);
    if (!created) {
        fwprintf(stderr, L"CreateProcessAsUserW failed: %lu\n", (unsigned long)create_error);
        DeleteProcThreadAttributeList(attributes); HeapFree(GetProcessHeap(), 0, attributes);
        CloseHandle(job); cleanup_appcontainer(profile_name, app_sid, runtime, entry);
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
        CloseHandle(job); cleanup_appcontainer(profile_name, app_sid, runtime, entry);
        CloseHandle(pinned_entry); if (pinned_schema != INVALID_HANDLE_VALUE) CloseHandle(pinned_schema);
        return 71;
    }
    CloseHandle(process.hThread);
    if (write_readiness(manifest_hash, schema_hash, entry_hash) != 0) {
        TerminateJobObject(job, 71);
        CloseHandle(process.hProcess);
        DeleteProcThreadAttributeList(attributes); HeapFree(GetProcessHeap(), 0, attributes);
        CloseHandle(job); cleanup_appcontainer(profile_name, app_sid, runtime, entry);
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
    DeleteProcThreadAttributeList(attributes);
    HeapFree(GetProcessHeap(), 0, attributes);
    int cleanup_result = cleanup_appcontainer(profile_name, app_sid, runtime, entry);
    return cleanup_result == 0 ? (int)exit_code : 73;
}
