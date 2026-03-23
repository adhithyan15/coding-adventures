defmodule ChmodTest do
  @moduledoc """
  Tests for the chmod tool.

  ## What These Tests Verify

  These tests cover:
  1. CLI Builder integration (parsing flags, --help, --version).
  2. Mode parsing (octal and symbolic notation).
  3. Symbolic mode parsing (who, operation, permissions).
  4. Mode application (add, remove, set for different who categories).
  5. Bitmask computation.
  6. File permission changes using real files.
  7. Recursive permission changes.
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.CliBuilder.Parser
  alias CodingAdventures.CliBuilder.{ParseResult, HelpResult, VersionResult}

  import Bitwise

  # ---------------------------------------------------------------------------
  # Helper: locate the spec file
  # ---------------------------------------------------------------------------

  @spec_path Path.join([__DIR__, "..", "chmod.json"]) |> Path.expand()

  defp parse_argv(argv) do
    Parser.parse(@spec_path, argv)
  end

  # ---------------------------------------------------------------------------
  # Test: CLI parsing
  # ---------------------------------------------------------------------------

  describe "CLI parsing" do
    test "mode and file arguments are captured" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["chmod", "755", "file.txt"])
      assert arguments["mode"] == "755"
      # Variadic args always return as list
      assert arguments["files"] == ["file.txt"]
    end

    test "multiple files" do
      {:ok, %ParseResult{arguments: arguments}} = parse_argv(["chmod", "644", "f1", "f2"])
      assert arguments["files"] == ["f1", "f2"]
    end

    test "-R sets recursive" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["chmod", "-R", "755", "dir"])
      assert flags["recursive"] == true
    end

    test "-v sets verbose" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["chmod", "-v", "755", "file"])
      assert flags["verbose"] == true
    end

    test "-c sets changes" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["chmod", "-c", "755", "file"])
      assert flags["changes"] == true
    end

    test "-f sets silent" do
      {:ok, %ParseResult{flags: flags}} = parse_argv(["chmod", "-f", "755", "file"])
      assert flags["silent"] == true
    end

    test "--help returns help text" do
      {:ok, %HelpResult{text: text}} = parse_argv(["chmod", "--help"])
      assert text =~ "chmod"
    end

    test "--version returns version" do
      {:ok, %VersionResult{version: version}} = parse_argv(["chmod", "--version"])
      assert version =~ "1.0.0"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_mode — octal
  # ---------------------------------------------------------------------------

  describe "parse_mode octal" do
    test "three-digit octal" do
      assert {:octal, 0o755} = UnixTools.Chmod.parse_mode("755")
    end

    test "four-digit octal with leading zero" do
      assert {:octal, 0o644} = UnixTools.Chmod.parse_mode("0644")
    end

    test "minimal permissions" do
      assert {:octal, 0o000} = UnixTools.Chmod.parse_mode("000")
    end

    test "maximum standard permissions" do
      assert {:octal, 0o777} = UnixTools.Chmod.parse_mode("777")
    end

    test "setuid bit" do
      assert {:octal, 0o4755} = UnixTools.Chmod.parse_mode("4755")
    end

    test "sticky bit" do
      assert {:octal, 0o1777} = UnixTools.Chmod.parse_mode("1777")
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_mode — symbolic
  # ---------------------------------------------------------------------------

  describe "parse_mode symbolic" do
    test "simple symbolic mode" do
      assert {:symbolic, _ops} = UnixTools.Chmod.parse_mode("u+x")
    end

    test "all users symbolic mode" do
      assert {:symbolic, _ops} = UnixTools.Chmod.parse_mode("a+r")
    end

    test "comma-separated symbolic modes" do
      {:symbolic, ops} = UnixTools.Chmod.parse_mode("u+x,g+r")
      assert length(ops) == 2
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_symbolic
  # ---------------------------------------------------------------------------

  describe "parse_symbolic" do
    test "user add execute" do
      [op] = UnixTools.Chmod.parse_symbolic("u+x")
      assert op.who == [:user]
      assert op.op == :add
      assert op.perms == [:execute]
    end

    test "group remove write" do
      [op] = UnixTools.Chmod.parse_symbolic("g-w")
      assert op.who == [:group]
      assert op.op == :remove
      assert op.perms == [:write]
    end

    test "other set read" do
      [op] = UnixTools.Chmod.parse_symbolic("o=r")
      assert op.who == [:other]
      assert op.op == :set
      assert op.perms == [:read]
    end

    test "all add read-write" do
      [op] = UnixTools.Chmod.parse_symbolic("a+rw")
      assert op.who == [:user, :group, :other]
      assert op.op == :add
      assert :read in op.perms
      assert :write in op.perms
    end

    test "implicit all (no who specified)" do
      [op] = UnixTools.Chmod.parse_symbolic("+x")
      assert op.who == [:user, :group, :other]
      assert op.op == :add
      assert op.perms == [:execute]
    end

    test "multiple who characters" do
      [op] = UnixTools.Chmod.parse_symbolic("ug+r")
      assert :user in op.who
      assert :group in op.who
      refute :other in op.who
    end

    test "multiple permissions" do
      [op] = UnixTools.Chmod.parse_symbolic("u+rwx")
      assert :read in op.perms
      assert :write in op.perms
      assert :execute in op.perms
    end

    test "comma-separated clauses" do
      ops = UnixTools.Chmod.parse_symbolic("u+x,g-w,o=r")
      assert length(ops) == 3

      [op1, op2, op3] = ops
      assert op1.who == [:user]
      assert op1.op == :add
      assert op2.who == [:group]
      assert op2.op == :remove
      assert op3.who == [:other]
      assert op3.op == :set
    end
  end

  # ---------------------------------------------------------------------------
  # Test: apply_mode — octal
  # ---------------------------------------------------------------------------

  describe "apply_mode octal" do
    test "octal replaces entire mode" do
      assert UnixTools.Chmod.apply_mode({:octal, 0o755}, 0o644) == 0o755
    end

    test "octal with zero" do
      assert UnixTools.Chmod.apply_mode({:octal, 0o000}, 0o777) == 0o000
    end

    test "octal preserves special bits" do
      assert UnixTools.Chmod.apply_mode({:octal, 0o4755}, 0o644) == 0o4755
    end
  end

  # ---------------------------------------------------------------------------
  # Test: apply_mode — symbolic add
  # ---------------------------------------------------------------------------

  describe "apply_mode symbolic add" do
    test "add execute for user" do
      ops = [%{who: [:user], op: :add, perms: [:execute]}]
      result = UnixTools.Chmod.apply_mode({:symbolic, ops}, 0o644)
      assert result == 0o744
    end

    test "add read for group" do
      ops = [%{who: [:group], op: :add, perms: [:read]}]
      result = UnixTools.Chmod.apply_mode({:symbolic, ops}, 0o700)
      assert result == 0o740
    end

    test "add write for other" do
      ops = [%{who: [:other], op: :add, perms: [:write]}]
      result = UnixTools.Chmod.apply_mode({:symbolic, ops}, 0o750)
      assert result == 0o752
    end

    test "add multiple permissions" do
      ops = [%{who: [:user], op: :add, perms: [:read, :write, :execute]}]
      result = UnixTools.Chmod.apply_mode({:symbolic, ops}, 0o000)
      assert result == 0o700
    end

    test "add to all" do
      ops = [%{who: [:user, :group, :other], op: :add, perms: [:execute]}]
      result = UnixTools.Chmod.apply_mode({:symbolic, ops}, 0o644)
      assert result == 0o755
    end

    test "adding already-set bit is idempotent" do
      ops = [%{who: [:user], op: :add, perms: [:read]}]
      result = UnixTools.Chmod.apply_mode({:symbolic, ops}, 0o644)
      assert result == 0o644
    end
  end

  # ---------------------------------------------------------------------------
  # Test: apply_mode — symbolic remove
  # ---------------------------------------------------------------------------

  describe "apply_mode symbolic remove" do
    test "remove write for group" do
      ops = [%{who: [:group], op: :remove, perms: [:write]}]
      result = UnixTools.Chmod.apply_mode({:symbolic, ops}, 0o664)
      assert result == 0o644
    end

    test "remove execute for all" do
      ops = [%{who: [:user, :group, :other], op: :remove, perms: [:execute]}]
      result = UnixTools.Chmod.apply_mode({:symbolic, ops}, 0o755)
      assert result == 0o644
    end

    test "removing unset bit is idempotent" do
      ops = [%{who: [:other], op: :remove, perms: [:execute]}]
      result = UnixTools.Chmod.apply_mode({:symbolic, ops}, 0o644)
      assert result == 0o644
    end
  end

  # ---------------------------------------------------------------------------
  # Test: apply_mode — symbolic set
  # ---------------------------------------------------------------------------

  describe "apply_mode symbolic set" do
    test "set exactly replaces who's permissions" do
      ops = [%{who: [:user], op: :set, perms: [:read]}]
      result = UnixTools.Chmod.apply_mode({:symbolic, ops}, 0o755)
      # User bits cleared then set to read only: 4xx -> 4xx, but x removed
      assert result == 0o455
    end

    test "set to empty removes all for who" do
      ops = [%{who: [:other], op: :set, perms: []}]
      result = UnixTools.Chmod.apply_mode({:symbolic, ops}, 0o777)
      assert result == 0o770
    end
  end

  # ---------------------------------------------------------------------------
  # Test: apply_mode — multiple operations
  # ---------------------------------------------------------------------------

  describe "apply_mode multiple operations" do
    test "add execute for user, remove write for other" do
      ops = [
        %{who: [:user], op: :add, perms: [:execute]},
        %{who: [:other], op: :remove, perms: [:write]}
      ]
      result = UnixTools.Chmod.apply_mode({:symbolic, ops}, 0o646)
      assert result == 0o744
    end
  end

  # ---------------------------------------------------------------------------
  # Test: compute_mask
  # ---------------------------------------------------------------------------

  describe "compute_mask" do
    test "user read+write" do
      assert UnixTools.Chmod.compute_mask([:user], [:read, :write]) == 0o600
    end

    test "group execute" do
      assert UnixTools.Chmod.compute_mask([:group], [:execute]) == 0o010
    end

    test "other all permissions" do
      assert UnixTools.Chmod.compute_mask([:other], [:read, :write, :execute]) == 0o007
    end

    test "all users read" do
      mask = UnixTools.Chmod.compute_mask([:user, :group, :other], [:read])
      assert mask == 0o444
    end
  end

  # ---------------------------------------------------------------------------
  # Test: permission_bit
  # ---------------------------------------------------------------------------

  describe "permission_bit" do
    test "user read" do
      assert UnixTools.Chmod.permission_bit(:user, :read) == 0o400
    end

    test "user write" do
      assert UnixTools.Chmod.permission_bit(:user, :write) == 0o200
    end

    test "user execute" do
      assert UnixTools.Chmod.permission_bit(:user, :execute) == 0o100
    end

    test "group read" do
      assert UnixTools.Chmod.permission_bit(:group, :read) == 0o040
    end

    test "other execute" do
      assert UnixTools.Chmod.permission_bit(:other, :execute) == 0o001
    end
  end

  # ---------------------------------------------------------------------------
  # Test: format_octal_mode
  # ---------------------------------------------------------------------------

  describe "format_octal_mode" do
    test "standard permissions" do
      assert UnixTools.Chmod.format_octal_mode(0o755) == "0755"
    end

    test "minimal permissions" do
      assert UnixTools.Chmod.format_octal_mode(0o000) == "0000"
    end

    test "with setuid" do
      assert UnixTools.Chmod.format_octal_mode(0o4755) == "4755"
    end
  end

  # ---------------------------------------------------------------------------
  # Test: parse_mode integration (end-to-end from string to bits)
  # ---------------------------------------------------------------------------

  describe "parse_mode + apply_mode integration" do
    test "octal 755 from string" do
      mode_spec = UnixTools.Chmod.parse_mode("755")
      result = UnixTools.Chmod.apply_mode(mode_spec, 0o000)
      assert result == 0o755
    end

    test "symbolic u+x from string" do
      mode_spec = UnixTools.Chmod.parse_mode("u+x")
      result = UnixTools.Chmod.apply_mode(mode_spec, 0o644)
      assert result == 0o744
    end

    test "symbolic a=rw from string" do
      mode_spec = UnixTools.Chmod.parse_mode("a=rw")
      result = UnixTools.Chmod.apply_mode(mode_spec, 0o777)
      assert result == 0o666
    end

    test "symbolic go-w from string" do
      mode_spec = UnixTools.Chmod.parse_mode("go-w")
      result = UnixTools.Chmod.apply_mode(mode_spec, 0o666)
      assert result == 0o644
    end

    test "comma-separated u+x,g+r from string" do
      mode_spec = UnixTools.Chmod.parse_mode("u+x,g+r")
      result = UnixTools.Chmod.apply_mode(mode_spec, 0o600)
      assert result == 0o740
    end

    test "setuid via symbolic" do
      ops = [%{who: [:user], op: :add, perms: [:setuid]}]
      result = UnixTools.Chmod.apply_mode({:symbolic, ops}, 0o755)
      assert (result &&& 0o4000) == 0o4000
    end

    test "sticky bit via symbolic" do
      ops = [%{who: [:other], op: :add, perms: [:sticky]}]
      result = UnixTools.Chmod.apply_mode({:symbolic, ops}, 0o755)
      assert (result &&& 0o1000) == 0o1000
    end
  end

  # ---------------------------------------------------------------------------
  # Test: File operations
  # ---------------------------------------------------------------------------

  # -------------------------------------------------------------------------
  # File operations tests — Unix only
  #
  # These tests verify actual file permission changes via File.chmod!/2.
  # Windows does not support Unix permission bits (owner/group/other rwx),
  # so these tests only run on Unix systems.
  # -------------------------------------------------------------------------

  if :os.type() != {:win32, :nt} do
    describe "file operations" do
      @tag :tmp_dir
      test "change file to 755", %{tmp_dir: tmp} do
        path = Path.join(tmp, "test.sh")
        File.write!(path, "#!/bin/sh\necho hello")
        File.chmod!(path, 0o644)

        mode_spec = UnixTools.Chmod.parse_mode("755")
        new_mode = UnixTools.Chmod.apply_mode(mode_spec, 0o644)
        File.chmod!(path, new_mode)

        %{mode: file_mode} = File.stat!(path)
        assert (file_mode &&& 0o777) == 0o755
      end

      @tag :tmp_dir
      test "add execute permission with symbolic mode", %{tmp_dir: tmp} do
        path = Path.join(tmp, "script.sh")
        File.write!(path, "#!/bin/sh")
        File.chmod!(path, 0o644)

        mode_spec = UnixTools.Chmod.parse_mode("u+x")
        current_mode = File.stat!(path).mode &&& 0o7777
        new_mode = UnixTools.Chmod.apply_mode(mode_spec, current_mode)
        File.chmod!(path, new_mode)

        %{mode: file_mode} = File.stat!(path)
        assert (file_mode &&& 0o100) == 0o100
      end

      @tag :tmp_dir
      test "remove write from group and other", %{tmp_dir: tmp} do
        path = Path.join(tmp, "readonly.txt")
        File.write!(path, "content")
        File.chmod!(path, 0o666)

        mode_spec = UnixTools.Chmod.parse_mode("go-w")
        current_mode = File.stat!(path).mode &&& 0o7777
        new_mode = UnixTools.Chmod.apply_mode(mode_spec, current_mode)
        File.chmod!(path, new_mode)

        %{mode: file_mode} = File.stat!(path)
        assert (file_mode &&& 0o022) == 0o000
      end

      @tag :tmp_dir
      test "recursive chmod on directory", %{tmp_dir: tmp} do
        dir = Path.join(tmp, "subdir")
        File.mkdir_p!(dir)
        file1 = Path.join(dir, "file1.txt")
        file2 = Path.join(dir, "file2.txt")
        File.write!(file1, "content1")
        File.write!(file2, "content2")
        File.chmod!(file1, 0o644)
        File.chmod!(file2, 0o644)

        # Apply u+x to both files via symbolic mode
        mode_spec = UnixTools.Chmod.parse_mode("u+x")

        [file1, file2]
        |> Enum.each(fn file_path ->
          current_mode = File.stat!(file_path).mode &&& 0o7777
          new_mode = UnixTools.Chmod.apply_mode(mode_spec, current_mode)
          File.chmod!(file_path, new_mode)
        end)

        assert (File.stat!(file1).mode &&& 0o100) == 0o100
        assert (File.stat!(file2).mode &&& 0o100) == 0o100
      end
    end
  end
end
