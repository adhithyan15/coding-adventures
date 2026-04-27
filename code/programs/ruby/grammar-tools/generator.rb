# frozen_string_literal: true

require "fileutils"
require "open3"
require "pathname"
require "tmpdir"

module GrammarToolsProgram
  class CompiledGrammarGenerator
    ECMASCRIPT_VERSIONS = %w[
      es1 es3 es5
      es2015 es2016 es2017 es2018 es2019 es2020
      es2021 es2022 es2023 es2024 es2025
    ].freeze

    TYPESCRIPT_VERSIONS = %w[ts1.0 ts2.0 ts3.0 ts4.0 ts5.0 ts5.8].freeze
    PYTHON_VERSIONS = %w[2.7 3.0 3.6 3.8 3.10 3.12].freeze
    JAVA_VERSIONS = %w[1.0 1.1 1.4 5 7 8 10 14 17 21].freeze

    def initialize(root)
      @root = Pathname(root).expand_path
      @grammars = @root.join("code", "grammars")
      @packages = @root.join("code", "packages")
      @programs = @root.join("code", "programs")
      @failures = 0
      @go_binary = nil
      @rs_binary = @programs.join("rust", "grammar-tools", "target", "release", "grammar-tools")
      @py_prog = @programs.join("python", "grammar-tools", "main.py")
      @ts_dir = @programs.join("typescript", "grammar-tools")
      @ts_vite = @ts_dir.join("node_modules", ".bin", "vite-node")
      @ts_vite_cfg = @ts_dir.join("vitest.config.ts")
      @ts_wrapper = @root.join("scripts", "_ts_grammar_compile.ts")
    end

    def run
      Dir.mktmpdir("compiled-grammars") do |tmp_dir|
        build_toolchains(Pathname(tmp_dir))

        puts
        puts "=== Generating _grammar files ==="
        puts

        emit_generic_sections
        emit_ruby_versioned_sections
      end

      report
    rescue StandardError => e
      warn "ERROR: #{e.message}"
      1
    end

    private

    def build_toolchains(tmp_dir)
      puts "=== Building grammar-tools binaries ==="

      @go_binary = tmp_dir.join("go-grammar-tools")
      run_required("Go", "go", "build", "-o", @go_binary.to_s, ".", chdir: @programs.join("go", "grammar-tools"))
      run_required("Rust", "cargo", "build", "--release", "--quiet", chdir: @programs.join("rust", "grammar-tools"))
    end

    def emit_generic_sections
      section("css") do
        compile_python_tokens("css")
        compile_python_grammar("css")
        compile_rust_tokens("css")
        compile_rust_grammar("css")
      end

      section("excel") do
        compile_python_tokens("excel")
        compile_python_grammar("excel")
        compile_go_tokens("excel", force: true)
        compile_go_grammar("excel", force: true)
        compile_ruby_tokens("excel")
        compile_ruby_grammar("excel")
        compile_typescript_tokens("excel")
        compile_typescript_grammar("excel")
        compile_rust_tokens("excel", tokens_name: "excel", force: true)
        compile_rust_grammar("excel", force: true)
      end

      section("javascript") do
        compile_python_tokens("javascript")
        compile_python_grammar("javascript")
        compile_go_tokens("javascript")
        compile_go_grammar("javascript")
        compile_ruby_tokens("javascript")
        compile_ruby_grammar("javascript")
        compile_typescript_tokens("javascript")
        compile_typescript_grammar("javascript")
        compile_rust_tokens("javascript")
        compile_rust_grammar("javascript")
      end

      section("json") do
        compile_python_tokens("json")
        compile_python_grammar("json")
        compile_go_tokens("json")
        compile_go_grammar("json")
        compile_ruby_tokens("json")
        compile_ruby_grammar("json")
        compile_typescript_tokens("json")
        compile_typescript_grammar("json")
        compile_rust_tokens("json")
        compile_rust_grammar("json")
      end

      section("lattice") do
        compile_python_tokens("lattice")
        compile_python_grammar("lattice")
        compile_go_tokens("lattice")
        compile_go_grammar("lattice")
        compile_ruby_tokens("lattice")
        compile_ruby_grammar("lattice")
        compile_typescript_tokens("lattice")
        compile_typescript_grammar("lattice")
        compile_rust_tokens("lattice")
        compile_rust_grammar("lattice")
      end

      section("lisp") do
        compile_python_tokens("lisp")
        compile_python_grammar("lisp")
        compile_rust_tokens("lisp")
        compile_rust_grammar("lisp")
      end

      section("python") do
        compile_python_tokens("python")
        compile_python_grammar("python")
        compile_go_tokens("python")
        compile_go_grammar("python")
        compile_typescript_tokens("python")
        compile_typescript_grammar("python")
      end

      section("ruby") do
        compile_python_tokens("ruby")
        compile_python_grammar("ruby")
        compile_go_tokens("ruby")
        compile_go_grammar("ruby")
        compile_ruby_tokens("ruby")
        compile_ruby_grammar("ruby")
        compile_typescript_tokens("ruby")
        compile_typescript_grammar("ruby")
        compile_rust_tokens("ruby")
        compile_rust_grammar("ruby")
      end

      section("sql") do
        compile_python_tokens("sql")
        compile_python_grammar("sql")
        compile_go_tokens("sql")
        compile_go_grammar("sql")
        compile_ruby_tokens("sql")
        compile_ruby_grammar("sql")
        compile_typescript_tokens("sql")
        compile_typescript_grammar("sql")
        compile_rust_tokens("sql")
        compile_rust_grammar("sql")
      end

      section("starlark") do
        compile_python_tokens("starlark")
        compile_python_grammar("starlark")
        compile_go_tokens("starlark")
        compile_go_grammar("starlark")
        compile_ruby_tokens("starlark")
        compile_ruby_grammar("starlark")
        compile_typescript_tokens("starlark")
        compile_typescript_grammar("starlark")
        compile_rust_tokens("starlark")
        compile_rust_grammar("starlark")
      end

      section("toml") do
        compile_python_tokens("toml")
        compile_python_grammar("toml")
        compile_go_tokens("toml")
        compile_go_grammar("toml")
        compile_ruby_tokens("toml")
        compile_ruby_grammar("toml")
        compile_typescript_tokens("toml")
        compile_typescript_grammar("toml")
        compile_rust_tokens("toml")
        compile_rust_grammar("toml")
      end

      section("typescript") do
        compile_python_tokens("typescript")
        compile_python_grammar("typescript")
        compile_go_tokens("typescript")
        compile_go_grammar("typescript")
        compile_ruby_tokens("typescript")
        compile_ruby_grammar("typescript")
        compile_typescript_tokens("typescript")
        compile_typescript_grammar("typescript")
        compile_rust_tokens("typescript")
        compile_rust_grammar("typescript")
      end

      section("verilog") do
        compile_python_tokens("verilog")
        compile_python_grammar("verilog")
        compile_go_tokens("verilog")
        compile_go_grammar("verilog")
        compile_ruby_tokens("verilog")
        compile_ruby_grammar("verilog")
        compile_typescript_tokens("verilog")
        compile_typescript_grammar("verilog")
        compile_rust_tokens("verilog")
        compile_rust_grammar("verilog")
      end

      section("vhdl") do
        compile_python_tokens("vhdl")
        compile_python_grammar("vhdl")
        compile_go_tokens("vhdl")
        compile_go_grammar("vhdl")
        compile_ruby_tokens("vhdl")
        compile_ruby_grammar("vhdl")
        compile_typescript_tokens("vhdl")
        compile_typescript_grammar("vhdl")
        compile_rust_tokens("vhdl")
        compile_rust_grammar("vhdl")
      end

      section("xml") do
        compile_python_tokens("xml")
        compile_go_tokens("xml", force: true)
        compile_ruby_tokens("xml")
        compile_typescript_tokens("xml")
        compile_rust_tokens("xml", tokens_name: "xml_rust")
      end
    end

    def emit_ruby_versioned_sections
      section("javascript versioned (ruby)") do
        compile_ruby_ecmascript_versions("javascript_lexer", "javascript_parser")
      end

      section("typescript versioned (ruby)") do
        compile_ruby_typescript_versions("typescript_lexer", "typescript_parser")
      end

      section("java versioned (ruby)") do
        compile_ruby_java_versions("java_lexer", "java_parser")
      end

      section("python versioned (ruby)") do
        compile_ruby_python_versions("python_lexer", "python_parser")
      end
    end

    def section(name)
      puts "#{name}:"
      yield
      puts
    end

    def report
      puts "==================================="
      if @failures.zero?
        puts "All _grammar files generated successfully."
        0
      else
        puts "FAILURES: #{@failures} compile command(s) failed."
        1
      end
    end

    def run_required(label, *cmd, chdir:)
      print "  #{label} ... "
      stdout, stderr, status = Open3.capture3(*cmd, chdir: chdir.to_s)
      raise stderr unless status.success?
      puts "OK"
      warn stdout unless stdout.empty?
    end

    def run_compile(description, *cmd, chdir: nil, env: nil)
      puts "  #{description}"
      stdout, stderr, status =
        if env && chdir
          Open3.capture3(env, *cmd, chdir: chdir.to_s)
        elsif env
          Open3.capture3(env, *cmd)
        elsif chdir
          Open3.capture3(*cmd, chdir: chdir.to_s)
        else
          Open3.capture3(*cmd)
        end
      return if status.success?

      warn stdout unless stdout.empty?
      warn stderr unless stderr.empty?
      warn "  FAILED: #{cmd.join(' ')}"
      @failures += 1
    end

    def compile_python_tokens(grammar)
      tokens = @grammars.join("#{grammar}.tokens")
      pkg_dir = "#{grammar.tr('-', '_')}_lexer"
      out_dir = @packages.join("python", "#{grammar}-lexer", "src", pkg_dir)
      return unless out_dir.directory?

      run_compile(
        "python: #{grammar}-lexer/_grammar.py",
        "mise", "exec", "--", "python", @py_prog.to_s, "compile-tokens", tokens.to_s,
        "-o", out_dir.join("_grammar.py").to_s,
        env: python_env
      )
    end

    def compile_python_grammar(grammar)
      grammar_file = @grammars.join("#{grammar}.grammar")
      pkg_dir = "#{grammar.tr('-', '_')}_parser"
      out_dir = @packages.join("python", "#{grammar}-parser", "src", pkg_dir)
      return unless out_dir.directory?

      run_compile(
        "python: #{grammar}-parser/_grammar.py",
        "mise", "exec", "--", "python", @py_prog.to_s, "compile-grammar", grammar_file.to_s,
        "-o", out_dir.join("_grammar.py").to_s,
        env: python_env
      )
    end

    def compile_go_tokens(grammar, force: false)
      tokens = @grammars.join("#{grammar}.tokens")
      pkg_dir = "#{grammar}-lexer"
      out_dir = @packages.join("go", pkg_dir)
      return unless out_dir.directory?

      cmd = [@go_binary.to_s, "compile-tokens", tokens.to_s, "-o", out_dir.join("_grammar.go").to_s, "-p", go_pkg_name(pkg_dir)]
      cmd << "--force" if force
      run_compile("go: #{grammar}-lexer/_grammar.go", *cmd)
    end

    def compile_go_grammar(grammar, force: false)
      grammar_file = @grammars.join("#{grammar}.grammar")
      pkg_dir = "#{grammar}-parser"
      out_dir = @packages.join("go", pkg_dir)
      return unless out_dir.directory?

      cmd = [@go_binary.to_s, "compile-grammar", grammar_file.to_s, "-o", out_dir.join("_grammar.go").to_s, "-p", go_pkg_name(pkg_dir)]
      cmd << "--force" if force
      run_compile("go: #{grammar}-parser/_grammar.go", *cmd)
    end

    def compile_ruby_tokens(grammar)
      source = @grammars.join("#{grammar}.tokens")
      pkg_dir = "#{grammar.tr('-', '_')}_lexer"
      compile_ruby_token_file(source, pkg_dir, "_grammar.rb")
    end

    def compile_ruby_grammar(grammar)
      source = @grammars.join("#{grammar}.grammar")
      pkg_dir = "#{grammar.tr('-', '_')}_parser"
      compile_ruby_parser_file(source, pkg_dir, "_grammar.rb")
    end

    def compile_typescript_tokens(grammar)
      tokens = @grammars.join("#{grammar}.tokens")
      out_dir = @packages.join("typescript", "#{grammar}-lexer", "src")
      return unless out_dir.directory?

      run_compile(
        "typescript: #{grammar}-lexer/_grammar.ts",
        "mise", "exec", "--", "node", @ts_vite.to_s, "--config", @ts_vite_cfg.to_s,
        @ts_wrapper.to_s, "tokens", tokens.to_s, out_dir.join("_grammar.ts").to_s,
        chdir: @ts_dir
      )
    end

    def compile_typescript_grammar(grammar)
      grammar_file = @grammars.join("#{grammar}.grammar")
      out_dir = @packages.join("typescript", "#{grammar}-parser", "src")
      return unless out_dir.directory?

      run_compile(
        "typescript: #{grammar}-parser/_grammar.ts",
        "mise", "exec", "--", "node", @ts_vite.to_s, "--config", @ts_vite_cfg.to_s,
        @ts_wrapper.to_s, "grammar", grammar_file.to_s, out_dir.join("_grammar.ts").to_s,
        chdir: @ts_dir
      )
    end

    def compile_rust_tokens(grammar, tokens_name: nil, force: false)
      tokens_file = @grammars.join("#{tokens_name || grammar}.tokens")
      out_dir = @packages.join("rust", "#{grammar}-lexer", "src")
      return unless out_dir.directory?

      cmd = [@rs_binary.to_s, "compile-tokens", tokens_file.to_s, "-o", out_dir.join("_grammar.rs").to_s]
      cmd << "--force" if force
      run_compile("rust: #{grammar}-lexer/_grammar.rs", *cmd)
    end

    def compile_rust_grammar(grammar, force: false)
      grammar_file = @grammars.join("#{grammar}.grammar")
      out_dir = @packages.join("rust", "#{grammar}-parser", "src")
      return unless out_dir.directory?

      cmd = [@rs_binary.to_s, "compile-grammar", grammar_file.to_s, "-o", out_dir.join("_grammar.rs").to_s]
      cmd << "--force" if force
      run_compile("rust: #{grammar}-parser/_grammar.rs", *cmd)
    end

    def compile_ruby_ecmascript_versions(lexer_pkg, parser_pkg)
      ECMASCRIPT_VERSIONS.each do |version|
        compile_ruby_token_file(@grammars.join("ecmascript", "#{version}.tokens"), lexer_pkg, "_grammar_#{version}.rb")
        compile_ruby_parser_file(@grammars.join("ecmascript", "#{version}.grammar"), parser_pkg, "_grammar_#{version}.rb")
      end
    end

    def compile_ruby_typescript_versions(lexer_pkg, parser_pkg)
      TYPESCRIPT_VERSIONS.each do |version|
        suffix = version.tr(".", "_")
        compile_ruby_token_file(@grammars.join("typescript", "#{version}.tokens"), lexer_pkg, "_grammar_#{suffix}.rb")
        compile_ruby_parser_file(@grammars.join("typescript", "#{version}.grammar"), parser_pkg, "_grammar_#{suffix}.rb")
      end
    end

    def compile_ruby_java_versions(lexer_pkg, parser_pkg)
      compile_ruby_token_file(@grammars.join("java", "java21.tokens"), lexer_pkg, "_grammar.rb")
      compile_ruby_parser_file(@grammars.join("java", "java21.grammar"), parser_pkg, "_grammar.rb")

      JAVA_VERSIONS.each do |version|
        suffix = version.tr(".", "_")
        compile_ruby_token_file(@grammars.join("java", "java#{version}.tokens"), lexer_pkg, "_grammar_#{suffix}.rb")
        compile_ruby_parser_file(@grammars.join("java", "java#{version}.grammar"), parser_pkg, "_grammar_#{suffix}.rb")
      end
    end

    def compile_ruby_python_versions(lexer_pkg, parser_pkg)
      PYTHON_VERSIONS.each do |version|
        suffix = version.tr(".", "_")
        compile_ruby_token_file(@grammars.join("python", "python#{version}.tokens"), lexer_pkg, "_grammar_#{suffix}.rb")
        compile_ruby_parser_file(@grammars.join("python", "python#{version}.grammar"), parser_pkg, "_grammar_#{suffix}.rb")
      end
    end

    def compile_ruby_token_file(source, pkg_dir, output_name)
      out_dir = ruby_output_dir(pkg_dir)
      return unless out_dir.directory?

      puts "  ruby: #{pkg_dir}/#{output_name}"

      grammar_tools = CodingAdventures::GrammarTools
      code = grammar_tools.compile_token_grammar(
        grammar_tools.parse_token_grammar(File.read(source, encoding: "UTF-8")),
        source.basename.to_s
      )
      File.write(out_dir.join(output_name), code)
    rescue StandardError => e
      warn "  FAILED: ruby #{source} -> #{out_dir.join(output_name)}"
      warn "  #{e}"
      @failures += 1
    end

    def compile_ruby_parser_file(source, pkg_dir, output_name)
      out_dir = ruby_output_dir(pkg_dir)
      return unless out_dir.directory?

      puts "  ruby: #{pkg_dir}/#{output_name}"

      grammar_tools = CodingAdventures::GrammarTools
      code = grammar_tools.compile_parser_grammar(
        grammar_tools.parse_parser_grammar(File.read(source, encoding: "UTF-8")),
        source.basename.to_s
      )
      File.write(out_dir.join(output_name), code)
    rescue StandardError => e
      warn "  FAILED: ruby #{source} -> #{out_dir.join(output_name)}"
      warn "  #{e}"
      @failures += 1
    end

    def ruby_output_dir(pkg_dir)
      @packages.join("ruby", pkg_dir, "lib", "coding_adventures", pkg_dir)
    end

    def python_env
      @python_env ||= begin
        paths = Dir.glob(@packages.join("python", "*", "src").to_s).sort
        existing = ENV["PYTHONPATH"]
        {
          "PYTHONPATH" => ([*paths, existing].compact.reject(&:empty?).join(File::PATH_SEPARATOR)),
        }
      end
    end

    def go_pkg_name(dir_name)
      dir_name.delete("-")
    end
  end
end
