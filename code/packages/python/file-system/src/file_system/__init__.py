"""File System --- Layer 15 of the computing stack.

A simplified inode-based file system inspired by ext2, the classic Linux file
system. This package implements the Virtual File System (VFS) layer that turns
raw block storage into the familiar world of files and directories.

Components:
    - Superblock: file system metadata (magic number, block/inode counts)
    - Inode / InodeTable: fixed-size records storing file metadata
    - DirectoryEntry: name-to-inode mappings inside directory data blocks
    - BlockBitmap: tracks which data blocks are free vs. allocated
    - OpenFileTable / FileDescriptorTable: system-wide and per-process fd management
    - VFS: the main API --- open, read, write, close, mkdir, unlink, etc.
"""

from file_system.block_bitmap import BlockBitmap
from file_system.constants import (
    BLOCK_SIZE,
    DIRECT_BLOCKS,
    MAX_BLOCKS,
    MAX_INODES,
    MAX_NAME_LENGTH,
    O_APPEND,
    O_CREAT,
    O_RDONLY,
    O_RDWR,
    O_TRUNC,
    O_WRONLY,
    ROOT_INODE,
    SEEK_CUR,
    SEEK_END,
    SEEK_SET,
)
from file_system.directory import DirectoryEntry
from file_system.file_descriptor import FileDescriptorTable, OpenFile, OpenFileTable
from file_system.inode import FileType, Inode
from file_system.inode_table import InodeTable
from file_system.superblock import Superblock
from file_system.vfs import VFS

__all__ = [
    "BLOCK_SIZE",
    "BlockBitmap",
    "DIRECT_BLOCKS",
    "DirectoryEntry",
    "FileDescriptorTable",
    "FileType",
    "Inode",
    "InodeTable",
    "MAX_BLOCKS",
    "MAX_INODES",
    "MAX_NAME_LENGTH",
    "O_APPEND",
    "O_CREAT",
    "O_RDONLY",
    "O_RDWR",
    "O_TRUNC",
    "O_WRONLY",
    "OpenFile",
    "OpenFileTable",
    "ROOT_INODE",
    "SEEK_CUR",
    "SEEK_END",
    "SEEK_SET",
    "Superblock",
    "VFS",
]
