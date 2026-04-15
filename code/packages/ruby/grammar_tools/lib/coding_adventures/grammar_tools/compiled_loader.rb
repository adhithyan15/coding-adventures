# frozen_string_literal: true

module CodingAdventures
  module GrammarTools
    # Loads generated _grammar.rb files and caches the embedded grammar object.
    #
    # Generated grammar files currently export a top-level TOKEN_GRAMMAR or
    # PARSER_GRAMMAR constant. This loader requires the file once, captures the
    # exported object, removes the temporary top-level constant, and memoizes
    # the result by absolute path so later callers can reuse it safely.
    module CompiledLoader
      module_function

      @cache = {}
      @mutex = Mutex.new

      def load_token_grammar(path)
        load_compiled_grammar(path, :TOKEN_GRAMMAR)
      end

      def load_parser_grammar(path)
        load_compiled_grammar(path, :PARSER_GRAMMAR)
      end

      def clear_cache!
        @mutex.synchronize { @cache.clear }
      end

      def load_compiled_grammar(path, exported_const)
        absolute_path = File.expand_path(path)

        @mutex.synchronize do
          return @cache[absolute_path] if @cache.key?(absolute_path)

          # Use `load` so clearing this cache really resets the loader.
          load absolute_path

          unless Object.const_defined?(exported_const, false)
            raise NameError,
              "Compiled grammar #{absolute_path} did not define #{exported_const}"
          end

          grammar = Object.const_get(exported_const)
          Object.send(:remove_const, exported_const)

          @cache[absolute_path] = grammar
        end
      end
      private_class_method :load_compiled_grammar
    end

    def load_token_grammar(path)
      CompiledLoader.load_token_grammar(path)
    end

    def load_parser_grammar(path)
      CompiledLoader.load_parser_grammar(path)
    end

    def clear_compiled_grammar_cache!
      CompiledLoader.clear_cache!
    end

    module_function :load_token_grammar, :load_parser_grammar, :clear_compiled_grammar_cache!
  end
end
