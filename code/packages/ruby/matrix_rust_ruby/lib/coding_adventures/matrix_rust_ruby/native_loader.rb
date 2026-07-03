# frozen_string_literal: true

# native_loader.rb — find and require the matrix_rust_ruby_native shared library
# =================================================================================
#
# The native extension can live in one of two places:
#
#   1. **Dev / from-source installs**:  Inside the workspace under
#      `code/packages/rust/matrix-rust-ruby-native/target/release/`.  This is
#      where `cargo build -p matrix-rust-ruby-native --release` puts the
#      output.  The gem's Rakefile copies it into
#      `lib/coding_adventures/matrix_rust_ruby/` so a plain `require` works.
#
#   2. **Installed gem**:  Inside the gem's extension_dir, which the
#      Rake::ExtensionTask plumbing handles automatically.
#
# Either way, by the time this file is required the .{so,bundle,dll} should
# already be sitting next to the version.rb file.  We just try `require` on
# the standard basename and surface a helpful error if it's missing.

module CodingAdventures
  module MatrixRustRuby
    # @api private
    module NativeLoader
      module_function

      # Try to load the native extension.  Raises a clear LoadError with
      # remediation steps if it's missing — by far the most common failure
      # mode for users (forgot to `rake compile`, installed the gem on an
      # unsupported platform, ...).
      def load!
        # The .so/.bundle/.dll lives next to this file once `rake compile`
        # has run (the Rakefile copies it from the workspace target dir).
        candidate = File.expand_path("matrix_rust_ruby_native", __dir__)

        # Ruby's `require` will pick the right extension (.so on Linux,
        # .bundle on macOS, .dll on Windows) automatically.
        begin
          require candidate
        rescue LoadError => e
          raise LoadError, <<~MSG
            matrix_rust_ruby could not load its native extension.

            Tried to require: #{candidate}.{so,bundle,dll}

            Original error: #{e.message}

            To build it from source, from the matrix_rust_ruby gem root:

              bundle install
              bundle exec rake compile

            Or from the workspace root:

              cargo build -p matrix-rust-ruby-native --release
              # then cp the resulting libmatrix_rust_ruby_native.{dylib,so,dll}
              # into lib/coding_adventures/matrix_rust_ruby/matrix_rust_ruby_native.<ext>
          MSG
        end
      end
    end
  end
end

CodingAdventures::MatrixRustRuby::NativeLoader.load!
