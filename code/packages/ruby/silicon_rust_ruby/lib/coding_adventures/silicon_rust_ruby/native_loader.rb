# frozen_string_literal: true

# native_loader.rb — find and require the silicon_rust_ruby_native shared library
# =================================================================================
#
# The native extension lives in one of two places:
#
#   1. **Dev / from-source**:  The gem's Rakefile copies the built
#      libsilicon_rust_ruby_native.{so,dylib,dll} from the workspace
#      target/release/ into lib/coding_adventures/silicon_rust_ruby/ after
#      `rake compile`.
#
#   2. **Installed gem**:  Inside the gem's extension_dir, installed by
#      the extconf.rb Makefile.
#
# Either way, the .{so,bundle,dll} should sit next to this file by the time
# this file is required.

module CodingAdventures
  module SiliconRustRuby
    # @api private
    module NativeLoader
      module_function

      def load!
        candidate = File.expand_path("silicon_rust_ruby_native", __dir__)
        begin
          require candidate
        rescue LoadError => e
          raise LoadError, <<~MSG
            silicon_rust_ruby could not load its native extension.

            Tried to require: #{candidate}.{so,bundle,dll}

            Original error: #{e.message}

            To build it from source, from the silicon_rust_ruby gem root:

              bundle install
              bundle exec rake compile

            Or from the workspace root:

              cargo build -p silicon-rust-ruby-native --release
              # then cp libsilicon_rust_ruby_native.{dylib,so,dll}
              # into lib/coding_adventures/silicon_rust_ruby/silicon_rust_ruby_native.<ext>
          MSG
        end
      end
    end
  end
end

CodingAdventures::SiliconRustRuby::NativeLoader.load!
