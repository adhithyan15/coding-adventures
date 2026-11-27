package codingadventures.unixhistory;

import java.nio.charset.StandardCharsets;

public final class ModelTest {
    public static void main(String[] args) {
        var table = new UnixHistory.DescriptorTable(2);
        require(table.allocate(2) == 0, "lowest slot");
        require(table.allocate(2) == 1, "second slot");
        require(table.allocate(2) == -1, "full table");
        require(table.advance(0, 4) && table.get(1).offset() == 0, "independent positions");
        require(!table.close(-1) && table.close(0), "close");
        require(table.allocate(3) == 0 && table.get(0).offset() == 0, "reused slot");
        var fs = UnixHistory.FileSystem.demo();
        require("ENOENT".equals(fs.read(99, 0, 1).error()), "missing inode read");
        require("EISDIR".equals(fs.read(1, 0, 1).error()), "directory read");
        require("EINVAL".equals(fs.read(2, -1, 1).error()), "negative byte offset");
        require(fs.read(2, 100, 0).data().length == 0, "read beyond eof");
        require(fs.resolve("//etc/../etc/./notes").inode() == 5, "directory walk");
        require("ENOTDIR".equals(fs.resolve("/hello/..").error()), "intermediate file");
        require("EINVAL".equals(fs.resolve("he\0llo").error()), "NUL path");

        var kernel = new UnixHistory.Kernel(fs, new UnixHistory.DescriptorTable(2));
        var opened = kernel.open("/hello");
        require(opened.get("fd").equals(0) && opened.get("inode").equals(2), "kernel open");
        var firstRead = kernel.read(0, 5);
        require(firstRead.event().get("offset").equals(0)
            && new String(firstRead.data(), StandardCharsets.UTF_8).equals("Hello"), "kernel read");
        require(kernel.close(0).get("call").equals("close"), "kernel close");
        require("EBADF".equals(kernel.read(0, 1).event().get("error")), "closed descriptor");

        var invalidInodeTable = new UnixHistory.DescriptorTable(1);
        require(invalidInodeTable.allocate(99) == 0, "descriptor can expose invalid image inode");
        var invalidInodeRead = new UnixHistory.Kernel(fs, invalidInodeTable).read(0, 1);
        require("ENOENT".equals(invalidInodeRead.event().get("error"))
            && invalidInodeRead.data().length == 0, "kernel reports missing descriptor inode");
    }

    private static void require(boolean condition, String description) {
        if (!condition) throw new AssertionError(description);
    }
}
