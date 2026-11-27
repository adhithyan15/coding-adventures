package codingadventures.unixhistory;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Map;

public final class UnixHistory {
    private UnixHistory() {}

    public record Entry(String name, int inode) {}
    public sealed interface Node permits Directory, File {}
    public record Directory(int parent, List<Entry> entries) implements Node {
        public Directory { entries = List.copyOf(entries); }
    }
    public record File(String text) implements Node {}
    public record Lookup(int inode, Node node, String error) {}

    public static final class FileSystem {
        private final Map<Integer, Node> nodes;

        public FileSystem(Map<Integer, Node> nodes) {
            this.nodes = Map.copyOf(nodes);
        }

        public static FileSystem demo() {
            return new FileSystem(Map.of(
                1, new Directory(1, List.of(new Entry("hello", 2), new Entry("empty", 3), new Entry("etc", 4))),
                2, new File("Hello, Unix!\n"),
                3, new File(""),
                4, new Directory(1, List.of(new Entry("notes", 5))),
                5, new File("A directory maps a name to an inode.\nAn open descriptor tracks a byte offset.\n")
            ));
        }

        public Lookup resolve(String path) {
            if (path.isEmpty() || path.indexOf(0) != -1) return missing("EINVAL");
            int inode = 1;
            for (String name : path.split("/")) {
                if (name.isEmpty()) continue;
                Node current = nodes.get(inode);
                if (current == null) return missing("ENOENT");
                if (!(current instanceof Directory directory)) return missing("ENOTDIR");
                switch (name) {
                    case "." -> { }
                    case ".." -> inode = directory.parent();
                    default -> {
                        var entry = directory.entries().stream().filter(item -> item.name().equals(name)).findFirst();
                        if (entry.isEmpty()) return missing("ENOENT");
                        inode = entry.get().inode();
                    }
                }
            }
            Node node = nodes.get(inode);
            if (node == null) return missing("ENOENT");
            if (path.endsWith("/") && node instanceof File) return missing("ENOTDIR");
            return new Lookup(inode, node, null);
        }

        public record FileRead(byte[] data, String error) {}

        public FileRead read(int inode, int offset, int count) {
            var node = nodes.get(inode);
            if (offset < 0 || count < 0) return new FileRead(new byte[0], "EINVAL");
            if (node == null) return new FileRead(new byte[0], "ENOENT");
            if (!(node instanceof File file)) return new FileRead(new byte[0], "EISDIR");
            byte[] data = file.text().getBytes(StandardCharsets.UTF_8);
            int start = Math.min(offset, data.length);
            return new FileRead(Arrays.copyOfRange(data, start, start + Math.min(count, data.length - start)), null);
        }

        private static Lookup missing(String error) { return new Lookup(0, null, error); }
    }

    public record Descriptor(int inode, int offset) {}

    public static final class DescriptorTable {
        private final Descriptor[] slots;

        public DescriptorTable(int capacity) { slots = new Descriptor[capacity]; }

        public Descriptor get(int fd) {
            return fd >= 0 && fd < slots.length ? slots[fd] : null;
        }

        public int allocate(int inode) {
            for (int fd = 0; fd < slots.length; fd++) {
                if (slots[fd] == null) {
                    slots[fd] = new Descriptor(inode, 0);
                    return fd;
                }
            }
            return -1;
        }

        public boolean advance(int fd, int count) {
            Descriptor slot = get(fd);
            if (slot == null || count < 0 || count > Integer.MAX_VALUE - slot.offset()) return false;
            slots[fd] = new Descriptor(slot.inode(), slot.offset() + count);
            return true;
        }

        public boolean close(int fd) {
            if (get(fd) == null) return false;
            slots[fd] = null;
            return true;
        }
    }

    public record Read(Map<String, Object> event, byte[] data) {}

    public record Kernel(FileSystem files, DescriptorTable descriptors) {
        public Map<String, Object> open(String path) {
            Lookup found = files.resolve(path);
            if (found.error() != null) return error("open", "path", path, found.error());
            if (found.node() instanceof Directory) return error("open", "path", path, "EISDIR");
            int fd = descriptors.allocate(found.inode());
            if (fd < 0) return error("open", "path", path, "EMFILE");
            return Map.of("call", "open", "path", path, "inode", found.inode(), "fd", fd, "offset", 0);
        }

        public Read read(int fd, int count) {
            Descriptor slot = descriptors.get(fd);
            if (slot == null) return new Read(error("read", "fd", fd, "EBADF"), new byte[0]);
            FileSystem.FileRead fileRead = files.read(slot.inode(), slot.offset(), count);
            if (fileRead.error() != null) return new Read(error("read", "fd", fd, fileRead.error()), new byte[0]);
            byte[] data = fileRead.data();
            descriptors.advance(fd, data.length);
            return new Read(Map.of("call", "read", "fd", fd, "inode", slot.inode(), "offset", slot.offset(),
                "bytes", data.length, "nextOffset", slot.offset() + data.length), data);
        }

        public Map<String, Object> close(int fd) {
            if (!descriptors.close(fd)) return error("close", "fd", fd, "EBADF");
            return Map.of("call", "close", "fd", fd);
        }

        public Map<String, Object> list(String path) {
            Lookup found = files.resolve(path);
            if (found.error() != null) return error("list", "path", path, found.error());
            if (!(found.node() instanceof Directory directory)) return error("list", "path", path, "ENOTDIR");
            return Map.of("call", "list", "path", path, "inode", found.inode(),
                "names", directory.entries().stream().map(Entry::name).toList());
        }

        private static Map<String, Object> error(String call, String argument, Object value, String code) {
            return Map.of("call", call, argument, value, "error", code);
        }
    }

    public record Result(String output, List<Map<String, Object>> events) {}

    public static Result run(String command) {
        var kernel = new Kernel(FileSystem.demo(), new DescriptorTable(4));
        var words = Arrays.stream(command.split("[ \\t\\n\\r\\f\\x0B]+"))
            .filter(word -> !word.isEmpty()).toList();
        var events = new ArrayList<Map<String, Object>>();
        var output = new StringBuilder();
        if (words.size() == 2 && words.getFirst().equals("cat")) {
            var opened = kernel.open(words.get(1));
            events.add(opened);
            if (opened.get("fd") instanceof Integer fd) {
                var file = new java.io.ByteArrayOutputStream();
                while (true) {
                    Read read = kernel.read(fd, 8);
                    events.add(read.event());
                    if (read.data().length == 0) break;
                    file.writeBytes(read.data());
                }
                output.append(file.toString(StandardCharsets.UTF_8));
                events.add(kernel.close(fd));
            }
        } else if (!words.isEmpty() && words.size() <= 2 && words.getFirst().equals("ls")) {
            var listed = kernel.list(words.size() == 2 ? words.get(1) : "/");
            events.add(listed);
            if (listed.get("names") instanceof List<?> names) {
                names.forEach(name -> output.append(name).append('\n'));
            }
        } else if (!words.isEmpty()) {
            output.append("usage: cat PATH or ls [PATH]\n");
        }
        for (var event : events) {
            if (event.get("error") instanceof String error) output.append("error: ").append(error).append('\n');
        }
        return new Result(output.toString(), List.copyOf(events));
    }
}
