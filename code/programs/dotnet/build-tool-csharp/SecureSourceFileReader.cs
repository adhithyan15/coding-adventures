namespace CodingAdventures.BuildTool.CSharp;

using System.Runtime.InteropServices;
using System.Text;

// Secure source snapshots
// =======================
//
// Source hashing is a trust boundary: the path names come from a mutable
// checkout, while the bytes become cache identity.  A check-then-open sequence
// is therefore insufficient.  This adapter opens every component without
// following the final component, retains the native object while it reads,
// and compares identity plus mutation metadata before returning any bytes.
// The pure selector above this adapter remains filesystem-free.

internal readonly record struct SecureObjectState(
    ulong Device,
    ulong IdentityHigh,
    ulong IdentityLow,
    ulong LinkCount,
    ulong Size,
    ulong ModifiedHigh,
    ulong ModifiedLow,
    ulong ChangedHigh,
    ulong ChangedLow,
    ulong Attributes);

internal readonly record struct SecureFileSnapshot(byte[] Content, SecureObjectState State);

internal enum SecureDirectoryEntryKind
{
    Directory,
    Regular,
    Linked,
    Other,
}

internal readonly record struct SecureDirectoryEntry(string Name, SecureDirectoryEntryKind Kind);

internal static class SecureSourceFileReader
{
    private const int ReadChunkBytes = 64 * 1024;
    private static readonly UTF8Encoding StrictUtf8 = new(
        encoderShouldEmitUTF8Identifier: false,
        throwOnInvalidBytes: true);

    internal static Scope RetainRepositoryRoot(string repositoryRoot) => new(repositoryRoot);

    internal static (int LinuxStatxSize, int LinuxModeOffset, int LinuxSizeOffset, int MacStatSize)
        NativeLayoutForTest() =>
        (
            Marshal.SizeOf<LinuxStatx>(),
            checked((int)Marshal.OffsetOf<LinuxStatx>(nameof(LinuxStatx.Mode))),
            checked((int)Marshal.OffsetOf<LinuxStatx>(nameof(LinuxStatx.Size))),
            Marshal.SizeOf<MacStat>()
        );

    internal sealed class Scope : IDisposable
    {
        private readonly string repositoryRoot;
        private readonly nint windowsRootHandle;
        private readonly int posixRootDescriptor = -1;
        private readonly SecureObjectState rootState;
        private readonly Dictionary<string, SecureObjectState> windowsDirectories =
            new(StringComparer.Ordinal);
        private readonly Dictionary<string, (string[] Components, SecureObjectState State)> posixDirectories =
            new(StringComparer.Ordinal);
        private bool disposed;

        internal Scope(string root)
        {
            repositoryRoot = Path.GetFullPath(root);
            if (OperatingSystem.IsWindows())
            {
                windowsRootHandle = OpenWindowsObject(repositoryRoot, expectDirectory: true);
                rootState = WindowsState(windowsRootHandle, expectDirectory: true);
            }
            else
            {
                posixRootDescriptor = OpenPosixObject(null, repositoryRoot, expectDirectory: true);
                rootState = PosixState(posixRootDescriptor, expectDirectory: true);
            }
        }

        internal int RetainedNativeObjectCountForTest => disposed ? 0 : 1;

        internal IReadOnlyList<SecureDirectoryEntry> EnumerateDirectory(string path, int maximumEntries)
        {
            ObjectDisposedException.ThrowIf(disposed, this);
            if (maximumEntries < 0)
            {
                throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
            }
            return OperatingSystem.IsWindows()
                ? EnumerateWindowsDirectory(path, maximumEntries)
                : EnumeratePosixDirectory(path, maximumEntries);
        }

        internal SecureFileSnapshot ReadFile(
            string path,
            ulong maximumFileBytes,
            ulong remainingPackageBytes)
        {
            ObjectDisposedException.ThrowIf(disposed, this);
            SecureFileSnapshot ReadWindows(nint handle, SecureObjectState before)
            {
                EnsureReadableSize(before.Size, maximumFileBytes, remainingPackageBytes);
                return new SecureFileSnapshot(ReadWindowsBytes(handle, before.Size), before);
            }

            SecureFileSnapshot ReadPosix(int descriptor, SecureObjectState before)
            {
                EnsureReadableSize(before.Size, maximumFileBytes, remainingPackageBytes);
                return new SecureFileSnapshot(ReadPosixBytes(descriptor, before.Size), before);
            }

            var components = RelativeComponents(path, repositoryRoot);
            if (OperatingSystem.IsWindows())
            {
                RecordWindowsDirectories(components[..^1]);
                return WithSecureWindowsObject(
                    path,
                    repositoryRoot,
                    expectDirectory: false,
                    ReadWindows,
                    windowsRootHandle);
            }
            RecordPosixDirectories(components[..^1]);
            return WithSecurePosixObjectBelowRoot(
                components,
                expectDirectory: false,
                ReadPosix);
        }

        internal SecureObjectState FileState(string path)
        {
            var components = RelativeComponents(path, repositoryRoot);
            if (OperatingSystem.IsWindows())
            {
                RecordWindowsDirectories(components[..^1]);
                return WithSecureWindowsObject(
                    path,
                    repositoryRoot,
                    expectDirectory: false,
                    static (_, state) => state,
                    windowsRootHandle);
            }
            RecordPosixDirectories(components[..^1]);
            return WithSecurePosixObjectBelowRoot(
                    components,
                    expectDirectory: false,
                    static (_, state) => state);
        }

        internal void Validate()
        {
            ObjectDisposedException.ThrowIf(disposed, this);
            if (OperatingSystem.IsWindows())
            {
                using var reopenedRoot = new WindowsHandleOwner(
                    OpenWindowsObject(repositoryRoot, expectDirectory: true));
                if (WindowsState(windowsRootHandle, expectDirectory: true) != rootState ||
                    WindowsState(reopenedRoot.Handle, expectDirectory: true) != rootState)
                {
                    throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
                }
                foreach (var item in windowsDirectories)
                {
                    using var reopened = new WindowsHandleOwner(
                        OpenWindowsObject(item.Key, expectDirectory: true));
                    if (WindowsState(reopened.Handle, expectDirectory: true) != item.Value)
                    {
                        throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
                    }
                }
            }
            else
            {
                if (PosixState(posixRootDescriptor, expectDirectory: true) != rootState)
                {
                    throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
                }
                foreach (var item in posixDirectories.Values)
                {
                    var reopened = OpenPosixBelowRoot(
                        posixRootDescriptor,
                        item.Components,
                        expectDirectory: true);
                    try
                    {
                        if (PosixState(reopened, expectDirectory: true) != item.State)
                        {
                            throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
                        }
                    }
                    finally
                    {
                        _ = Close(reopened);
                    }
                }
            }
        }

        public void Dispose()
        {
            if (disposed)
            {
                return;
            }
            disposed = true;
            if (OperatingSystem.IsWindows())
            {
                _ = CloseHandle(windowsRootHandle);
            }
            else
            {
                _ = Close(posixRootDescriptor);
            }
        }

        private IReadOnlyList<SecureDirectoryEntry> EnumerateWindowsDirectory(
            string path,
            int maximumEntries)
        {
            var components = RelativeComponents(path, repositoryRoot);
            RecordWindowsDirectories(components[..^1]);
            var snapshot = WithSecureWindowsObject(
                path,
                repositoryRoot,
                expectDirectory: true,
                (handle, state) => (Entries: ReadWindowsDirectoryEntries(handle, maximumEntries), State: state),
                windowsRootHandle);
            RecordWindowsDirectory(Path.GetFullPath(path), snapshot.State);
            return snapshot.Entries;
        }

        private static IReadOnlyList<SecureDirectoryEntry> ReadWindowsDirectoryEntries(
            nint handle,
            int maximumEntries)
        {
            var entries = new List<SecureDirectoryEntry>();
            var buffer = new byte[64 * 1024];
            var informationClass = FileIdBothDirectoryRestartInfoClass;
            while (true)
            {
                if (!GetFileInformationByHandleExBuffer(
                        handle,
                        informationClass,
                        buffer,
                        checked((uint)buffer.Length)))
                {
                    if (Marshal.GetLastPInvokeError() == WindowsNoMoreFiles)
                    {
                        return entries;
                    }
                    throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
                }
                informationClass = FileIdBothDirectoryInfoClass;
                var offset = 0;
                while (true)
                {
                    if (offset < 0 || offset + FileIdBothDirectoryNameOffset > buffer.Length)
                    {
                        throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
                    }
                    var nextOffset = BitConverter.ToUInt32(buffer, offset);
                    var attributes = BitConverter.ToUInt32(buffer, offset + 56);
                    var nameBytes = checked((int)BitConverter.ToUInt32(buffer, offset + 60));
                    if ((nameBytes & 1) != 0 ||
                        nameBytes <= 0 ||
                        nameBytes > 512 ||
                        offset + FileIdBothDirectoryNameOffset + nameBytes > buffer.Length)
                    {
                        throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
                    }
                    var name = Encoding.Unicode.GetString(
                        buffer,
                        offset + FileIdBothDirectoryNameOffset,
                        nameBytes);
                    if (name is not ("." or ".."))
                    {
                        if (entries.Count >= maximumEntries)
                        {
                            throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
                        }
                        var kind = (attributes & FileAttributeReparsePoint) != 0
                            ? SecureDirectoryEntryKind.Linked
                            : (attributes & FileAttributeDirectory) != 0
                                ? SecureDirectoryEntryKind.Directory
                                : SecureDirectoryEntryKind.Regular;
                        entries.Add(new SecureDirectoryEntry(name, kind));
                    }
                    if (nextOffset == 0)
                    {
                        break;
                    }
                    if (nextOffset < FileIdBothDirectoryNameOffset || nextOffset > int.MaxValue)
                    {
                        throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
                    }
                    offset = checked(offset + (int)nextOffset);
                }
            }
        }

        private IReadOnlyList<SecureDirectoryEntry> EnumeratePosixDirectory(
            string path,
            int maximumEntries)
        {
            var components = RelativeComponents(path, repositoryRoot);
            RecordPosixDirectories(components[..^1]);
            var descriptor = OpenPosixBelowRoot(posixRootDescriptor, components, expectDirectory: true);
            try
            {
                var state = PosixState(descriptor, expectDirectory: true);
                var duplicate = Duplicate(descriptor);
                if (duplicate < 0)
                {
                    throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
                }
                var directory = FileDescriptorOpenDirectory(duplicate);
                if (directory == nint.Zero)
                {
                    _ = Close(duplicate);
                    throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
                }

                var entries = new List<SecureDirectoryEntry>();
                try
                {
                    while (true)
                    {
                        Marshal.SetLastPInvokeError(0);
                        var entry = ReadDirectory(directory);
                        if (entry == nint.Zero)
                        {
                            if (Marshal.GetLastPInvokeError() != 0)
                            {
                                throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
                            }
                            break;
                        }
                        var name = ReadDirectoryEntryName(entry);
                        if (name is "." or "..")
                        {
                            continue;
                        }
                        if (entries.Count >= maximumEntries)
                        {
                            throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
                        }
                        entries.Add(new SecureDirectoryEntry(name, PosixEntryKind(descriptor, name)));
                    }
                }
                finally
                {
                    _ = CloseDirectory(directory);
                }
                if (PosixState(descriptor, expectDirectory: true) != state)
                {
                    throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
                }
                RecordPosixDirectory(components, state);
                return entries;
            }
            finally
            {
                _ = Close(descriptor);
            }
        }

        private void RecordWindowsDirectories(IReadOnlyList<string> components)
        {
            var path = repositoryRoot;
            foreach (var component in components)
            {
                path = Path.Combine(path, component);
                var state = WithSecureWindowsObject(
                    path,
                    repositoryRoot,
                    expectDirectory: true,
                    static (_, openedState) => openedState,
                    windowsRootHandle);
                RecordWindowsDirectory(path, state);
            }
        }

        private void RecordWindowsDirectory(string path, SecureObjectState state)
        {
            var fullPath = Path.GetFullPath(path);
            if (windowsDirectories.TryGetValue(fullPath, out var prior) && prior != state)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
            }
            windowsDirectories[fullPath] = state;
        }

        private void RecordPosixDirectories(IReadOnlyList<string> components)
        {
            for (var count = 1; count <= components.Count; count++)
            {
                var prefix = components.Take(count).ToArray();
                var descriptor = OpenPosixBelowRoot(posixRootDescriptor, prefix, expectDirectory: true);
                try
                {
                    RecordPosixDirectory(prefix, PosixState(descriptor, expectDirectory: true));
                }
                finally
                {
                    _ = Close(descriptor);
                }
            }
        }

        private void RecordPosixDirectory(IReadOnlyList<string> components, SecureObjectState state)
        {
            var copied = components.ToArray();
            var key = string.Join('\0', copied);
            if (posixDirectories.TryGetValue(key, out var prior) && prior.State != state)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
            }
            posixDirectories[key] = (copied, state);
        }

        private T WithSecurePosixObjectBelowRoot<T>(
            IReadOnlyList<string> components,
            bool expectDirectory,
            Func<int, SecureObjectState, T> body)
        {
            var descriptor = OpenPosixBelowRoot(posixRootDescriptor, components, expectDirectory);
            try
            {
                var before = PosixState(descriptor, expectDirectory);
                var result = body(descriptor, before);
                if (PosixState(descriptor, expectDirectory) != before)
                {
                    throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
                }
                var reopened = OpenPosixBelowRoot(posixRootDescriptor, components, expectDirectory);
                try
                {
                    if (PosixState(reopened, expectDirectory) != before)
                    {
                        throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
                    }
                }
                finally
                {
                    _ = Close(reopened);
                }
                return result;
            }
            finally
            {
                _ = Close(descriptor);
            }
        }
    }

    internal static SecureObjectState DirectoryState(string path, string repositoryRoot) =>
        OperatingSystem.IsWindows()
            ? WithSecureWindowsObject(path, repositoryRoot, expectDirectory: true, static (_, state) => state)
            : WithSecurePosixObject(path, repositoryRoot, expectDirectory: true, static (_, state) => state);

    internal static SecureObjectState FileState(string path, string repositoryRoot) =>
        OperatingSystem.IsWindows()
            ? WithSecureWindowsObject(path, repositoryRoot, expectDirectory: false, static (_, state) => state)
            : WithSecurePosixObject(path, repositoryRoot, expectDirectory: false, static (_, state) => state);

    internal static SecureFileSnapshot ReadFile(
        string path,
        string repositoryRoot,
        ulong maximumFileBytes,
        ulong remainingPackageBytes)
    {
        SecureFileSnapshot ReadWindows(nint handle, SecureObjectState before)
        {
            EnsureReadableSize(before.Size, maximumFileBytes, remainingPackageBytes);
            return new SecureFileSnapshot(ReadWindowsBytes(handle, before.Size), before);
        }

        SecureFileSnapshot ReadPosix(int descriptor, SecureObjectState before)
        {
            EnsureReadableSize(before.Size, maximumFileBytes, remainingPackageBytes);
            return new SecureFileSnapshot(ReadPosixBytes(descriptor, before.Size), before);
        }

        return OperatingSystem.IsWindows()
            ? WithSecureWindowsObject(path, repositoryRoot, expectDirectory: false, ReadWindows)
            : WithSecurePosixObject(path, repositoryRoot, expectDirectory: false, ReadPosix);
    }

    internal static SecureFileSnapshot ReadFileForMutationTest(
        string path,
        string repositoryRoot,
        Action afterSnapshot)
    {
        ArgumentNullException.ThrowIfNull(afterSnapshot);
        SecureFileSnapshot ReadWindows(nint handle, SecureObjectState before)
        {
            afterSnapshot();
            return new SecureFileSnapshot(ReadWindowsBytes(handle, before.Size), before);
        }

        SecureFileSnapshot ReadPosix(int descriptor, SecureObjectState before)
        {
            afterSnapshot();
            return new SecureFileSnapshot(ReadPosixBytes(descriptor, before.Size), before);
        }

        return OperatingSystem.IsWindows()
            ? WithSecureWindowsObject(path, repositoryRoot, expectDirectory: false, ReadWindows)
            : WithSecurePosixObject(path, repositoryRoot, expectDirectory: false, ReadPosix);
    }

    private static void EnsureReadableSize(ulong size, ulong maximumFileBytes, ulong remainingPackageBytes)
    {
        if (size > maximumFileBytes || size > remainingPackageBytes || size > int.MaxValue)
        {
            throw new SourceHashException("SOURCE_HASH_LIMIT_EXCEEDED");
        }
    }

    private static string[] RelativeComponents(string path, string repositoryRoot)
    {
        var fullRoot = Path.GetFullPath(repositoryRoot);
        var fullPath = Path.GetFullPath(path);
        var relative = Path.GetRelativePath(fullRoot, fullPath).Replace('\\', '/');
        if (relative is "" or "." or ".." ||
            relative.StartsWith("../", StringComparison.Ordinal) ||
            Path.IsPathRooted(relative))
        {
            throw new SourceHashException("SOURCE_HASH_PATH_INVALID");
        }

        var components = relative.Split('/');
        if (components.Any(component => component is "" or "." or ".."))
        {
            throw new SourceHashException("SOURCE_HASH_PATH_INVALID");
        }
        return components;
    }

    private static byte[] ReadWindowsBytes(nint handle, ulong expectedSize)
    {
        var content = new byte[checked((int)expectedSize)];
        var buffer = new byte[ReadChunkBytes];
        var offset = 0;
        while (offset < content.Length)
        {
            var requested = Math.Min(buffer.Length, content.Length - offset);
            if (!ReadFile(handle, buffer, checked((uint)requested), out var count, nint.Zero))
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
            }
            if (count == 0)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
            }
            Buffer.BlockCopy(buffer, 0, content, offset, checked((int)count));
            offset += checked((int)count);
        }

        if (!ReadFile(handle, buffer, 1, out var probeCount, nint.Zero))
        {
            throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
        }
        if (probeCount != 0)
        {
            throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
        }
        return content;
    }

    private static byte[] ReadPosixBytes(int descriptor, ulong expectedSize)
    {
        var content = new byte[checked((int)expectedSize)];
        var buffer = new byte[ReadChunkBytes];
        var offset = 0;
        while (offset < content.Length)
        {
            var requested = Math.Min(buffer.Length, content.Length - offset);
            var count = Read(descriptor, buffer, checked((nuint)requested));
            if (count < 0)
            {
                if (Marshal.GetLastPInvokeError() == PosixInterrupted)
                {
                    continue;
                }
                throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
            }
            if (count == 0)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
            }
            Buffer.BlockCopy(buffer, 0, content, offset, checked((int)count));
            offset += checked((int)count);
        }

        while (true)
        {
            var count = Read(descriptor, buffer, 1);
            if (count < 0 && Marshal.GetLastPInvokeError() == PosixInterrupted)
            {
                continue;
            }
            if (count < 0)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
            }
            if (count != 0)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
            }
            return content;
        }
    }

    private static T WithSecureWindowsObject<T>(
        string path,
        string repositoryRoot,
        bool expectDirectory,
        Func<nint, SecureObjectState, T> body,
        nint retainedRootHandle = default)
    {
        var components = RelativeComponents(path, repositoryRoot);
        var ownedHandles = new List<nint>();
        var directoryHandles = new List<nint>();
        try
        {
            var rootHandle = retainedRootHandle;
            if (rootHandle == nint.Zero)
            {
                rootHandle = OpenWindowsObject(Path.GetFullPath(repositoryRoot), expectDirectory: true);
                ownedHandles.Add(rootHandle);
            }
            directoryHandles.Add(rootHandle);
            var directoryStates = new List<SecureObjectState> { WindowsState(rootHandle, expectDirectory: true) };
            var current = Path.GetFullPath(repositoryRoot);
            foreach (var component in components[..^1])
            {
                current = Path.Combine(current, component);
                var handle = OpenWindowsObject(current, expectDirectory: true);
                ownedHandles.Add(handle);
                directoryHandles.Add(handle);
                directoryStates.Add(WindowsState(handle, expectDirectory: true));
            }

            current = Path.Combine(current, components[^1]);
            var finalHandle = OpenWindowsObject(current, expectDirectory);
            ownedHandles.Add(finalHandle);
            var before = WindowsState(finalHandle, expectDirectory);
            var result = body(finalHandle, before);
            if (WindowsState(finalHandle, expectDirectory) != before)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
            }
            for (var index = 0; index < directoryHandles.Count; index++)
            {
                if (WindowsState(directoryHandles[index], expectDirectory: true) != directoryStates[index])
                {
                    throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
                }
            }
            return result;
        }
        finally
        {
            for (var index = ownedHandles.Count - 1; index >= 0; index--)
            {
                _ = CloseHandle(ownedHandles[index]);
            }
        }
    }

    private static nint OpenWindowsObject(string path, bool expectDirectory)
    {
        var flags = FileFlagOpenReparsePoint |
                    (expectDirectory ? FileFlagBackupSemantics : FileFlagSequentialScan);
        var handle = CreateFile(
            path,
            GenericRead,
            FileShareRead,
            nint.Zero,
            OpenExisting,
            flags,
            nint.Zero);
        if (handle == InvalidHandleValue)
        {
            throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
        }
        try
        {
            _ = WindowsState(handle, expectDirectory);
            return handle;
        }
        catch
        {
            _ = CloseHandle(handle);
            throw;
        }
    }

    private static SecureObjectState WindowsState(nint handle, bool expectDirectory)
    {
        if (!GetFileInformationByHandle(handle, out var information) ||
            !GetFileInformationByHandleEx(
                handle,
                FileBasicInfoClass,
                out var basicInformation,
                checked((uint)Marshal.SizeOf<FileBasicInformation>())))
        {
            throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
        }

        var attributes = information.FileAttributes;
        var isDirectory = (attributes & FileAttributeDirectory) != 0;
        if ((attributes & FileAttributeReparsePoint) != 0 ||
            isDirectory != expectDirectory ||
            (!expectDirectory && information.NumberOfLinks != 1))
        {
            throw new SourceHashException("SOURCE_HASH_LINK_REJECTED");
        }

        return new SecureObjectState(
            information.VolumeSerialNumber,
            information.FileIndexHigh,
            information.FileIndexLow,
            information.NumberOfLinks,
            ((ulong)information.FileSizeHigh << 32) | information.FileSizeLow,
            unchecked((ulong)basicInformation.LastWriteTime) >> 32,
            unchecked((ulong)basicInformation.LastWriteTime) & uint.MaxValue,
            unchecked((ulong)basicInformation.ChangeTime) >> 32,
            unchecked((ulong)basicInformation.ChangeTime) & uint.MaxValue,
            attributes);
    }

    private static T WithSecurePosixObject<T>(
        string path,
        string repositoryRoot,
        bool expectDirectory,
        Func<int, SecureObjectState, T> body,
        int retainedRootDescriptor = -1)
    {
        var components = RelativeComponents(path, repositoryRoot);
        var ownedDescriptors = new List<int>();
        var directoryDescriptors = new List<int>();
        try
        {
            var rootDescriptor = retainedRootDescriptor;
            if (rootDescriptor < 0)
            {
                rootDescriptor = OpenPosixObject(null, Path.GetFullPath(repositoryRoot), expectDirectory: true);
                ownedDescriptors.Add(rootDescriptor);
            }
            directoryDescriptors.Add(rootDescriptor);
            var directoryStates = new List<SecureObjectState> { PosixState(rootDescriptor, expectDirectory: true) };
            var parent = rootDescriptor;
            foreach (var component in components[..^1])
            {
                var descriptor = OpenPosixObject(parent, component, expectDirectory: true);
                ownedDescriptors.Add(descriptor);
                directoryDescriptors.Add(descriptor);
                directoryStates.Add(PosixState(descriptor, expectDirectory: true));
                parent = descriptor;
            }

            var finalDescriptor = OpenPosixObject(parent, components[^1], expectDirectory);
            ownedDescriptors.Add(finalDescriptor);
            var before = PosixState(finalDescriptor, expectDirectory);
            var result = body(finalDescriptor, before);
            if (PosixState(finalDescriptor, expectDirectory) != before)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
            }
            for (var index = 0; index < directoryDescriptors.Count; index++)
            {
                if (PosixState(directoryDescriptors[index], expectDirectory: true) != directoryStates[index])
                {
                    throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
                }
            }

            var reopened = OpenPosixObject(parent, components[^1], expectDirectory);
            try
            {
                if (PosixState(reopened, expectDirectory) != before)
                {
                    throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
                }
            }
            finally
            {
                _ = Close(reopened);
            }
            return result;
        }
        finally
        {
            for (var index = ownedDescriptors.Count - 1; index >= 0; index--)
            {
                _ = Close(ownedDescriptors[index]);
            }
        }
    }

    private static T WithSecurePosixObjectAtParent<T>(
        int parentDescriptor,
        string basename,
        bool expectDirectory,
        Func<int, SecureObjectState, T> body)
    {
        var descriptor = OpenPosixObject(parentDescriptor, basename, expectDirectory);
        try
        {
            var before = PosixState(descriptor, expectDirectory);
            var result = body(descriptor, before);
            if (PosixState(descriptor, expectDirectory) != before)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
            }
            var reopened = OpenPosixObject(parentDescriptor, basename, expectDirectory);
            try
            {
                if (PosixState(reopened, expectDirectory) != before)
                {
                    throw new SourceHashException("SOURCE_HASH_FILE_UNSTABLE");
                }
            }
            finally
            {
                _ = Close(reopened);
            }
            return result;
        }
        finally
        {
            _ = Close(descriptor);
        }
    }

    private static int OpenPosixObject(int? parent, string path, bool expectDirectory)
    {
        var flags = PosixReadOnly | PosixNoFollow | PosixNonBlocking | PosixCloseOnExec |
                    (expectDirectory ? PosixDirectory : 0);
        int descriptor;
        do
        {
            descriptor = parent is null
                ? Open(path, flags)
                : OpenAt(parent.Value, path, flags);
        }
        while (descriptor < 0 && Marshal.GetLastPInvokeError() == PosixInterrupted);

        if (descriptor < 0)
        {
            if (Marshal.GetLastPInvokeError() == PosixSymbolicLinkLoop)
            {
                throw new SourceHashException("SOURCE_HASH_LINK_REJECTED");
            }
            throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
        }
        try
        {
            _ = PosixState(descriptor, expectDirectory);
            return descriptor;
        }
        catch
        {
            _ = Close(descriptor);
            throw;
        }
    }

    private static int OpenPosixBelowRoot(
        int rootDescriptor,
        IReadOnlyList<string> components,
        bool expectDirectory)
    {
        if (components.Count == 0)
        {
            throw new SourceHashException("SOURCE_HASH_PATH_INVALID");
        }

        var parent = rootDescriptor;
        var ownedParent = -1;
        try
        {
            for (var index = 0; index < components.Count; index++)
            {
                var descriptor = OpenPosixObject(
                    parent,
                    components[index],
                    expectDirectory: index + 1 < components.Count || expectDirectory);
                if (ownedParent >= 0)
                {
                    _ = Close(ownedParent);
                }
                ownedParent = descriptor;
                parent = descriptor;
            }
            var result = ownedParent;
            ownedParent = -1;
            return result;
        }
        finally
        {
            if (ownedParent >= 0)
            {
                _ = Close(ownedParent);
            }
        }
    }

    private static int Duplicate(int descriptor)
    {
        int duplicate;
        do
        {
            duplicate = Dup(descriptor);
        }
        while (duplicate < 0 && Marshal.GetLastPInvokeError() == PosixInterrupted);
        return duplicate;
    }

    private static string ReadDirectoryEntryName(nint entry)
    {
        var nameOffset = OperatingSystem.IsMacOS() ? 21 : 19;
        var recordLength = checked((int)(ushort)Marshal.ReadInt16(entry, 16));
        if (recordLength <= nameOffset || recordLength > 4096)
        {
            throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
        }
        var maximumNameBytes = recordLength - nameOffset;
        if (OperatingSystem.IsMacOS())
        {
            maximumNameBytes = checked((int)(ushort)Marshal.ReadInt16(entry, 18));
            if (maximumNameBytes <= 0 || maximumNameBytes > recordLength - nameOffset)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
            }
        }

        var bytes = new byte[maximumNameBytes];
        Marshal.Copy(entry + nameOffset, bytes, 0, bytes.Length);
        var terminator = Array.IndexOf(bytes, (byte)0);
        var length = terminator >= 0 ? terminator : bytes.Length;
        string name;
        try
        {
            name = StrictUtf8.GetString(bytes, 0, length);
        }
        catch (DecoderFallbackException)
        {
            throw new SourceHashException("SOURCE_HASH_PATH_INVALID");
        }
        if (name.Length == 0 || name.Contains('/') || name.Contains('\0'))
        {
            throw new SourceHashException("SOURCE_HASH_PATH_INVALID");
        }
        return name;
    }

    private static SecureDirectoryEntryKind PosixEntryKind(int directoryDescriptor, string name)
    {
        uint mode;
        if (OperatingSystem.IsMacOS())
        {
            if (FStatAtMac(directoryDescriptor, name, out var information, PosixAtSymlinkNoFollow) != 0)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
            }
            mode = information.Mode;
        }
        else
        {
            if (StatxLinux(
                    directoryDescriptor,
                    name,
                    PosixAtSymlinkNoFollow,
                    StatxRequiredMask,
                    out var information) != 0 ||
                (information.Mask & StatxRequiredMask) != StatxRequiredMask)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
            }
            mode = information.Mode;
        }

        return (mode & PosixFileTypeMask) switch
        {
            PosixDirectoryType => SecureDirectoryEntryKind.Directory,
            PosixRegularType => SecureDirectoryEntryKind.Regular,
            PosixSymbolicLinkType => SecureDirectoryEntryKind.Linked,
            _ => SecureDirectoryEntryKind.Other,
        };
    }

    private static SecureObjectState PosixState(int descriptor, bool expectDirectory)
    {
        ulong device;
        ulong inode;
        ulong linkCount;
        uint mode;
        ulong size;
        long modifiedSeconds;
        long modifiedNanoseconds;
        long changedSeconds;
        long changedNanoseconds;

        if (OperatingSystem.IsMacOS())
        {
            if (FStatMac(descriptor, out var information) != 0)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
            }
            device = unchecked((uint)information.Device);
            inode = information.Inode;
            linkCount = information.LinkCount;
            mode = information.Mode;
            if (information.Size < 0)
            {
                throw new SourceHashException("SOURCE_HASH_LINK_REJECTED");
            }
            size = checked((ulong)information.Size);
            modifiedSeconds = information.Modified.Seconds;
            modifiedNanoseconds = information.Modified.Nanoseconds;
            changedSeconds = information.Changed.Seconds;
            changedNanoseconds = information.Changed.Nanoseconds;
        }
        else
        {
            if (StatxLinux(
                    descriptor,
                    string.Empty,
                    PosixAtEmptyPath | PosixAtSymlinkNoFollow,
                    StatxRequiredMask,
                    out var information) != 0 ||
                (information.Mask & StatxRequiredMask) != StatxRequiredMask)
            {
                throw new SourceHashException("SOURCE_HASH_FILE_UNAVAILABLE");
            }
            device = ((ulong)information.DeviceMajor << 32) | information.DeviceMinor;
            inode = information.Inode;
            linkCount = information.LinkCount;
            mode = information.Mode;
            size = information.Size;
            modifiedSeconds = information.Modified.Seconds;
            modifiedNanoseconds = information.Modified.Nanoseconds;
            changedSeconds = information.Changed.Seconds;
            changedNanoseconds = information.Changed.Nanoseconds;
        }

        var kind = mode & PosixFileTypeMask;
        var expectedKind = expectDirectory ? PosixDirectoryType : PosixRegularType;
        if (kind != expectedKind || (!expectDirectory && linkCount != 1))
        {
            throw new SourceHashException("SOURCE_HASH_LINK_REJECTED");
        }
        return new SecureObjectState(
            device,
            0,
            inode,
            linkCount,
            size,
            unchecked((ulong)modifiedSeconds),
            unchecked((ulong)modifiedNanoseconds),
            unchecked((ulong)changedSeconds),
            unchecked((ulong)changedNanoseconds),
            mode);
    }

    private const uint GenericRead = 0x80000000;
    private const uint FileShareRead = 0x00000001;
    private const uint OpenExisting = 3;
    private const uint FileAttributeDirectory = 0x00000010;
    private const uint FileAttributeReparsePoint = 0x00000400;
    private const uint FileFlagOpenReparsePoint = 0x00200000;
    private const uint FileFlagBackupSemantics = 0x02000000;
    private const uint FileFlagSequentialScan = 0x08000000;
    private const int FileBasicInfoClass = 0;
    private const int FileIdBothDirectoryInfoClass = 10;
    private const int FileIdBothDirectoryRestartInfoClass = 11;
    private const int FileIdBothDirectoryNameOffset = 104;
    private const int WindowsNoMoreFiles = 18;
    private static readonly nint InvalidHandleValue = new(-1);

    private const int PosixReadOnly = 0;
    private const int PosixInterrupted = 4;
    private static int PosixSymbolicLinkLoop => OperatingSystem.IsMacOS() ? 62 : 40;
    private const uint PosixFileTypeMask = 0xF000;
    private const uint PosixRegularType = 0x8000;
    private const uint PosixDirectoryType = 0x4000;
    private const uint PosixSymbolicLinkType = 0xA000;
    private static int PosixNoFollow => OperatingSystem.IsMacOS() ? 0x00000100 : 0x00020000;
    private static int PosixNonBlocking => OperatingSystem.IsMacOS() ? 0x00000004 : 0x00000800;
    private static int PosixCloseOnExec => OperatingSystem.IsMacOS() ? 0x01000000 : 0x00080000;
    private static int PosixDirectory => OperatingSystem.IsMacOS() ? 0x00100000 : 0x00010000;
    private static int PosixAtSymlinkNoFollow => OperatingSystem.IsMacOS() ? 0x0020 : 0x0100;
    private const int PosixAtEmptyPath = 0x1000;
    private const uint StatxRequiredMask = 0x03C7;

    private sealed class WindowsHandleOwner(nint handle) : IDisposable
    {
        internal nint Handle { get; } = handle;

        public void Dispose() => _ = CloseHandle(Handle);
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct WindowsFileTime
    {
        internal uint Low;
        internal uint High;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct ByHandleFileInformation
    {
        internal uint FileAttributes;
        internal WindowsFileTime CreationTime;
        internal WindowsFileTime LastAccessTime;
        internal WindowsFileTime LastWriteTime;
        internal uint VolumeSerialNumber;
        internal uint FileSizeHigh;
        internal uint FileSizeLow;
        internal uint NumberOfLinks;
        internal uint FileIndexHigh;
        internal uint FileIndexLow;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct FileBasicInformation
    {
        internal long CreationTime;
        internal long LastAccessTime;
        internal long LastWriteTime;
        internal long ChangeTime;
        internal uint FileAttributes;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct PosixTimeSpec
    {
        internal long Seconds;
        internal long Nanoseconds;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct LinuxStatxTimestamp
    {
        internal long Seconds;
        internal uint Nanoseconds;
        internal int Reserved;
    }

    [StructLayout(LayoutKind.Sequential, Size = 256)]
    private struct LinuxStatx
    {
        internal uint Mask;
        internal uint BlockSize;
        internal ulong Attributes;
        internal uint LinkCount;
        internal uint UserId;
        internal uint GroupId;
        internal ushort Mode;
        internal ushort Spare0;
        internal ulong Inode;
        internal ulong Size;
        internal ulong Blocks;
        internal ulong AttributesMask;
        internal LinuxStatxTimestamp Accessed;
        internal LinuxStatxTimestamp Created;
        internal LinuxStatxTimestamp Changed;
        internal LinuxStatxTimestamp Modified;
        internal uint SpecialDeviceMajor;
        internal uint SpecialDeviceMinor;
        internal uint DeviceMajor;
        internal uint DeviceMinor;
        internal ulong MountId;
        internal uint DirectIoMemoryAlignment;
        internal uint DirectIoOffsetAlignment;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct MacStat
    {
        internal int Device;
        internal ushort Mode;
        internal ushort LinkCount;
        internal ulong Inode;
        internal uint UserId;
        internal uint GroupId;
        internal int SpecialDevice;
        internal PosixTimeSpec Accessed;
        internal PosixTimeSpec Modified;
        internal PosixTimeSpec Changed;
        internal PosixTimeSpec Created;
        internal long Size;
        internal long Blocks;
        internal int BlockSize;
        internal uint Flags;
        internal uint Generation;
        internal int Spare;
        internal long Reserved0;
        internal long Reserved1;
    }

    [DllImport("kernel32.dll", EntryPoint = "CreateFileW", SetLastError = true, CharSet = CharSet.Unicode)]
    private static extern nint CreateFile(
        string fileName,
        uint desiredAccess,
        uint shareMode,
        nint securityAttributes,
        uint creationDisposition,
        uint flagsAndAttributes,
        nint templateFile);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ReadFile(
        nint file,
        byte[] buffer,
        uint bytesToRead,
        out uint bytesRead,
        nint overlapped);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetFileInformationByHandle(
        nint file,
        out ByHandleFileInformation information);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetFileInformationByHandleEx(
        nint file,
        int informationClass,
        out FileBasicInformation information,
        uint bufferSize);

    [DllImport("kernel32.dll", EntryPoint = "GetFileInformationByHandleEx", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetFileInformationByHandleExBuffer(
        nint file,
        int informationClass,
        [Out] byte[] information,
        uint bufferSize);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseHandle(nint handle);

    [DllImport("libc", EntryPoint = "open", SetLastError = true)]
    private static extern int Open(string path, int flags);

    [DllImport("libc", EntryPoint = "openat", SetLastError = true)]
    private static extern int OpenAt(int directory, string path, int flags);

    [DllImport("libc", EntryPoint = "read", SetLastError = true)]
    private static extern nint Read(int descriptor, byte[] buffer, nuint count);

    [DllImport("libc", EntryPoint = "close", SetLastError = true)]
    private static extern int Close(int descriptor);

    [DllImport("libc", EntryPoint = "dup", SetLastError = true)]
    private static extern int Dup(int descriptor);

    [DllImport("libc", EntryPoint = "fdopendir", SetLastError = true)]
    private static extern nint FileDescriptorOpenDirectory(int descriptor);

    [DllImport("libc", EntryPoint = "readdir", SetLastError = true)]
    private static extern nint ReadDirectory(nint directory);

    [DllImport("libc", EntryPoint = "closedir", SetLastError = true)]
    private static extern int CloseDirectory(nint directory);

    [DllImport("libc", EntryPoint = "fstat", SetLastError = true)]
    private static extern int FStatMac(int descriptor, out MacStat information);

    [DllImport("libc", EntryPoint = "statx", SetLastError = true)]
    private static extern int StatxLinux(
        int directory,
        string path,
        int flags,
        uint mask,
        out LinuxStatx information);

    [DllImport("libc", EntryPoint = "fstatat", SetLastError = true)]
    private static extern int FStatAtMac(
        int directory,
        string path,
        out MacStat information,
        int flags);
}
