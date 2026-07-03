# frozen_string_literal: true

# build_config.rb — shared build configuration for the matrix_rust_ruby_native
# Rust native extension.  Pulled in by both extconf.rb (gem install path) and
# the gem's Rakefile (dev path).
#
# Mirrors conduit_native's build_config.rb almost verbatim — Windows linking
# discoverability is fiddly and one working incantation is plenty.  The key
# differences from conduit are:
#
#   1. Cargo invocation uses `-p matrix-rust-ruby-native` instead of building
#      the crate's own Cargo.toml in-place.  That's because the Rust crate
#      lives in the workspace at
#      code/packages/rust/matrix-rust-ruby-native/, NOT inside this
#      gem's ext/ directory.  We want the workspace's Cargo.lock and shared
#      target/ so we don't recompile the matrix-* dependency chain from
#      scratch every time we install this gem.
#
#   2. `target_release_dir` therefore points to the WORKSPACE target dir,
#      not the gem-local one.  See `workspace_root` below.
#
# Why share the workspace target dir?
#   - Building the matrix-cpu dependency chain takes ~1 minute cold.  If
#     this gem reused only its own target dir, every developer's edit-test
#     cycle would pay that cost.
#   - The workspace `cargo build -p matrix-rust-ruby-native` already does
#     the right thing — we just have to look in the workspace's target
#     dir for the output, not ours.

require "rbconfig"

module MatrixRustRubyNativeBuildConfig
  module_function

  # Build command — cargo discovers the workspace by walking up from CWD,
  # so we can run this from anywhere as long as we're inside the workspace.
  # We pass `-p matrix-rust-ruby-native` to scope the build to just our
  # crate (and its transitive deps).
  def cargo_build_command
    cargo_runner + ["build", "--release", "-p", "matrix-rust-ruby-native"] + rust_target_args
  end

  def cargo_clean_command
    cargo_runner + ["clean", "-p", "matrix-rust-ruby-native"]
  end

  def cargo_make_command
    cargo_build_command.join(" ")
  end

  # The directory containing the built libmatrix_rust_ruby_native.{so,dylib,dll}.
  # Walks up from ext_dir to the workspace root, then into target/release.
  #
  # ext_dir layout:
  #   <workspace>/code/packages/ruby/matrix_rust_ruby/ext/matrix_rust_ruby_native/
  #
  # Workspace root contains the top-level Cargo.toml for the Rust workspace
  # at code/packages/rust/Cargo.toml.  Cargo puts build output in
  # code/packages/rust/target/{release,<triple>/release}.
  def target_release_dir(ext_dir)
    rust_workspace = File.expand_path("../../../../rust", ext_dir)
    target = rust_target
    base = File.join(rust_workspace, "target")
    target.nil? ? File.join(base, "release") : File.join(base, target, "release")
  end

  def rust_target_args
    target = rust_target
    target.nil? ? [] : ["--target", target]
  end

  def rust_target
    env_override = ENV["MATRIX_RUST_RUBY_NATIVE_RUST_TARGET"]
    unless env_override.to_s.empty?
      # Defence-in-depth: the env override flows into both an argv element
      # (`--target <value>`) AND into the generated Makefile body (where
      # shell metacharacters or newlines could inject extra rules).  In
      # practice the env-var setter already has local code execution via
      # the surrounding `gem install` / `cargo build` flow, so this isn't
      # a privilege boundary today — but allowlisting a sane character set
      # for Rust target triples (alphanumerics, underscore, dot, hyphen)
      # is free and stops the pattern from becoming a real vuln if the env
      # var is ever sourced from less-trusted config.
      unless env_override.match?(/\A[a-zA-Z0-9_.\-]+\z/)
        raise "MATRIX_RUST_RUBY_NATIVE_RUST_TARGET contains invalid characters: #{env_override.inspect}"
      end
      return env_override
    end

    case host_os
    when /mingw|cygwin/
      case host_cpu
      when /x64|x86_64/ then "x86_64-pc-windows-gnu"
      when /i\d86|x86/ then "i686-pc-windows-gnu"
      end
    when /mswin|msvc/
      case host_cpu
      when /x64|x86_64/ then "x86_64-pc-windows-msvc"
      when /i\d86|x86/ then "i686-pc-windows-msvc"
      end
    end
  end

  def cargo_runner
    if ruby_mingw? && ridk_available?
      ["ridk", "exec", "cargo"]
    else
      ["cargo"]
    end
  end

  def ruby_mingw?
    host_os.match?(/mingw|cygwin/)
  end

  def host_os
    RbConfig::CONFIG["host_os"].to_s
  end

  def host_cpu
    RbConfig::CONFIG["host_cpu"].to_s
  end

  def ridk_available?
    @ridk_available = system("ridk", "version", out: File::NULL, err: File::NULL) if @ridk_available.nil?
    @ridk_available
  end
end
