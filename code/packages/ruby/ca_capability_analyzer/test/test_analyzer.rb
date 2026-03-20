# frozen_string_literal: true

require "test_helper"

# ============================================================================
# Tests for the Capability Analyzer
# ============================================================================
#
# These tests verify that the AST walker correctly detects capability usage
# in Ruby source code. Each test provides a snippet of Ruby code and checks
# that the analyzer produces the expected capability detections.
#
# The tests are organized by detection type:
# 1. Import detection (require statements)
# 2. Class method detection (File.read, Dir.glob, etc.)
# 3. Bare method detection (system, exec, etc.)
# 4. ENV access detection
# 5. Backtick execution detection
# 6. Net::HTTP detection
# 7. Pure code (no capabilities)
# ============================================================================

class TestAnalyzer < Minitest::Test
  # Helper: analyze a source string and return detected capabilities.
  def analyze(source, filename: "test.rb")
    analyzer = CA::CapabilityAnalyzer::Analyzer.new(filename)
    analyzer.analyze(source)
  end

  # ── Import Detection ─────────────────────────────────────────────

  def test_require_socket_detects_network
    caps = analyze('require "socket"')
    assert_equal 1, caps.length
    assert_equal "net", caps.first.category
    assert_equal "*", caps.first.action
    assert_equal "*", caps.first.target
  end

  def test_require_net_http_detects_network_connect
    caps = analyze('require "net/http"')
    assert_equal 1, caps.length
    assert_equal "net", caps.first.category
    assert_equal "connect", caps.first.action
  end

  def test_require_open3_detects_proc_exec
    caps = analyze('require "open3"')
    assert_equal 1, caps.length
    assert_equal "proc", caps.first.category
    assert_equal "exec", caps.first.action
  end

  def test_require_fileutils_detects_fs
    caps = analyze('require "fileutils"')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "*", caps.first.action
  end

  def test_require_tempfile_detects_fs_write
    caps = analyze('require "tempfile"')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "write", caps.first.action
  end

  def test_require_pathname_detects_fs
    caps = analyze('require "pathname"')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "*", caps.first.action
  end

  def test_require_etc_detects_env_read
    caps = analyze('require "etc"')
    assert_equal 1, caps.length
    assert_equal "env", caps.first.category
    assert_equal "read", caps.first.action
  end

  def test_require_fiddle_detects_ffi
    caps = analyze('require "fiddle"')
    assert_equal 1, caps.length
    assert_equal "ffi", caps.first.category
    assert_equal "*", caps.first.action
  end

  def test_require_unknown_library_no_detection
    caps = analyze('require "json"')
    assert_empty caps
  end

  def test_require_open_uri_detects_network
    caps = analyze('require "open-uri"')
    assert_equal 1, caps.length
    assert_equal "net", caps.first.category
    assert_equal "connect", caps.first.action
  end

  def test_require_ffi_gem_detects_ffi
    caps = analyze('require "ffi"')
    assert_equal 1, caps.length
    assert_equal "ffi", caps.first.category
  end

  def test_require_find_detects_fs_list
    caps = analyze('require "find"')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "list", caps.first.action
  end

  # ── File Class Detection ─────────────────────────────────────────

  def test_file_read_detects_fs_read_with_target
    caps = analyze('File.read("config.yml")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "read", caps.first.action
    assert_equal "config.yml", caps.first.target
  end

  def test_file_write_detects_fs_write_with_target
    caps = analyze('File.write("output.txt", data)')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "write", caps.first.action
    assert_equal "output.txt", caps.first.target
  end

  def test_file_delete_detects_fs_delete
    caps = analyze('File.delete("tmp.txt")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "delete", caps.first.action
    assert_equal "tmp.txt", caps.first.target
  end

  def test_file_open_detects_fs_read
    caps = analyze('File.open("data.txt")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "read", caps.first.action
  end

  def test_file_readlines_detects_fs_read
    caps = analyze('File.readlines("log.txt")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "read", caps.first.action
  end

  def test_file_exist_detects_fs_read
    caps = analyze('File.exist?("path")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "read", caps.first.action
  end

  def test_file_rename_detects_fs_write
    caps = analyze('File.rename("old", "new")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "write", caps.first.action
  end

  def test_file_symlink_detects_fs_create
    caps = analyze('File.symlink("src", "link")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "create", caps.first.action
  end

  def test_file_stat_detects_fs_read
    caps = analyze('File.stat("path")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "read", caps.first.action
  end

  def test_file_with_variable_target_uses_wildcard
    caps = analyze("File.read(path)")
    assert_equal 1, caps.length
    assert_equal "*", caps.first.target
  end

  # ── Dir Class Detection ──────────────────────────────────────────

  def test_dir_glob_detects_fs_list
    caps = analyze('Dir.glob("*.rb")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "list", caps.first.action
  end

  def test_dir_entries_detects_fs_list
    caps = analyze('Dir.entries(".")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "list", caps.first.action
  end

  def test_dir_mkdir_detects_fs_create
    caps = analyze('Dir.mkdir("new_dir")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "create", caps.first.action
  end

  def test_dir_rmdir_detects_fs_delete
    caps = analyze('Dir.rmdir("old_dir")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "delete", caps.first.action
  end

  def test_dir_home_detects_env_read
    caps = analyze("Dir.home")
    assert_equal 1, caps.length
    assert_equal "env", caps.first.category
    assert_equal "read", caps.first.action
  end

  # ── IO Class Detection ──────────────────────────────────────────

  def test_io_read_detects_fs_read
    caps = analyze('IO.read("data.bin")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "read", caps.first.action
  end

  def test_io_write_detects_fs_write
    caps = analyze('IO.write("out.bin", data)')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "write", caps.first.action
  end

  # ── ENV Detection ────────────────────────────────────────────────

  def test_env_subscript_detects_env_read_with_key
    caps = analyze('ENV["HOME"]')
    assert_equal 1, caps.length
    assert_equal "env", caps.first.category
    assert_equal "read", caps.first.action
    assert_equal "HOME", caps.first.target
  end

  def test_env_fetch_detects_env_read
    caps = analyze('ENV.fetch("PATH")')
    assert_equal 1, caps.length
    assert_equal "env", caps.first.category
    assert_equal "read", caps.first.action
  end

  def test_env_subscript_with_variable_uses_wildcard
    caps = analyze("ENV[key]")
    assert_equal 1, caps.length
    assert_equal "*", caps.first.target
  end

  def test_env_keys_detects_env_read
    caps = analyze("ENV.keys")
    assert_equal 1, caps.length
    assert_equal "env", caps.first.category
    assert_equal "read", caps.first.action
  end

  # ── FileUtils Detection ──────────────────────────────────────────

  def test_fileutils_rm_detects_fs_delete
    caps = analyze('FileUtils.rm("file.txt")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "delete", caps.first.action
  end

  def test_fileutils_cp_detects_fs_write
    caps = analyze('FileUtils.cp("src", "dst")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "write", caps.first.action
  end

  def test_fileutils_mkdir_p_detects_fs_create
    caps = analyze('FileUtils.mkdir_p("deep/path")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "create", caps.first.action
  end

  def test_fileutils_rm_rf_detects_fs_delete
    caps = analyze('FileUtils.rm_rf("dir")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "delete", caps.first.action
  end

  def test_fileutils_touch_detects_fs_write
    caps = analyze('FileUtils.touch("file")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "write", caps.first.action
  end

  # ── Network Class Detection ──────────────────────────────────────

  def test_tcp_socket_new_detects_net_connect
    caps = analyze('TCPSocket.new("example.com", 80)')
    assert_equal 1, caps.length
    assert_equal "net", caps.first.category
    assert_equal "connect", caps.first.action
  end

  def test_udp_socket_new_detects_net_connect
    caps = analyze('UDPSocket.new("example.com")')
    assert_equal 1, caps.length
    assert_equal "net", caps.first.category
    assert_equal "connect", caps.first.action
  end

  def test_tcp_server_new_detects_net_listen
    caps = analyze("TCPServer.new(8080)")
    assert_equal 1, caps.length
    assert_equal "net", caps.first.category
    assert_equal "listen", caps.first.action
  end

  # ── Process Class Detection ──────────────────────────────────────

  def test_process_spawn_detects_proc_exec
    caps = analyze('Process.spawn("ls")')
    assert_equal 1, caps.length
    assert_equal "proc", caps.first.category
    assert_equal "exec", caps.first.action
  end

  def test_process_fork_detects_proc_fork
    caps = analyze("Process.fork { }")
    assert_equal 1, caps.length
    assert_equal "proc", caps.first.category
    assert_equal "fork", caps.first.action
  end

  def test_process_kill_detects_proc_signal
    caps = analyze("Process.kill(:TERM, pid)")
    assert_equal 1, caps.length
    assert_equal "proc", caps.first.category
    assert_equal "signal", caps.first.action
  end

  # ── Bare Method Detection ────────────────────────────────────────

  def test_system_detects_proc_exec
    caps = analyze('system("ls -la")')
    assert_equal 1, caps.length
    assert_equal "proc", caps.first.category
    assert_equal "exec", caps.first.action
    assert_equal "ls -la", caps.first.target
  end

  def test_exec_detects_proc_exec
    caps = analyze('exec("bash")')
    assert_equal 1, caps.length
    assert_equal "proc", caps.first.category
    assert_equal "exec", caps.first.action
  end

  def test_spawn_detects_proc_exec
    caps = analyze('spawn("background-job")')
    assert_equal 1, caps.length
    assert_equal "proc", caps.first.category
    assert_equal "exec", caps.first.action
  end

  def test_fork_detects_proc_fork
    caps = analyze("fork { work }")
    assert_equal 1, caps.length
    assert_equal "proc", caps.first.category
    assert_equal "fork", caps.first.action
  end

  def test_open_detects_fs_read
    caps = analyze('open("file.txt")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "read", caps.first.action
  end

  # ── Backtick Detection ──────────────────────────────────────────

  def test_backtick_detects_proc_exec
    caps = analyze('`ls -la`')
    assert_equal 1, caps.length
    assert_equal "proc", caps.first.category
    assert_equal "exec", caps.first.action
  end

  def test_interpolated_backtick_detects_proc_exec
    caps = analyze('`ls #{dir}`')
    assert_equal 1, caps.length
    assert_equal "proc", caps.first.category
    assert_equal "exec", caps.first.action
  end

  # ── Net::HTTP Detection ──────────────────────────────────────────

  def test_net_http_get_detects_net_connect
    caps = analyze("Net::HTTP.get(uri)")
    assert_equal 1, caps.length
    assert_equal "net", caps.first.category
    assert_equal "connect", caps.first.action
  end

  def test_net_http_post_detects_net_connect
    caps = analyze("Net::HTTP.post(uri, data)")
    assert_equal 1, caps.length
    assert_equal "net", caps.first.category
    assert_equal "connect", caps.first.action
  end

  def test_net_http_start_detects_net_connect
    caps = analyze('Net::HTTP.start("example.com") { |http| }')
    assert_equal 1, caps.length
    assert_equal "net", caps.first.category
    assert_equal "connect", caps.first.action
  end

  # ── Pure Code (No Capabilities) ─────────────────────────────────

  def test_pure_arithmetic_no_detection
    caps = analyze("x = 1 + 2 * 3")
    assert_empty caps
  end

  def test_pure_string_operations_no_detection
    caps = analyze('"hello".upcase.reverse')
    assert_empty caps
  end

  def test_pure_array_operations_no_detection
    caps = analyze("[1, 2, 3].map { |x| x * 2 }")
    assert_empty caps
  end

  def test_class_definition_no_detection
    source = <<~RUBY
      class MyClass
        def initialize(name)
          @name = name
        end

        def greet
          "Hello, \#{@name}!"
        end
      end
    RUBY
    caps = analyze(source)
    assert_empty caps
  end

  def test_require_safe_library_no_detection
    caps = analyze('require "json"')
    assert_empty caps
  end

  # ── Line Number Tracking ─────────────────────────────────────────

  def test_line_numbers_are_correct
    source = <<~RUBY
      x = 1
      y = 2
      File.read("config.yml")
    RUBY
    caps = analyze(source)
    assert_equal 1, caps.length
    assert_equal 3, caps.first.line
  end

  def test_filename_is_recorded
    caps = analyze('File.read("x")', filename: "app/models/user.rb")
    assert_equal "app/models/user.rb", caps.first.file
  end

  # ── Multiple Detections ──────────────────────────────────────────

  def test_multiple_capabilities_detected
    source = <<~RUBY
      require "socket"
      File.read("config.yml")
      ENV["DATABASE_URL"]
      system("migrate")
    RUBY
    caps = analyze(source)
    assert_equal 4, caps.length

    categories = caps.map(&:category)
    assert_includes categories, "net"
    assert_includes categories, "fs"
    assert_includes categories, "env"
    assert_includes categories, "proc"
  end

  # ── Evidence Strings ─────────────────────────────────────────────

  def test_evidence_for_require
    caps = analyze('require "socket"')
    assert_equal 'require "socket"', caps.first.evidence
  end

  def test_evidence_for_class_method
    caps = analyze('File.read("config.yml")')
    assert_includes caps.first.evidence, "File.read"
  end

  def test_evidence_for_bare_method
    caps = analyze('system("ls")')
    assert_includes caps.first.evidence, "system"
  end

  # ── to_s format ──────────────────────────────────────────────────

  def test_detected_capability_to_s
    cap = CA::CapabilityAnalyzer::DetectedCapability.new(
      category: "fs", action: "read", target: "file.txt",
      file: "test.rb", line: 1, evidence: 'File.read("file.txt")'
    )
    assert_equal "fs:read:file.txt", cap.to_s
  end

  def test_detected_capability_to_h
    cap = CA::CapabilityAnalyzer::DetectedCapability.new(
      category: "fs", action: "read", target: "file.txt",
      file: "test.rb", line: 1, evidence: 'File.read("file.txt")'
    )
    h = cap.to_h
    assert_equal "fs", h[:category]
    assert_equal "read", h[:action]
    assert_equal "file.txt", h[:target]
  end

  # ── Dir children/each_child Detection ────────────────────────────

  def test_dir_children_detects_fs_list
    caps = analyze('Dir.children(".")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "list", caps.first.action
  end

  # ── Socket class Detection ──────────────────────────────────────

  def test_socket_tcp_detects_net_connect
    caps = analyze('Socket.tcp("example.com", 80)')
    assert_equal 1, caps.length
    assert_equal "net", caps.first.category
    assert_equal "connect", caps.first.action
  end

  # ── File binread/binwrite Detection ─────────────────────────────

  def test_file_binread_detects_fs_read
    caps = analyze('File.binread("image.png")')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "read", caps.first.action
  end

  def test_file_binwrite_detects_fs_write
    caps = analyze('File.binwrite("output.bin", data)')
    assert_equal 1, caps.length
    assert_equal "fs", caps.first.category
    assert_equal "write", caps.first.action
  end

  # ── Net::HTTP new Detection ─────────────────────────────────────

  def test_net_http_new_detects_net_connect
    caps = analyze('Net::HTTP.new("example.com")')
    assert_equal 1, caps.length
    assert_equal "net", caps.first.category
    assert_equal "connect", caps.first.action
  end
end
