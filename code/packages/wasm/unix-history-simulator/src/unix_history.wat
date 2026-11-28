;; HOS-003e bounded First Edition Unix teaching image.
;;
;; This module contains only guest state and guest transitions. It imports no
;; host functions. The fixture adapter owns command parsing, byte marshalling,
;; and presentation of results as portable JSON events.
(module
  (memory (export "memory") 1 1)

  ;; Four descriptors match the portable teaching model. An inode of zero
  ;; means that a slot is free; actual image inodes start at one.
  (global $fd0_inode (mut i32) (i32.const 0))
  (global $fd0_offset (mut i32) (i32.const 0))
  (global $fd1_inode (mut i32) (i32.const 0))
  (global $fd1_offset (mut i32) (i32.const 0))
  (global $fd2_inode (mut i32) (i32.const 0))
  (global $fd2_offset (mut i32) (i32.const 0))
  (global $fd3_inode (mut i32) (i32.const 0))
  (global $fd3_offset (mut i32) (i32.const 0))

  ;; Strings are kept in read-only guest memory. Their addresses are part of
  ;; the simple fixture-facing ABI returned by entry_name_ptr.
  (data (i32.const 256) "Hello, Unix!\0a")
  (data (i32.const 288) "A directory maps a name to an inode.\0aAn open descriptor tracks a byte offset.\0a")
  (data (i32.const 384) "hello")
  (data (i32.const 392) "empty")
  (data (i32.const 400) "etc")
  (data (i32.const 408) "notes")
  (data (i32.const 416) ".")
  (data (i32.const 417) "..")

  ;; Error values are negative so every successful inode/fd/count remains
  ;; nonnegative. Callers compare these constants rather than host errnos.
  ;; EINVAL=-1, ENOENT=-2, ENOTDIR=-3, EISDIR=-4, EBADF=-5, EMFILE=-6.

  (func $eq (param $ptr i32) (param $len i32) (param $literal i32) (param $literal_len i32) (result i32)
    (local $i i32)
    (if (i32.ne (local.get $len) (local.get $literal_len)) (then (return (i32.const 0))))
    (block $same
      (loop $compare
        (br_if $same (i32.ge_u (local.get $i) (local.get $len)))
        (if (i32.ne
              (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))
              (i32.load8_u (i32.add (local.get $literal) (local.get $i))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $compare)))
    (i32.const 1))

  (func $kind (param $inode i32) (result i32)
    (if (i32.eq (local.get $inode) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $inode) (i32.const 4)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $inode) (i32.const 2)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $inode) (i32.const 3)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $inode) (i32.const 5)) (then (return (i32.const 2))))
    (i32.const -2))

  (func (export "kind") (param $inode i32) (result i32)
    (call $kind (local.get $inode)))

  (func (export "reset")
    (global.set $fd0_inode (i32.const 0)) (global.set $fd0_offset (i32.const 0))
    (global.set $fd1_inode (i32.const 0)) (global.set $fd1_offset (i32.const 0))
    (global.set $fd2_inode (i32.const 0)) (global.set $fd2_offset (i32.const 0))
    (global.set $fd3_inode (i32.const 0)) (global.set $fd3_offset (i32.const 0)))

  ;; Resolve each component while walking the image. In particular, the
  ;; current inode must be a directory before processing '..'; this makes
  ;; '/hello/..' fail with ENOTDIR instead of normalizing through a file.
  ;; Parent inode numbers are held in a bounded guest-memory stack at 16384.
  (func (export "resolve") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32) (local $start i32) (local $seglen i32)
    (local $node i32) (local $child i32) (local $depth i32)
    (local $is_dot i32) (local $is_dotdot i32)
    (local $base i32)
    (if (i32.lt_s (local.get $ptr) (i32.const 4096)) (then (return (i32.const -1))))
    (if (i32.lt_s (local.get $len) (i32.const 0)) (then (return (i32.const -1))))
    (if (i32.gt_u (local.get $ptr) (i32.const 8192)) (then (return (i32.const -1))))
    (if (i32.gt_u (local.get $len) (i32.const 4096)) (then (return (i32.const -1))))
    (if (i32.gt_u (local.get $len) (i32.sub (i32.const 8192) (local.get $ptr)))
      (then (return (i32.const -1))))
    (if (i32.eqz (local.get $len)) (then (return (i32.const -1))))
    (local.set $node (i32.const 1))
    (local.set $base (i32.const 16384))
    (block $done
      (loop $next_component
        ;; Skip repeated separators. A trailing separator is checked below.
        (block $separator_done
          (loop $skip_separator
            (br_if $separator_done (i32.ge_u (local.get $i) (local.get $len)))
            (br_if $separator_done
              (i32.ne (i32.load8_u (i32.add (local.get $ptr) (local.get $i))) (i32.const 47)))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $skip_separator)))
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $start (local.get $i))
        (block $component_done
          (loop $scan_component
            (br_if $component_done (i32.ge_u (local.get $i) (local.get $len)))
            (br_if $component_done
              (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $i))) (i32.const 47)))
            (if (i32.eqz (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
              (then (return (i32.const -1))))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $scan_component)))
        (local.set $seglen (i32.sub (local.get $i) (local.get $start)))
        (if (i32.ne (call $kind (local.get $node)) (i32.const 1))
          (then (return (i32.const -3))))
        (local.set $is_dot
          (call $eq (i32.add (local.get $ptr) (local.get $start)) (local.get $seglen) (i32.const 416) (i32.const 1)))
        (local.set $is_dotdot
          (call $eq (i32.add (local.get $ptr) (local.get $start)) (local.get $seglen) (i32.const 417) (i32.const 2)))
        (if (local.get $is_dot)
          (then)
          (else
            (if (local.get $is_dotdot)
              (then
                (if (i32.gt_s (local.get $depth) (i32.const 0))
                  (then
                    (local.set $depth (i32.sub (local.get $depth) (i32.const 1)))
                    (local.set $node (i32.load (i32.add (local.get $base) (i32.mul (local.get $depth) (i32.const 4))))))))
              (else
                (local.set $child (i32.const 0))
                (if (i32.eq (local.get $node) (i32.const 1))
                  (then
                    (if (call $eq (i32.add (local.get $ptr) (local.get $start)) (local.get $seglen) (i32.const 384) (i32.const 5)) (then (local.set $child (i32.const 2))))
                    (if (call $eq (i32.add (local.get $ptr) (local.get $start)) (local.get $seglen) (i32.const 392) (i32.const 5)) (then (local.set $child (i32.const 3))))
                    (if (call $eq (i32.add (local.get $ptr) (local.get $start)) (local.get $seglen) (i32.const 400) (i32.const 3)) (then (local.set $child (i32.const 4))))))
                (if (i32.eq (local.get $node) (i32.const 4))
                  (then
                    (if (call $eq (i32.add (local.get $ptr) (local.get $start)) (local.get $seglen) (i32.const 408) (i32.const 5)) (then (local.set $child (i32.const 5))))))
                (if (i32.eqz (local.get $child)) (then (return (i32.const -2))))
                (if (i32.ge_u (local.get $depth) (i32.const 512)) (then (return (i32.const -1))))
                (i32.store (i32.add (local.get $base) (i32.mul (local.get $depth) (i32.const 4))) (local.get $node))
                (local.set $depth (i32.add (local.get $depth) (i32.const 1)))
                (local.set $node (local.get $child))))))
        (br $next_component)))
    (if (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 1)))) (i32.const 47))
      (then
        (if (i32.ne (call $kind (local.get $node)) (i32.const 1))
          (then (return (i32.const -3))))))
    (local.get $node))

  (func (export "open") (param $inode i32) (result i32)
    (if (i32.lt_s (local.get $inode) (i32.const 0)) (then (return (local.get $inode))))
    (if (i32.ne (call $kind (local.get $inode)) (i32.const 2))
      (then
        (if (i32.eq (call $kind (local.get $inode)) (i32.const 1))
          (then (return (i32.const -4)))
          (else (return (i32.const -2))))))
    (if (i32.eqz (global.get $fd0_inode))
      (then (global.set $fd0_inode (local.get $inode)) (global.set $fd0_offset (i32.const 0)) (return (i32.const 0))))
    (if (i32.eqz (global.get $fd1_inode))
      (then (global.set $fd1_inode (local.get $inode)) (global.set $fd1_offset (i32.const 0)) (return (i32.const 1))))
    (if (i32.eqz (global.get $fd2_inode))
      (then (global.set $fd2_inode (local.get $inode)) (global.set $fd2_offset (i32.const 0)) (return (i32.const 2))))
    (if (i32.eqz (global.get $fd3_inode))
      (then (global.set $fd3_inode (local.get $inode)) (global.set $fd3_offset (i32.const 0)) (return (i32.const 3))))
    (i32.const -6))

  (func $descriptor_inode (export "descriptor_inode") (param $fd i32) (result i32)
    (if (i32.eq (local.get $fd) (i32.const 0)) (then (if (global.get $fd0_inode) (then (return (global.get $fd0_inode))))))
    (if (i32.eq (local.get $fd) (i32.const 1)) (then (if (global.get $fd1_inode) (then (return (global.get $fd1_inode))))))
    (if (i32.eq (local.get $fd) (i32.const 2)) (then (if (global.get $fd2_inode) (then (return (global.get $fd2_inode))))))
    (if (i32.eq (local.get $fd) (i32.const 3)) (then (if (global.get $fd3_inode) (then (return (global.get $fd3_inode))))))
    (i32.const -5))

  (func $descriptor_offset (export "descriptor_offset") (param $fd i32) (result i32)
    (if (i32.eq (local.get $fd) (i32.const 0)) (then (if (global.get $fd0_inode) (then (return (global.get $fd0_offset))))))
    (if (i32.eq (local.get $fd) (i32.const 1)) (then (if (global.get $fd1_inode) (then (return (global.get $fd1_offset))))))
    (if (i32.eq (local.get $fd) (i32.const 2)) (then (if (global.get $fd2_inode) (then (return (global.get $fd2_offset))))))
    (if (i32.eq (local.get $fd) (i32.const 3)) (then (if (global.get $fd3_inode) (then (return (global.get $fd3_offset))))))
    (i32.const -5))

  ;; Read is a bounded guest-memory copy. The host supplies a maximum of 4096
  ;; bytes and a destination range; the guest updates only this fd's offset.
  (func (export "read") (param $fd i32) (param $dst i32) (param $capacity i32) (result i32)
    (local $inode i32) (local $offset i32) (local $size i32)
    (local $source i32) (local $count i32) (local $i i32)
    (local.set $inode (call $descriptor_inode (local.get $fd)))
    (if (i32.lt_s (local.get $inode) (i32.const 0)) (then (return (local.get $inode))))
    (if (i32.eq (call $kind (local.get $inode)) (i32.const 1)) (then (return (i32.const -4))))
    (if (i32.lt_s (local.get $dst) (i32.const 8192)) (then (return (i32.const -1))))
    (if (i32.lt_s (local.get $capacity) (i32.const 0)) (then (return (i32.const -1))))
    (if (i32.gt_u (local.get $dst) (i32.const 12288)) (then (return (i32.const -1))))
    (if (i32.gt_u (local.get $capacity) (i32.const 4096)) (then (return (i32.const -1))))
    (if (i32.gt_u (local.get $capacity) (i32.sub (i32.const 12288) (local.get $dst))) (then (return (i32.const -1))))
    (local.set $offset (call $descriptor_offset (local.get $fd)))
    (if (i32.eq (local.get $inode) (i32.const 2)) (then (local.set $source (i32.const 256)) (local.set $size (i32.const 13))))
    (if (i32.eq (local.get $inode) (i32.const 3)) (then (local.set $source (i32.const 0)) (local.set $size (i32.const 0))))
    (if (i32.eq (local.get $inode) (i32.const 5)) (then (local.set $source (i32.const 288)) (local.set $size (i32.const 78))))
    (if (i32.lt_u (local.get $offset) (local.get $size))
      (then (local.set $count (i32.sub (local.get $size) (local.get $offset))))
      (else (local.set $count (i32.const 0))))
    (if (i32.gt_u (local.get $count) (local.get $capacity)) (then (local.set $count (local.get $capacity))))
    (block $copied
      (loop $copy
        (br_if $copied (i32.ge_u (local.get $i) (local.get $count)))
        (i32.store8 (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $source) (i32.add (local.get $offset) (local.get $i)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $copy)))
    (if (i32.eq (local.get $fd) (i32.const 0)) (then (global.set $fd0_offset (i32.add (local.get $offset) (local.get $count)))))
    (if (i32.eq (local.get $fd) (i32.const 1)) (then (global.set $fd1_offset (i32.add (local.get $offset) (local.get $count)))))
    (if (i32.eq (local.get $fd) (i32.const 2)) (then (global.set $fd2_offset (i32.add (local.get $offset) (local.get $count)))))
    (if (i32.eq (local.get $fd) (i32.const 3)) (then (global.set $fd3_offset (i32.add (local.get $offset) (local.get $count)))))
    (local.get $count))

  (func (export "close") (param $fd i32) (result i32)
    (if (i32.eq (local.get $fd) (i32.const 0)) (then (if (global.get $fd0_inode) (then (global.set $fd0_inode (i32.const 0)) (global.set $fd0_offset (i32.const 0)) (return (i32.const 0))))))
    (if (i32.eq (local.get $fd) (i32.const 1)) (then (if (global.get $fd1_inode) (then (global.set $fd1_inode (i32.const 0)) (global.set $fd1_offset (i32.const 0)) (return (i32.const 0))))))
    (if (i32.eq (local.get $fd) (i32.const 2)) (then (if (global.get $fd2_inode) (then (global.set $fd2_inode (i32.const 0)) (global.set $fd2_offset (i32.const 0)) (return (i32.const 0))))))
    (if (i32.eq (local.get $fd) (i32.const 3)) (then (if (global.get $fd3_inode) (then (global.set $fd3_inode (i32.const 0)) (global.set $fd3_offset (i32.const 0)) (return (i32.const 0))))))
    (i32.const -5))

  (func (export "entry_count") (param $inode i32) (result i32)
    (if (i32.eq (local.get $inode) (i32.const 1)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $inode) (i32.const 4)) (then (return (i32.const 1))))
    (if (i32.lt_s (call $kind (local.get $inode)) (i32.const 0)) (then (return (i32.const -2))))
    (i32.const 0))

  (func (export "entry_name_ptr") (param $inode i32) (param $index i32) (result i32)
    (if (i32.eq (local.get $inode) (i32.const 1))
      (then
        (if (i32.eq (local.get $index) (i32.const 0)) (then (return (i32.const 384))))
        (if (i32.eq (local.get $index) (i32.const 1)) (then (return (i32.const 392))))
        (if (i32.eq (local.get $index) (i32.const 2)) (then (return (i32.const 400)))))
      (else
        (if (i32.eq (local.get $inode) (i32.const 4))
          (then (if (i32.eq (local.get $index) (i32.const 0)) (then (return (i32.const 408))))))))
    (i32.const -1))

  (func (export "entry_name_len") (param $inode i32) (param $index i32) (result i32)
    (if (i32.eq (local.get $inode) (i32.const 1))
      (then
        (if (i32.eq (local.get $index) (i32.const 0)) (then (return (i32.const 5))))
        (if (i32.eq (local.get $index) (i32.const 1)) (then (return (i32.const 5))))
        (if (i32.eq (local.get $index) (i32.const 2)) (then (return (i32.const 3)))))
      (else
        (if (i32.eq (local.get $inode) (i32.const 4))
          (then (if (i32.eq (local.get $index) (i32.const 0)) (then (return (i32.const 5))))))))
    (i32.const -1))
)
