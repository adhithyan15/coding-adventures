# frozen_string_literal: true

# build_config.rb — shared build configuration for the silicon_rust_ruby_native
# Rust native extension.  Used by both extconf.rb (gem install path) and the
# gem's Rakefile (dev path).
#
# Mirrors matrix_rust_ruby_native's build_config.rb verbatim — the cargo
# invocation pattern and workspace-relative target discovery are identical;
# only the crate name changes.
#
# Why use the workspace target dir?
#   Building device-physics + mosfet-models + fab-process-simulation cold takes
#   ~30 s.  If each gem used its own private target dir, every `rake compile`
#   would pay that cost.  Sharing the workspace target/ directory amortises it.

require "rbconfig"

module SiliconRustRubyNativeBuildConfig
  module_function

  def cargo_build_command
    cargo_runner + ["build", "--release", "-p", "silicon-rust-ruby-native"] + rust_target_args
  end

  def cargo_clean_command
    cargo_runner + ["clean", "-p", "silicon-rust-ruby-native"]
  end

  def cargo_make_command
    cargo_build_command.join(" ")
  end

  # The directory where cargo places the built shared library.
  #
  # ext_dir layout:
  #   <workspace>/code/packages/ruby/silicon_rust_ruby/ext/silicon_rust_ruby_native/
  #
  # Rust workspace is at code/packages/rust/ and puts output in
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
    env_override = ENV["SILICON_RUST_RUBY_NATIVE_RUST_TARGET"]
    unless env_override.to_s.empty?
      unless env_override.match?(/\A[a-zA-Z0-9_.\-]+\z/)
        raise "SILICON_RUST_RUBY_NATIVE_RUST_TARGET contains invalid characters: #{env_override.inspect}"
      end
      return env_override
    end

    case host_os
    when /mingw|cygwin/
      case host_cpu
      when /x64|x86_64/ then "x86_64-pc-windows-gnu"
      when /i\d86|x86/  then "i686-pc-windows-gnu"
      end
    when /mswin|msvc/
      case host_cpu
      when /x64|x86_64/ then "x86_64-pc-windows-msvc"
      when /i\d86|x86/  then "i686-pc-windows-msvc"
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
