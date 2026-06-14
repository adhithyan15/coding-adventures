# frozen_string_literal: true

require "coding_adventures_vm_core"

module CodingAdventures
  module JitCore
    class PureVmBackend
      def compile_callable(function, mod)
        lambda do |args|
          CodingAdventures::VmCore::VMCore.new(
            profiler_enabled: false,
            u8_wrap: function.return_type == "u8"
          ).execute(mod, fn: function.name, args: args)
        end
      end
    end
  end
end
