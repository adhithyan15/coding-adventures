#include "unix_history.hpp"
#include <cassert>
#include <limits>

int main() {
    unix_history::DescriptorTable table(2);
    assert(table.allocate(2) == 0);
    assert(table.allocate(2) == 1);
    assert(table.allocate(2) == std::nullopt);
    assert(table.advance(0, 4));
    assert(!table.advance(0, std::numeric_limits<std::size_t>::max()));
    assert(table.get(1)->offset == 0);
    assert(!table.close(4));
    assert(table.close(0));
    assert(table.allocate(3) == 0);
    assert(table.get(0)->offset == 0);
    const auto files = unix_history::FileSystem::demo();
    assert(files.resolve("//etc/../etc/./notes").inode == 5);
    assert(files.resolve("/hello/..").error == "ENOTDIR");
    assert(files.resolve(std::string_view{"a\0b", 3}).error == "EINVAL");

    unix_history::Kernel kernel(unix_history::FileSystem::demo(), unix_history::DescriptorTable(2));
    const auto opened = kernel.open("/hello");
    assert(opened.fd == 0 && opened.inode == 2);
    auto [first_read, first_bytes] = kernel.read(*opened.fd, 5);
    assert(first_read.offset == 0 && first_read.next_offset == 5 && first_bytes == "Hello");
    assert(kernel.close(*opened.fd).error.empty());
    assert(kernel.read(*opened.fd, 1).first.error == "EBADF");
}
