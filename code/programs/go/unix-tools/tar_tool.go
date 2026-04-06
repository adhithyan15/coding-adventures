// =========================================================================
// tar — Tape Archive Utility
// =========================================================================
//
// The `tar` utility creates, extracts, and lists archive files. Despite
// its name referring to "tape," it's now used almost exclusively for
// creating file archives on disk.
//
// # The tar format
//
// A tar archive is a sequence of file entries, each consisting of:
//
//   ┌─────────────────────┐
//   │ Header (512 bytes)  │  File metadata: name, size, permissions, etc.
//   ├─────────────────────┤
//   │ File data           │  Rounded up to a multiple of 512 bytes
//   │ (0 or more blocks)  │
//   ├─────────────────────┤
//   │ Header (512 bytes)  │  Next file...
//   ├─────────────────────┤
//   │ File data           │
//   ├─────────────────────┤
//   │ ...                 │
//   ├─────────────────────┤
//   │ Two zero blocks     │  End-of-archive marker (1024 zero bytes)
//   └─────────────────────┘
//
// # Header format (POSIX/UStar)
//
// The 512-byte header contains these fields:
//
//   Offset  Size  Field
//   ──────  ────  ──────────────────
//   0       100   File name
//   100     8     File mode (octal)
//   108     8     Owner UID (octal)
//   116     8     Group GID (octal)
//   124     12    File size (octal)
//   136     12    Modification time (octal, Unix epoch)
//   148     8     Header checksum
//   156     1     Type flag ('0'=file, '5'=dir)
//   157     100   Link name
//   257     6     Magic ("ustar")
//   263     2     Version ("00")
//   265     32    Owner name
//   297     32    Group name
//   329     8     Device major
//   337     8     Device minor
//   345     155   Prefix (for long names)
//
// # Basic usage
//
//   tar -cf archive.tar file1 file2   Create archive
//   tar -xf archive.tar               Extract archive
//   tar -tf archive.tar               List archive contents
//   tar -cvf archive.tar dir/         Create with verbose listing
//
// # Architecture
//
//   tar.json (spec)              tar_tool.go (this file)
//   ┌──────────────────┐       ┌──────────────────────────────────┐
//   │ flags: -c,-x,-t  │       │ create: walk files, write headers│
//   │ -f,-v,-C         │──────>│ extract: read headers, write files│
//   │ -z,-j,-J         │       │ list: read headers, print names  │
//   │ arg: FILES...    │       │ all using 512-byte block format  │
//   └──────────────────┘       └──────────────────────────────────┘

package main

import (
	"archive/tar"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"

	clibuilder "github.com/adhithyan15/coding-adventures/code/packages/go/cli-builder"
)

// =========================================================================
// TarOptions — configuration for the tar operation
// =========================================================================

type TarOptions struct {
	Create    bool   // -c: create archive
	Extract   bool   // -x: extract archive
	List      bool   // -t: list archive contents
	File      string // -f: archive file name
	Verbose   bool   // -v: verbose listing
	Directory string // -C: change to directory before operating
	Gzip      bool   // -z: gzip compression (stub)
	Bzip2     bool   // -j: bzip2 compression (stub)
	Xz        bool   // -J: xz compression (stub)
	KeepOld   bool   // -k: don't replace existing files
	StripN    int    // --strip-components: strip N leading path components
}

func pathWithinBaseDir(baseDir string, candidate string) bool {
	rel, err := filepath.Rel(baseDir, candidate)
	if err != nil {
		return false
	}
	return rel == "." || (rel != ".." && !strings.HasPrefix(rel, ".."+string(os.PathSeparator)))
}

// =========================================================================
// tarCreate — create a new tar archive
// =========================================================================
//
// Creating an archive involves:
//   1. Open the output file (or stdout)
//   2. For each input file/directory:
//      a. Walk the directory tree
//      b. For each file, write a tar header + file data
//   3. Close the tar writer (writes end-of-archive marker)
//
// We use Go's archive/tar package which handles the 512-byte block
// format, checksums, and padding automatically.

func tarCreate(files []string, opts TarOptions, stdout io.Writer, stderr io.Writer) int {
	// Open output.
	var output io.Writer
	if opts.File == "" || opts.File == "-" {
		output = stdout
	} else {
		f, err := os.Create(opts.File)
		if err != nil {
			fmt.Fprintf(stderr, "tar: %s: %s\n", opts.File, err)
			return 2
		}
		defer f.Close()
		output = f
	}

	tw := tar.NewWriter(output)
	defer tw.Close()

	for _, file := range files {
		// If -C was specified, resolve paths relative to that directory.
		fullPath := file
		if opts.Directory != "" {
			fullPath = filepath.Join(opts.Directory, file)
		}

		err := tarAddPath(tw, fullPath, file, opts, stdout, stderr)
		if err != nil {
			fmt.Fprintf(stderr, "tar: %s: %s\n", file, err)
			return 2
		}
	}

	return 0
}

// =========================================================================
// tarAddPath — add a file or directory to the tar archive
// =========================================================================

func tarAddPath(tw *tar.Writer, diskPath, archivePath string, opts TarOptions,
	stdout io.Writer, stderr io.Writer) error {

	info, err := os.Lstat(diskPath)
	if err != nil {
		return err
	}

	if info.IsDir() {
		// Walk the directory tree.
		return filepath.Walk(diskPath, func(p string, fi os.FileInfo, err error) error {
			if err != nil {
				return err
			}

			// Compute the archive-relative path.
			relPath, err := filepath.Rel(filepath.Dir(diskPath), p)
			if err != nil {
				relPath = p
			}

			return tarAddFile(tw, p, relPath, fi, opts, stdout)
		})
	}

	return tarAddFile(tw, diskPath, archivePath, info, opts, stdout)
}

// =========================================================================
// tarAddFile — add a single file entry to the tar archive
// =========================================================================

func tarAddFile(tw *tar.Writer, diskPath, archivePath string, info os.FileInfo,
	opts TarOptions, stdout io.Writer) error {

	// Create the tar header from file info.
	header, err := tar.FileInfoHeader(info, "")
	if err != nil {
		return err
	}

	// Set the archive name (may differ from disk name).
	header.Name = archivePath

	// Ensure directories end with /.
	if info.IsDir() && !strings.HasSuffix(header.Name, "/") {
		header.Name += "/"
	}

	// Write the header.
	err = tw.WriteHeader(header)
	if err != nil {
		return err
	}

	if opts.Verbose {
		fmt.Fprintln(stdout, header.Name)
	}

	// Write file data (directories have no data).
	if !info.IsDir() && info.Mode().IsRegular() {
		f, err := os.Open(diskPath)
		if err != nil {
			return err
		}
		defer f.Close()

		_, err = io.Copy(tw, f)
		if err != nil {
			return err
		}
	}

	return nil
}

// =========================================================================
// tarExtract — extract files from a tar archive
// =========================================================================
//
// Extracting involves:
//   1. Open the archive file
//   2. Read each header
//   3. Create the corresponding file or directory on disk
//   4. Copy the file data from the archive to the new file
//
// Security note: we validate that extracted paths don't escape the
// target directory (no "../" traversal attacks).

func tarExtract(files []string, opts TarOptions, stdout io.Writer, stderr io.Writer) int {
	// Open input.
	var input io.Reader
	if opts.File == "" || opts.File == "-" {
		input = os.Stdin
	} else {
		f, err := os.Open(opts.File)
		if err != nil {
			fmt.Fprintf(stderr, "tar: %s: %s\n", opts.File, err)
			return 2
		}
		defer f.Close()
		input = f
	}

	tr := tar.NewReader(input)
	targetDir := "."
	if opts.Directory != "" {
		targetDir = opts.Directory
	}
	targetDirAbs, err := filepath.Abs(targetDir)
	if err != nil {
		fmt.Fprintf(stderr, "tar: %s: %s\n", targetDir, err)
		return 2
	}

	// Build a set of requested files for filtering.
	filterSet := make(map[string]bool)
	for _, f := range files {
		filterSet[f] = true
	}

	for {
		header, err := tr.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			fmt.Fprintf(stderr, "tar: %s\n", err)
			return 2
		}

		// Apply --strip-components.
		name := header.Name
		if opts.StripN > 0 {
			name = tarStripComponents(name, opts.StripN)
			if name == "" {
				continue
			}
		}

		// Filter: if specific files were requested, only extract those.
		if len(filterSet) > 0 {
			matched := false
			for f := range filterSet {
				if name == f || strings.HasPrefix(name, f+"/") {
					matched = true
					break
				}
			}
			if !matched {
				continue
			}
		}

		// Security: prevent path traversal.
		targetPath := filepath.Join(targetDirAbs, name)
		targetPathAbs, err := filepath.Abs(targetPath)
		if err != nil {
			fmt.Fprintf(stderr, "tar: %s: %s\n", name, err)
			continue
		}
		if !pathWithinBaseDir(targetDirAbs, targetPathAbs) {
			fmt.Fprintf(stderr, "tar: %s: path escapes target directory\n", name)
			continue
		}
		targetPath = targetPathAbs

		if opts.Verbose {
			fmt.Fprintln(stdout, name)
		}

		switch header.Typeflag {
		case tar.TypeDir:
			err = os.MkdirAll(targetPath, os.FileMode(header.Mode))
			if err != nil {
				fmt.Fprintf(stderr, "tar: %s: %s\n", name, err)
				return 2
			}

		case tar.TypeReg, 0:
			// Check keep-old-files flag.
			if opts.KeepOld {
				if _, err := os.Lstat(targetPath); err == nil {
					continue
				}
			}

			// Ensure parent directory exists.
			err = os.MkdirAll(filepath.Dir(targetPath), 0755)
			if err != nil {
				fmt.Fprintf(stderr, "tar: %s: %s\n", name, err)
				return 2
			}

			f, err := os.OpenFile(targetPath, os.O_WRONLY|os.O_CREATE|os.O_TRUNC,
				os.FileMode(header.Mode))
			if err != nil {
				fmt.Fprintf(stderr, "tar: %s: %s\n", name, err)
				return 2
			}

			_, err = io.Copy(f, tr)
			f.Close()
			if err != nil {
				fmt.Fprintf(stderr, "tar: %s: %s\n", name, err)
				return 2
			}

		case tar.TypeSymlink:
			// Ensure parent directory exists.
			err = os.MkdirAll(filepath.Dir(targetPath), 0755)
			if err != nil {
				fmt.Fprintf(stderr, "tar: %s: %s\n", name, err)
				return 2
			}
			if filepath.IsAbs(header.Linkname) {
				fmt.Fprintf(stderr, "tar: %s: symlink target escapes target directory\n", name)
				continue
			}
			resolvedLinkTarget := filepath.Join(filepath.Dir(targetPath), header.Linkname)
			resolvedLinkTargetAbs, err := filepath.Abs(resolvedLinkTarget)
			if err != nil {
				fmt.Fprintf(stderr, "tar: %s: %s\n", name, err)
				continue
			}
			if !pathWithinBaseDir(targetDirAbs, resolvedLinkTargetAbs) {
				fmt.Fprintf(stderr, "tar: %s: symlink target escapes target directory\n", name)
				continue
			}
			err = os.Symlink(header.Linkname, targetPath)
			if err != nil {
				fmt.Fprintf(stderr, "tar: %s: %s\n", name, err)
			}
		}
	}

	return 0
}

// =========================================================================
// tarList — list the contents of a tar archive
// =========================================================================

func tarList(files []string, opts TarOptions, stdout io.Writer, stderr io.Writer) int {
	// Open input.
	var input io.Reader
	if opts.File == "" || opts.File == "-" {
		input = os.Stdin
	} else {
		f, err := os.Open(opts.File)
		if err != nil {
			fmt.Fprintf(stderr, "tar: %s: %s\n", opts.File, err)
			return 2
		}
		defer f.Close()
		input = f
	}

	tr := tar.NewReader(input)

	// Build filter set.
	filterSet := make(map[string]bool)
	for _, f := range files {
		filterSet[f] = true
	}

	for {
		header, err := tr.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			fmt.Fprintf(stderr, "tar: %s\n", err)
			return 2
		}

		name := header.Name

		// Filter if specific files requested.
		if len(filterSet) > 0 {
			matched := false
			for f := range filterSet {
				if name == f || strings.HasPrefix(name, f+"/") {
					matched = true
					break
				}
			}
			if !matched {
				continue
			}
		}

		if opts.Verbose {
			// Verbose listing includes permissions, size, date, name.
			fmt.Fprintf(stdout, "%s %8d %s %s\n",
				os.FileMode(header.Mode).String(),
				header.Size,
				header.ModTime.Format("2006-01-02 15:04"),
				name)
		} else {
			fmt.Fprintln(stdout, name)
		}
	}

	return 0
}

// =========================================================================
// tarStripComponents — strip leading path components from a name
// =========================================================================
//
// For example, with n=1:
//   "dir/subdir/file.txt" → "subdir/file.txt"
//   "dir/" → "" (directory entry stripped entirely)

func tarStripComponents(name string, n int) string {
	parts := strings.SplitN(name, "/", n+1)
	if len(parts) <= n {
		return ""
	}
	return parts[n]
}

// =========================================================================
// runTar — the testable core of the tar tool
// =========================================================================

func runTar(specPath string, argv []string, stdout io.Writer, stderr io.Writer) int {
	// Step 1: Create the parser from the spec.
	parser, err := clibuilder.NewParser(specPath, argv)
	if err != nil {
		fmt.Fprintf(stderr, "tar: %s\n", err)
		return 2
	}

	// Step 2: Parse the arguments.
	result, err := parser.Parse()
	if err != nil {
		fmt.Fprintf(stderr, "%s\n", err)
		return 2
	}

	// Step 3: Handle the result.
	switch r := result.(type) {

	case *clibuilder.HelpResult:
		fmt.Fprintln(stdout, r.Text)
		return 0

	case *clibuilder.VersionResult:
		fmt.Fprintln(stdout, r.Version)
		return 0

	case *clibuilder.ParseResult:
		opts := TarOptions{
			Create:  getBool(r.Flags, "create"),
			Extract: getBool(r.Flags, "extract"),
			List:    getBool(r.Flags, "list"),
			Verbose: getBool(r.Flags, "verbose"),
			Gzip:    getBool(r.Flags, "gzip"),
			Bzip2:   getBool(r.Flags, "bzip2"),
			Xz:      getBool(r.Flags, "xz"),
			KeepOld: getBool(r.Flags, "keep_old_files"),
		}

		if f, ok := r.Flags["file"].(string); ok {
			opts.File = f
		}
		if d, ok := r.Flags["directory"].(string); ok {
			opts.Directory = d
		}
		if n, ok := getInt(r.Flags, "strip_components"); ok {
			opts.StripN = n
		}

		// Check for compression flags (stubs).
		if opts.Gzip || opts.Bzip2 || opts.Xz {
			fmt.Fprintf(stderr, "tar: compression is not supported in this implementation\n")
			return 2
		}

		// Get file list.
		files := getStringSlice(r.Arguments, "files")

		// Dispatch to the appropriate operation.
		if opts.Create {
			if len(files) == 0 {
				fmt.Fprintf(stderr, "tar: cowardly refusing to create an empty archive\n")
				return 2
			}
			return tarCreate(files, opts, stdout, stderr)
		}

		if opts.Extract {
			return tarExtract(files, opts, stdout, stderr)
		}

		if opts.List {
			return tarList(files, opts, stdout, stderr)
		}

		fmt.Fprintf(stderr, "tar: you must specify one of -c, -x, or -t\n")
		return 2

	default:
		fmt.Fprintf(stderr, "tar: unexpected result type: %T\n", result)
		return 2
	}
}
