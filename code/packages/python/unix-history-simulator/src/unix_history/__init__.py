"""First Edition Unix file-read model with an explicit syscall trace."""

import re
from dataclasses import dataclass
from typing import Any

type Event = dict[str, Any]


@dataclass(frozen=True)
class Inode:
    number: int
    data: bytes | None = None
    entries: tuple[tuple[str, int], ...] = ()


class FileSystem:
    """Directories resolve names. File data is only addressed by inode."""

    def __init__(self) -> None:
        self.inodes = {
            1: Inode(1, entries=(("hello", 2), ("empty", 3), ("etc", 4))),
            2: Inode(2, b"Hello, Unix!\n"),
            3: Inode(3, b""),
            4: Inode(4, entries=(("notes", 5),)),
            5: Inode(
                5,
                b"A directory maps a name to an inode.\n"
                b"An open descriptor tracks a byte offset.\n",
            ),
        }

    def resolve(self, path: str) -> Inode | str:
        if not path or "\0" in path:
            return "EINVAL"
        root = self.inodes[1]
        node = root
        parents: list[Inode] = []
        for name in filter(None, path.split("/")):
            if node.data is not None:
                return "ENOTDIR"
            if name == ".":
                continue
            if name == "..":
                node = parents.pop() if parents else root
                continue
            entry = dict(node.entries).get(name)
            if entry is None:
                return "ENOENT"
            parents.append(node)
            node = self.inodes[entry]
        if path.endswith("/") and node.data is not None:
            return "ENOTDIR"
        return node

    def read(self, inode: int, offset: int, count: int) -> bytes:
        data = self.inodes[inode].data
        assert data is not None
        return data[offset : offset + count]


@dataclass(frozen=True)
class Descriptor:
    inode: int
    offset: int = 0


class DescriptorTable:
    """Each open slot owns an independent offset."""

    def __init__(self, capacity: int = 4) -> None:
        if not 1 <= capacity <= 1024:
            raise ValueError("descriptor slots must be from 1 through 1024")
        self._slots: list[Descriptor | None] = [None] * capacity

    def get(self, fd: int) -> Descriptor | None:
        return self._slots[fd] if 0 <= fd < len(self._slots) else None

    def open(self, inode: int) -> int | None:
        for fd, slot in enumerate(self._slots):
            if slot is None:
                self._slots[fd] = Descriptor(inode)
                return fd
        return None

    def advance(self, fd: int, count: int) -> None:
        slot = self.get(fd)
        if slot is not None and count >= 0:
            self._slots[fd] = Descriptor(slot.inode, slot.offset + count)

    def close(self, fd: int) -> bool:
        if self.get(fd) is None:
            return False
        self._slots[fd] = None
        return True


class Kernel:
    """Dispatch calls across the filesystem and descriptor boundaries."""

    def __init__(self) -> None:
        self.fs = FileSystem()
        self.descriptors = DescriptorTable()

    def open(self, path: str) -> Event:
        event = {"call": "open", "path": path}
        node = self.fs.resolve(path)
        if isinstance(node, str):
            return {**event, "error": node}
        if node.data is None:
            return {**event, "error": "EISDIR"}
        fd = self.descriptors.open(node.number)
        if fd is None:
            return {**event, "error": "EMFILE"}
        return {**event, "inode": node.number, "fd": fd, "offset": 0}

    def read(self, fd: int, count: int) -> tuple[Event, bytes]:
        event = {"call": "read", "fd": fd}
        slot = self.descriptors.get(fd)
        if slot is None:
            return {**event, "error": "EBADF"}, b""
        if count < 0:
            return {**event, "error": "EINVAL"}, b""
        data = self.fs.read(slot.inode, slot.offset, count)
        self.descriptors.advance(fd, len(data))
        return {
            **event,
            "inode": slot.inode,
            "offset": slot.offset,
            "bytes": len(data),
            "nextOffset": slot.offset + len(data),
        }, data

    def close(self, fd: int) -> Event:
        event = {"call": "close", "fd": fd}
        return event if self.descriptors.close(fd) else {**event, "error": "EBADF"}

    def list(self, path: str) -> Event:
        event = {"call": "list", "path": path}
        node = self.fs.resolve(path)
        if isinstance(node, str):
            return {**event, "error": node}
        if node.data is not None:
            return {**event, "error": "ENOTDIR"}
        names = [name for name, _ in node.entries]
        return {**event, "inode": node.number, "names": names}


def run_command(command: str) -> dict[str, Any]:
    words = [word for word in re.split(r"[ \t\n\r\f\v]+", command) if word]
    kernel = Kernel()
    events: list[Event] = []
    output = ""
    match words:
        case ["cat", path]:
            opened = kernel.open(path)
            events.append(opened)
            if "fd" in opened:
                fd = opened["fd"]
                contents = bytearray()
                while True:
                    read, data = kernel.read(fd, 8)
                    events.append(read)
                    if not data:
                        break
                    contents.extend(data)
                events.append(kernel.close(fd))
                output = contents.decode("utf-8")
        case ["ls"] | ["ls", _]:
            listed = kernel.list(words[1] if len(words) == 2 else "/")
            events.append(listed)
            output = "".join(f"{name}\n" for name in listed.get("names", ()))
        case []:
            pass
        case _:
            output = "usage: cat PATH or ls [PATH]\n"
    for event in events:
        if "error" in event:
            output += f"error: {event['error']}\n"
    return {"output": output, "events": events}
