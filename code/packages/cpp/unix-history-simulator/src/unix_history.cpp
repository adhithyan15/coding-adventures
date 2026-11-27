#include "unix_history.hpp"
#include <algorithm>
#include <limits>

namespace unix_history {

FileSystem::FileSystem(std::map<Inode, Node> nodes) : nodes_(std::move(nodes)) {}

FileSystem FileSystem::demo() {
    return FileSystem({
        {1, {1, {{"hello", 2}, {"empty", 3}, {"etc", 4}}, std::nullopt}},
        {2, {1, {}, "Hello, Unix!\n"}},
        {3, {1, {}, ""}},
        {4, {1, {{"notes", 5}}, std::nullopt}},
        {5, {4, {}, "A directory maps a name to an inode.\nAn open descriptor tracks a byte offset.\n"}},
    });
}

Lookup FileSystem::resolve(std::string_view path) const {
    if (path.empty() || path.find('\0') != path.npos) return {0, nullptr, "EINVAL"};
    Inode current = 1;
    auto cursor = path;
    while (!cursor.empty()) {
        const auto slash = cursor.find('/');
        auto component = cursor.substr(0, slash);
        cursor = slash == cursor.npos ? std::string_view{} : cursor.substr(slash + 1);
        if (component.empty()) continue;
        auto node = nodes_.find(current);
        if (node == nodes_.end()) return {0, nullptr, "ENOENT"};
        if (node->second.data) return {0, nullptr, "ENOTDIR"};
        if (component == ".") continue;
        if (component == "..") {
            current = node->second.parent;
            continue;
        }
        const auto& entries = node->second.entries;
        const auto entry = std::find_if(entries.begin(), entries.end(), [&](const auto& candidate) {
            return candidate.first == component;
        });
        if (entry == entries.end()) return {0, nullptr, "ENOENT"};
        current = entry->second;
    }
    auto node = nodes_.find(current);
    if (node == nodes_.end()) return {0, nullptr, "ENOENT"};
    if (path.back() == '/' && node->second.data) return {0, nullptr, "ENOTDIR"};
    return {current, &node->second, {}};
}

std::pair<std::string_view, std::string_view> FileSystem::read(Inode inode, std::size_t offset, std::size_t count) const {
    const auto node = nodes_.find(inode);
    if (node == nodes_.end()) return {{}, "ENOENT"};
    if (!node->second.data) return {{}, "EISDIR"};
    const std::string_view bytes = *node->second.data;
    return {bytes.substr(std::min(offset, bytes.size()), count), {}};
}

DescriptorTable::DescriptorTable(std::size_t capacity) : slots_(capacity) {}

std::optional<std::size_t> DescriptorTable::allocate(Inode inode) {
    for (std::size_t fd = 0; fd < slots_.size(); ++fd) {
        if (!slots_[fd]) {
            slots_[fd] = Descriptor{inode, 0};
            return fd;
        }
    }
    return std::nullopt;
}

std::optional<Descriptor> DescriptorTable::get(std::size_t fd) const {
    return fd < slots_.size() ? slots_[fd] : std::nullopt;
}

bool DescriptorTable::advance(std::size_t fd, std::size_t count) {
    const auto slot = get(fd);
    if (!slot || count > std::numeric_limits<std::size_t>::max() - slot->offset) return false;
    slots_[fd]->offset += count;
    return true;
}

bool DescriptorTable::close(std::size_t fd) {
    if (!get(fd)) return false;
    slots_[fd] = std::nullopt;
    return true;
}

Kernel::Kernel(FileSystem files, DescriptorTable descriptors)
    : files_(std::move(files)), descriptors_(std::move(descriptors)) {}

Event Kernel::open(std::string path) {
    Event event("open");
    const auto found = files_.resolve(path);
    event.path = std::move(path);
    if (!found.node) event.error = found.error;
    else if (!found.node->data) event.error = "EISDIR";
    else if (const auto fd = descriptors_.allocate(found.inode)) {
        event.fd = fd;
        event.inode = found.inode;
        event.offset = 0;
    } else event.error = "EMFILE";
    return event;
}

std::pair<Event, std::string_view> Kernel::read(std::size_t fd, std::size_t count) {
    Event event("read");
    event.fd = fd;
    const auto slot = descriptors_.get(fd);
    if (!slot) {
        event.error = "EBADF";
        return {std::move(event), {}};
    }
    const auto [bytes, error] = files_.read(slot->inode, slot->offset, count);
    if (!error.empty()) {
        event.error = error;
        return {std::move(event), {}};
    }
    descriptors_.advance(fd, bytes.size());
    event.inode = slot->inode;
    event.offset = slot->offset;
    event.bytes = bytes.size();
    event.next_offset = slot->offset + bytes.size();
    return {std::move(event), bytes};
}

Event Kernel::close(std::size_t fd) {
    Event event("close");
    event.fd = fd;
    if (!descriptors_.close(fd)) event.error = "EBADF";
    return event;
}

Event Kernel::list(std::string path) const {
    Event event("list");
    const auto found = files_.resolve(path);
    event.path = std::move(path);
    if (!found.node) event.error = found.error;
    else if (found.node->data) event.error = "ENOTDIR";
    else {
        event.inode = found.inode;
        event.names.emplace();
        for (const auto& [name, inode] : found.node->entries) {
            event.names->push_back(name);
        }
    }
    return event;
}

Result run(std::string_view command) {
    Kernel kernel(FileSystem::demo(), DescriptorTable(4));
    Result result;
    std::vector<std::string> words;
    constexpr std::string_view spaces = " \t\r\n\f\v";
    for (auto begin = command.find_first_not_of(spaces); begin != command.npos;) {
        auto end = command.find_first_of(spaces, begin);
        words.emplace_back(command.substr(begin, end - begin));
        begin = command.find_first_not_of(spaces, end);
    }
    if (words.size() == 2 && words[0] == "cat") {
        auto opened = kernel.open(words[1]);
        const auto fd = opened.fd;
        result.events.push_back(std::move(opened));
        if (fd) {
            for (;;) {
                auto [event, bytes] = kernel.read(*fd, 8);
                result.events.push_back(std::move(event));
                if (bytes.empty()) break;
                result.output.append(bytes);
            }
            result.events.push_back(kernel.close(*fd));
        }
    } else if (!words.empty() && words[0] == "ls" && words.size() <= 2) {
        auto listed = kernel.list(words.size() == 2 ? words[1] : "/");
        if (listed.names) for (const auto& name : *listed.names) result.output.append(name).push_back('\n');
        result.events.push_back(std::move(listed));
    } else if (!words.empty()) {
        result.output = "usage: cat PATH or ls [PATH]\n";
    }
    for (const auto& event : result.events) {
        if (!event.error.empty()) result.output.append("error: ").append(event.error).push_back('\n');
    }
    return result;
}

} // namespace unix_history
