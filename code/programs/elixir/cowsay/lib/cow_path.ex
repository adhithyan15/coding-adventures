defmodule CodingAdventures.Cowsay.CowPath do
  @moduledoc """
  Choosing a cow file safely.

  `-f NAME` / `--file NAME` picks which cow to draw. The name is glued into a
  path -- `<cows dir>/NAME.cow` -- so it is untrusted input that decides which
  file this program opens and echoes to stdout. Left unchecked, a name can walk
  out of the cows directory (issue #12169):

      name given                    path actually opened
      ----------------------------- ------------------------------------
      tux                           code/specs/cows/tux.cow     (fine)
      ../../../../home/me/notes     /home/me/notes.cow          (escape)
      /etc/secret                   /etc/secret.cow             (escape --
                                    Path.join keeps the absolute path's root)
      C:\\Users\\me\\x                 C:\\Users\\me\\x.cow           (Windows)

  The forced `.cow` suffix limits what can be read, and a local CLI already
  runs with its caller's privileges -- but a wrapper script, web service or CI
  job that forwards someone else's string to `cowsay -f` would hand that
  someone a file-read primitive. So we defend in two layers, mirroring the C#,
  F#, Java, Kotlin, Perl, Haskell, Dart, Lua and Swift ports:

  1. **Syntactic check** (`safe_name?/1`). A cow name is a bare file stem, so
     anything that could mean "some other directory" is refused:

         /     path separator
         \\     path separator on Windows
         ..    parent-directory step
         :     Windows drive letter (`C:x`) / alternate data stream
         NUL   C APIs stop reading at it, so the OS would see a different
               name from the one we checked
         ""    the empty name

     An absolute path always contains `/`, `\\` or `:`, so it falls to the same
     rule.

  2. **Containment check** (`resolve/2`). Erlang has no `realpath`, so we do
     the next best thing: expand both the cows directory and the candidate to
     absolute, `..`-free form with `Path.expand/1` and require the candidate to
     sit inside the cows directory; and, because lexical expansion cannot see
     through symlinks, we refuse a candidate that *is* a symlink (checked with
     `File.lstat/1`, which inspects the link rather than its target). That
     closes the "symlink planted in the cows directory" escape that string
     inspection alone would miss.

  A name failing either layer is treated exactly like a cow that does not
  exist: we quietly draw `default.cow`, matching every other port and cowsay's
  long-standing "unknown cow -> default cow" behaviour. Nothing is URL-decoded:
  `%2F` is three literal characters to the OS, never a slash.
  """

  # Characters that let a "name" smuggle in a directory, a drive, or a
  # truncation point.
  @forbidden ["/", "\\", ":", <<0>>]

  @doc "True when `cow_name` is a bare file stem that cannot name another directory."
  @spec safe_name?(term()) :: boolean()
  def safe_name?(cow_name) when is_binary(cow_name) and cow_name != "" do
    not String.contains?(cow_name, "..") and not String.contains?(cow_name, @forbidden)
  end

  def safe_name?(_cow_name), do: false

  @doc """
  Map `cow_name` to a `.cow` file inside `cows_dir`, falling back to
  `cows_dir/default.cow` when the name is unsafe, the file is missing, it is a
  symlink, or it lies outside `cows_dir`.
  """
  @spec resolve(term(), Path.t()) :: Path.t()
  def resolve(cow_name, cows_dir) do
    default_path = Path.join(cows_dir, "default.cow")

    if safe_name?(cow_name) do
      root = Path.expand(cows_dir)
      candidate = Path.expand(Path.join(root, cow_name <> ".cow"))

      if inside?(candidate, root) and regular_file?(candidate) do
        candidate
      else
        default_path
      end
    else
      default_path
    end
  end

  # Path.expand has already removed every `.` and `..`, so a plain prefix test
  # (with a trailing separator, so `/cows-evil` is not "inside" `/cows`) is a
  # sound containment check.
  defp inside?(candidate, root) do
    String.starts_with?(candidate, String.trim_trailing(root, "/") <> "/")
  end

  # lstat looks at the directory entry itself: a symlink reports :symlink here
  # even when its target is a perfectly ordinary file somewhere else.
  defp regular_file?(path) do
    match?({:ok, %File.Stat{type: :regular}}, File.lstat(path))
  end
end
