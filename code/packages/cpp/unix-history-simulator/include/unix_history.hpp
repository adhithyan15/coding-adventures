#ifndef CODING_ADVENTURES_UNIX_HISTORY_HPP
#define CODING_ADVENTURES_UNIX_HISTORY_HPP

#include <cstddef>
#include <map>
#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace unix_history {

using Inode = std::size_t;
using Entry = std::pair<std::string, Inode>;

struct Node {
    Inode parent;
    std::vector<Entry> entries;
    std::optional<std::string> data;
};

struct Lookup {
    Inode inode;
    const Node* node;
    std::string_view error;
};

class FileSystem {
public:
    explicit FileSystem(std::map<Inode, Node> nodes);
    static FileSystem demo();
    Lookup resolve(std::string_view path) const;
    std::pair<std::string_view, std::string_view> read(Inode inode, std::size_t offset, std::size_t count) const;

private:
    std::map<Inode, Node> nodes_;
};

struct Descriptor {
    Inode inode;
    std::size_t offset;
};

class DescriptorTable {
public:
    explicit DescriptorTable(std::size_t capacity);
    std::optional<std::size_t> allocate(Inode inode);
    std::optional<Descriptor> get(std::size_t fd) const;
    bool advance(std::size_t fd, std::size_t count);
    bool close(std::size_t fd);

private:
    std::vector<std::optional<Descriptor>> slots_;
};

struct Event {
    std::string_view call;
    std::optional<std::string> path;
    std::optional<std::size_t> fd, inode, offset, bytes, next_offset;
    std::optional<std::vector<std::string>> names;
    std::string_view error;

    explicit Event(std::string_view syscall) : call(syscall) {}
};

class Kernel {
public:
    Kernel(FileSystem files, DescriptorTable descriptors);
    Event open(std::string path);
    std::pair<Event, std::string_view> read(std::size_t fd, std::size_t count);
    Event close(std::size_t fd);
    Event list(std::string path) const;

private:
    FileSystem files_;
    DescriptorTable descriptors_;
};

struct Result {
    std::string output;
    std::vector<Event> events;
};

Result run(std::string_view command);

} // namespace unix_history
#endif
