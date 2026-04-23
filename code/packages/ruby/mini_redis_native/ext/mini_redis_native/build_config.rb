# frozen_string_literal: true

require "rbconfig"

module MiniRedisNativeBuildConfig
  module_function

  def cargo_build_command
    cargo_runner + ["build", "--release"] + rust_target_args
  end

  def cargo_clean_command
    cargo_runner + ["clean"]
  end

  def cargo_make_command
    (cargo_runner + ["build", "--release"] + rust_target_args).join(" ")
  end

  def target_release_dir(ext_dir)
    target = rust_target
    return File.join(ext_dir, "target", "release") if target.nil?

    File.join(ext_dir, "target", target, "release")
  end

  def rust_target_args
    target = rust_target
    target.nil? ? [] : ["--target", target]
  end

  def rust_target
    return ENV["MINI_REDIS_NATIVE_RUST_TARGET"] unless ENV["MINI_REDIS_NATIVE_RUST_TARGET"].to_s.empty?

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
