# Python junction checks must precede realpath

Python 3.10 has no os.path.isjunction, and is_symlink misses NTFS junctions. Inspect os.lstat file attributes for FILE_ATTRIBUTE_REPARSE_POINT before realpath erases that boundary; check ancestors as well as output components. Test real junction roots, missing and existing descendants, and unchanged external targets using the supported Python runtime.
