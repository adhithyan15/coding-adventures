package traced

import (
	"bytes"
	"debug/elf"
	"encoding/binary"
	"strings"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/trace"
)

// Exec (KILN04 section 5.1).
//
// execve(path) reads the program.  Then the KERNEL, without any system call a
// tracer can see, reads more:
//
//	a "#!" script     -> the interpreter it names, and if that is a script
//	                     too, its interpreter, up to four levels deep
//	a dynamic ELF     -> the loader named by its PT_INTERP header
//
// A manifest that missed those would call a cached run valid after the
// interpreter or the loader changed.  So every one of them is read here, from
// the tracked tree when the file is tracked and from the post-run disk
// otherwise, and observed exactly like a file the program opened itself.

// maxShebangDepth is how many "#!" hops the kernel follows.
const maxShebangDepth = 4

// exec observes an execve/execveat and, when it succeeded, everything the
// kernel loaded for it.
func (c *classifier) exec(ev *trace.Event) {
	tg, ok := c.targetOf(ev, true)
	if !ok {
		return
	}
	// An exec failure says nothing about WHICH name was missing (a PATH search
	// probes every directory), so it is observed but never remembered as absent.
	c.observe(tg.abs, presence(ev.Result), false)
	if !ev.Result.OK {
		return
	}
	if tg.virtual {
		// /proc/self/exe and friends: the real file is not the one the
		// classifier would read.
		c.reason(rExecUnreadable)
		return
	}
	c.loadChain(tg.abs, ev.Cwd)
}

// loadChain follows the loading chain of the program at abs.  cwd is the
// directory a relative interpreter path is looked up from.
func (c *classifier) loadChain(abs, cwd string) {
	for level := 0; ; level++ {
		head, ok := c.readHead(abs)
		if !ok {
			c.reason(rExecUnreadable)
			return
		}
		switch {
		case bytes.HasPrefix(head, []byte("#!")):
			if level >= maxShebangDepth {
				c.reason(rExecDepth)
				return
			}
			interp, ok := shebangInterpreter(head)
			if !ok {
				c.reason(rExecFormat)
				return
			}
			tg, ok := c.resolve(join(cwd, "", interp), true)
			if !ok {
				return
			}
			c.observe(tg.abs, true, false)
			if tg.virtual {
				c.reason(rExecUnreadable)
				return
			}
			abs = tg.abs
		case bytes.HasPrefix(head, []byte(elf.ELFMAG)):
			interp, ok := elfInterpreter(head)
			if !ok {
				c.reason(rExecELF)
				return
			}
			if interp == "" {
				return // statically linked: no loader
			}
			if tg, ok := c.resolve(interp, true); ok {
				c.observe(tg.abs, true, false)
			}
			return
		default:
			// binfmt_misc, a.out, a file the kernel would refuse: whatever
			// loaded it is invisible to us.
			c.reason(rExecFormat)
			return
		}
	}
}

// readHead returns the first bytes of the file at abs: from the tracked tree
// when it is a tracked file (those bytes are what the key digests), else from
// the post-run disk.  The disk is never asked under $HOME or TMPDIR (fresh
// tmpfs, gone after the run).  A file read from the disk is remembered: if the
// action changes it after the exec, the bytes are not the ones the kernel read
// (checkStability).
func (c *classifier) readHead(abs string) ([]byte, bool) {
	if c.underTmp(abs) {
		return nil, false
	}
	if rel, ok := c.rel(abs); ok {
		if c.cfg.Tree.Stat(rel).Kind == KindFile {
			if b, err := c.cfg.Tree.Head(rel, headLimit); err == nil {
				return b, true
			}
		}
		if _, seen := c.execReads[abs]; !seen {
			c.execReads[abs] = c.start // the earliest exec decides
		}
	}
	b, err := c.cfg.FS.ReadHead(abs, headLimit)
	return b, err == nil
}

// shebangInterpreter reads the interpreter path from a "#!" line the way the
// kernel does: after "#!", skip blanks, take the text up to the next blank.
// The line is at most 255 bytes, and an optional single argument after the
// interpreter is not a file the kernel reads.
func shebangInterpreter(head []byte) (string, bool) {
	line := head[2:]
	if len(line) > 253 {
		line = line[:253]
	}
	if i := bytes.IndexByte(line, '\n'); i >= 0 {
		line = line[:i]
	}
	s := strings.TrimLeft(string(line), " \t")
	if i := strings.IndexAny(s, " \t"); i >= 0 {
		s = s[:i]
	}
	if s == "" || strings.IndexByte(s, 0) >= 0 {
		return "", false
	}
	return s, true
}

// elfInterpreter returns the PT_INTERP path of an ELF image, "" when it has
// none.  Only the program-header table is read, from the head of the file:
// that is where it lives, and it is all a loader name needs.  debug/elf
// supplies the structures and constants; its File type is not used because it
// insists on reading the section headers at the end of the file.
func elfInterpreter(head []byte) (string, bool) {
	if len(head) < elf.EI_NIDENT {
		return "", false
	}
	var order binary.ByteOrder
	switch elf.Data(head[elf.EI_DATA]) {
	case elf.ELFDATA2LSB:
		order = binary.LittleEndian
	case elf.ELFDATA2MSB:
		order = binary.BigEndian
	default:
		return "", false
	}
	var phoff uint64
	var phentsize, phnum uint16
	r := bytes.NewReader(head)
	switch elf.Class(head[elf.EI_CLASS]) {
	case elf.ELFCLASS64:
		var h elf.Header64
		if binary.Read(r, order, &h) != nil {
			return "", false
		}
		phoff, phentsize, phnum = h.Phoff, h.Phentsize, h.Phnum
	case elf.ELFCLASS32:
		var h elf.Header32
		if binary.Read(r, order, &h) != nil {
			return "", false
		}
		phoff, phentsize, phnum = uint64(h.Phoff), h.Phentsize, h.Phnum
	default:
		return "", false
	}
	for i := uint64(0); i < uint64(phnum); i++ {
		off := phoff + i*uint64(phentsize)
		if off > uint64(len(head)) {
			return "", false
		}
		pr := bytes.NewReader(head[off:])
		var typ elf.ProgType
		var poff, psize uint64
		if elf.Class(head[elf.EI_CLASS]) == elf.ELFCLASS64 {
			var p elf.Prog64
			if binary.Read(pr, order, &p) != nil {
				return "", false
			}
			typ, poff, psize = elf.ProgType(p.Type), p.Off, p.Filesz
		} else {
			var p elf.Prog32
			if binary.Read(pr, order, &p) != nil {
				return "", false
			}
			typ, poff, psize = elf.ProgType(p.Type), uint64(p.Off), uint64(p.Filesz)
		}
		if typ != elf.PT_INTERP {
			continue
		}
		if psize == 0 || poff > uint64(len(head)) || psize > uint64(len(head))-poff {
			return "", false
		}
		s := head[poff : poff+psize]
		if i := bytes.IndexByte(s, 0); i >= 0 {
			s = s[:i]
		}
		if len(s) == 0 || s[0] != '/' {
			return "", false
		}
		return string(s), true
	}
	return "", true
}
