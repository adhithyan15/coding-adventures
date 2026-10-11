# frozen_string_literal: true

# = Choosing a cow file safely
#
# <tt>-f NAME</tt> / <tt>--file NAME</tt> picks which cow to draw. The name is
# glued into a path -- <tt><cows dir>/NAME.cow</tt> -- so it is untrusted input
# that decides which file this program opens and echoes to stdout. Left
# unchecked, a name can walk out of the cows directory (issue #12169):
#
#   name given                    path actually opened
#   ----------------------------- ------------------------------------
#   tux                           code/specs/cows/tux.cow     (fine)
#   ../../../../home/me/notes     /home/me/notes.cow          (escape)
#   C:\Users\me\x                 C:\Users\me\x.cow           (Windows)
#
# The forced <tt>.cow</tt> suffix limits what can be read, and a local CLI
# already runs with its caller's privileges -- but a wrapper script, web
# service or CI job that forwards someone else's string to <tt>cowsay -f</tt>
# would hand that someone a file-read primitive. So we defend in two layers,
# mirroring the C#, F#, Java, Kotlin, Perl, Haskell, Dart, Lua and Swift ports:
#
# 1. *Syntactic check* (CowPath.safe_name?). A cow name is a bare file stem,
#    so anything that could mean "some other directory" is refused:
#
#      /    path separator
#      \    path separator on Windows
#      ..   parent-directory step
#      :    Windows drive letter (C:x) / alternate data stream
#      NUL  C APIs stop reading at it, so the OS would see a different
#           name from the one we checked
#      ""   the empty name
#
#    An absolute path always contains /, \ or :, so it falls to the same rule.
#
# 2. *Containment check* (CowPath.resolve). After joining, both the cows
#    directory and the candidate are canonicalized with File.realpath
#    (symlinks and ./.. resolved by the OS) and the candidate must still sit
#    inside the cows directory. This catches what string inspection cannot
#    see -- e.g. a symlink planted in the cows directory that points elsewhere.
#
# A name failing either layer is treated exactly like a cow that does not
# exist: we quietly draw default.cow, matching every other port and cowsay's
# long-standing "unknown cow -> default cow" behaviour. Nothing is
# URL-decoded: %2F is three literal characters to the OS, never a slash.
#
# Only the standard library is used, so this can be tested on its own.
module CowPath
  # Characters that let a "name" smuggle in a directory, a drive, or a
  # truncation point.
  FORBIDDEN_CHARS = ["/", "\\", ":", "\0"].freeze

  module_function

  # True when +cow_name+ is a bare file stem that cannot name another directory.
  def safe_name?(cow_name)
    return false unless cow_name.is_a?(String)
    return false if cow_name.empty? || cow_name.include?("..")

    FORBIDDEN_CHARS.none? { |ch| cow_name.include?(ch) }
  end

  # Map +cow_name+ to a .cow file inside +cows_dir+, falling back to
  # <tt>cows_dir/default.cow</tt> when the name is unsafe, the file is missing,
  # or the resolved file escapes +cows_dir+.
  def resolve(cow_name, cows_dir)
    default_path = File.join(cows_dir, "default.cow")
    return default_path unless safe_name?(cow_name)

    # File.realpath raises for a path that does not exist -- the "unknown cow"
    # case -- so any SystemCallError simply means "use the default".
    root = File.realpath(cows_dir)
    resolved = File.realpath(File.join(cows_dir, "#{cow_name}.cow"))
    inside = resolved.start_with?(root.end_with?("/") ? root : "#{root}/")
    (inside && File.file?(resolved)) ? resolved : default_path
  rescue SystemCallError
    default_path
  end
end
