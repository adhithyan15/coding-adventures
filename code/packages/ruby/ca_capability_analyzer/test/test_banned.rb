# frozen_string_literal: true

require "test_helper"

# ============================================================================
# Tests for the Banned Construct Detector
# ============================================================================
#
# These tests verify that the banned construct detector correctly identifies
# dynamic execution constructs that evade static analysis. Each test provides
# a snippet of Ruby code and checks that the detector flags the right
# constructs.
#
# The tests are organized by banned construct type:
# 1. eval and eval-family methods
# 2. Dynamic dispatch (send/public_send with dynamic args)
# 3. Dynamic require
# 4. Dynamic const_get
# 5. Dynamic define_method
# 6. method_missing definition
# 7. Backtick execution
# 8. System with interpolation
# 9. Safe code (no violations)
# ============================================================================

class TestBanned < Minitest::Test
  # Helper: detect banned constructs in a source string.
  def detect(source, filename: "test.rb")
    detector = CA::CapabilityAnalyzer::BannedConstructDetector.new(filename)
    detector.detect(source)
  end

  # ── eval Detection ───────────────────────────────────────────────

  def test_bare_eval_is_banned
    violations = detect('eval("puts 1")')
    assert_equal 1, violations.length
    assert_equal "eval", violations.first.construct
  end

  def test_eval_with_variable_is_banned
    violations = detect("eval(code)")
    assert_equal 1, violations.length
    assert_equal "eval", violations.first.construct
  end

  # ── Eval Family Detection ────────────────────────────────────────

  def test_instance_eval_with_string_is_banned
    violations = detect('obj.instance_eval("@secret")')
    assert_equal 1, violations.length
    assert_equal "instance_eval", violations.first.construct
  end

  def test_class_eval_with_string_is_banned
    violations = detect('MyClass.class_eval("attr_accessor :x")')
    assert_equal 1, violations.length
    assert_equal "class_eval", violations.first.construct
  end

  def test_module_eval_with_string_is_banned
    violations = detect('MyModule.module_eval("def foo; end")')
    assert_equal 1, violations.length
    assert_equal "module_eval", violations.first.construct
  end

  def test_instance_eval_with_block_is_not_banned
    # Block form is safe — the code is visible in the AST
    violations = detect("obj.instance_eval { @secret }")
    assert_empty violations
  end

  def test_class_eval_with_dynamic_arg_is_banned
    violations = detect("MyClass.class_eval(code_string)")
    assert_equal 1, violations.length
    assert_equal "class_eval", violations.first.construct
  end

  # ── Dynamic Dispatch Detection ───────────────────────────────────

  def test_send_with_literal_symbol_is_safe
    violations = detect("obj.send(:to_s)")
    assert_empty violations
  end

  def test_send_with_literal_string_is_safe
    violations = detect('obj.send("to_s")')
    assert_empty violations
  end

  def test_send_with_variable_is_banned
    violations = detect("obj.send(method_name)")
    assert_equal 1, violations.length
    assert_equal "dynamic_send", violations.first.construct
  end

  def test_public_send_with_variable_is_banned
    violations = detect("obj.public_send(method_name)")
    assert_equal 1, violations.length
    assert_equal "dynamic_public_send", violations.first.construct
  end

  def test___send___with_variable_is_banned
    violations = detect("obj.__send__(method_name)")
    assert_equal 1, violations.length
    assert_equal "dynamic___send__", violations.first.construct
  end

  # ── Dynamic Require Detection ────────────────────────────────────

  def test_require_with_literal_is_safe
    violations = detect('require "json"')
    assert_empty violations
  end

  def test_require_with_variable_is_banned
    violations = detect("require(lib_name)")
    assert_equal 1, violations.length
    assert_equal "dynamic_require", violations.first.construct
  end

  def test_require_relative_with_variable_is_banned
    violations = detect("require_relative(path)")
    assert_equal 1, violations.length
    assert_equal "dynamic_require", violations.first.construct
  end

  # ── Dynamic const_get Detection ──────────────────────────────────

  def test_const_get_with_literal_symbol_is_safe
    violations = detect("Object.const_get(:String)")
    assert_empty violations
  end

  def test_const_get_with_literal_string_is_safe
    violations = detect('Object.const_get("String")')
    assert_empty violations
  end

  def test_const_get_with_variable_is_banned
    violations = detect("Object.const_get(class_name)")
    assert_equal 1, violations.length
    assert_equal "dynamic_const_get", violations.first.construct
  end

  def test_const_get_on_module_with_variable_is_banned
    violations = detect("MyModule.const_get(name)")
    assert_equal 1, violations.length
    assert_equal "dynamic_const_get", violations.first.construct
  end

  # ── Dynamic define_method Detection ──────────────────────────────

  def test_define_method_with_literal_symbol_is_safe
    violations = detect("define_method(:foo) { 42 }")
    assert_empty violations
  end

  def test_define_method_with_literal_string_is_safe
    violations = detect('define_method("foo") { 42 }')
    assert_empty violations
  end

  def test_define_method_with_variable_is_banned
    violations = detect("define_method(name) { 42 }")
    assert_equal 1, violations.length
    assert_equal "dynamic_define_method", violations.first.construct
  end

  # ── method_missing Definition Detection ──────────────────────────

  def test_method_missing_definition_is_banned
    source = <<~RUBY
      class DynamicProxy
        def method_missing(name, *args)
          target.send(name, *args)
        end
      end
    RUBY
    violations = detect(source)
    # Should detect both method_missing definition AND dynamic send
    method_missing_violations = violations.select { |v| v.construct == "method_missing_definition" }
    assert_equal 1, method_missing_violations.length
  end

  def test_regular_method_definition_is_safe
    source = <<~RUBY
      class MyClass
        def my_method
          42
        end
      end
    RUBY
    violations = detect(source)
    assert_empty violations
  end

  # ── Backtick Execution Detection ─────────────────────────────────

  def test_backtick_execution_is_banned
    violations = detect('`ls -la`')
    assert_equal 1, violations.length
    assert_equal "backtick_execution", violations.first.construct
  end

  def test_interpolated_backtick_is_banned
    violations = detect('`ls #{dir}`')
    assert_equal 1, violations.length
    assert_equal "backtick_execution", violations.first.construct
  end

  # ── System with Interpolation Detection ──────────────────────────

  def test_system_with_interpolation_is_banned
    violations = detect('system("rm -rf #{path}")')
    assert_equal 1, violations.length
    assert_equal "system_interpolation", violations.first.construct
  end

  def test_exec_with_interpolation_is_banned
    violations = detect('exec("#{cmd} --force")')
    assert_equal 1, violations.length
    assert_equal "exec_interpolation", violations.first.construct
  end

  def test_system_with_literal_is_not_flagged_as_interpolation
    violations = detect('system("ls -la")')
    assert_empty violations
  end

  # ── Banned Class Methods Detection ───────────────────────────────

  def test_binding_eval_is_banned
    violations = detect('Binding.eval("code")')
    assert_equal 1, violations.length
    assert_equal "Binding.eval", violations.first.construct
  end

  def test_kernel_eval_is_banned
    violations = detect('Kernel.eval("code")')
    assert_equal 1, violations.length
    assert_equal "Kernel.eval", violations.first.construct
  end

  def test_kernel_system_is_banned
    violations = detect('Kernel.system("cmd")')
    assert_equal 1, violations.length
    assert_equal "Kernel.system", violations.first.construct
  end

  # ── Safe Code (No Violations) ────────────────────────────────────

  def test_pure_arithmetic_no_violations
    violations = detect("x = 1 + 2 * 3")
    assert_empty violations
  end

  def test_normal_method_calls_no_violations
    source = <<~RUBY
      result = [1, 2, 3].map { |x| x * 2 }
      puts result.inspect
    RUBY
    violations = detect(source)
    assert_empty violations
  end

  def test_hash_access_no_violations
    source = <<~RUBY
      config = { name: "test", value: 42 }
      puts config[:name]
    RUBY
    violations = detect(source)
    assert_empty violations
  end

  # ── Violation Metadata ───────────────────────────────────────────

  def test_violation_records_line_number
    source = <<~RUBY
      x = 1
      y = 2
      eval("puts 3")
    RUBY
    violations = detect(source)
    assert_equal 3, violations.first.line
  end

  def test_violation_records_filename
    violations = detect('eval("code")', filename: "lib/my_module.rb")
    assert_equal "lib/my_module.rb", violations.first.file
  end

  def test_violation_to_s_format
    v = CA::CapabilityAnalyzer::BannedConstructViolation.new(
      construct: "eval", file: "test.rb", line: 5, evidence: "eval(...)"
    )
    assert_includes v.to_s, "BANNED eval"
    assert_includes v.to_s, "test.rb:5"
  end

  # ── Multiple Violations ──────────────────────────────────────────

  def test_multiple_violations_detected
    source = <<~RUBY
      eval("code1")
      obj.send(dynamic_method)
      `dangerous_command`
    RUBY
    violations = detect(source)
    assert_equal 3, violations.length

    constructs = violations.map(&:construct)
    assert_includes constructs, "eval"
    assert_includes constructs, "dynamic_send"
    assert_includes constructs, "backtick_execution"
  end

  # ── module_eval with interpolated string ─────────────────────────

  def test_module_eval_with_interpolated_string_is_banned
    violations = detect('MyModule.module_eval("def #{name}; end")')
    assert_equal 1, violations.length
    assert_equal "module_eval", violations.first.construct
  end
end
