# frozen_string_literal: true

module CodingAdventures
  module WasmModuleEncoder
    class WasmEncodeError < StandardError; end

    WASM_MAGIC = "\0asm".b
    WASM_VERSION = [0x01, 0x00, 0x00, 0x00].pack("C*").b

    IMPORT_KIND_BY_NAME = {
      func: 0,
      function: 0,
      table: 1,
      mem: 2,
      memory: 2,
      global: 3
    }.freeze

    module_function

    def encode_module(wasm_module)
      sections = +"".b

      (wasm_module.customs || []).each do |custom|
        sections << section(0, encode_custom(custom))
      end
      sections << section(1, encode_vector(wasm_module.types, method(:encode_func_type))) if wasm_module.types.any?
      sections << section(2, encode_vector(wasm_module.imports, method(:encode_import))) if wasm_module.imports.any?
      sections << section(3, encode_vector(wasm_module.functions, method(:u32))) if wasm_module.functions.any?
      sections << section(4, encode_vector(wasm_module.tables, method(:encode_table_type))) if wasm_module.tables.any?
      sections << section(5, encode_vector(wasm_module.memories, method(:encode_memory_type))) if wasm_module.memories.any?
      sections << section(6, encode_vector(wasm_module.globals, method(:encode_global))) if wasm_module.globals.any?
      sections << section(7, encode_vector(wasm_module.exports, method(:encode_export))) if wasm_module.exports.any?
      sections << section(8, u32(wasm_module.start)) unless wasm_module.start.nil?
      sections << section(9, encode_vector(wasm_module.elements, method(:encode_element))) if wasm_module.elements.any?
      sections << section(10, encode_vector(wasm_module.code, method(:encode_function_body))) if wasm_module.code.any?
      sections << section(11, encode_vector(wasm_module.data, method(:encode_data_segment))) if wasm_module.data.any?

      WASM_MAGIC + WASM_VERSION + sections
    end

    def section(section_id, payload)
      [section_id].pack("C") + u32(payload.bytesize) + payload
    end

    def u32(value)
      CodingAdventures::WasmLeb128.encode_unsigned(value)
    end

    def encode_name(text)
      utf8 = text.encode("UTF-8")
      u32(utf8.bytesize) + utf8.b
    end

    def encode_vector(values, encoder)
      values.reduce(u32(values.length)) { |bytes, value| bytes << encoder.call(value) }
    end

    def encode_value_types(types)
      u32(types.length) + types.pack("C*")
    end

    def encode_func_type(func_type)
      [0x60].pack("C") +
        encode_value_types(func_type.params || []) +
        encode_value_types(func_type.results || [])
    end

    def encode_limits(limits)
      if limits.max.nil?
        [0x00].pack("C") + u32(limits.min)
      else
        [0x01].pack("C") + u32(limits.min) + u32(limits.max)
      end
    end

    def encode_memory_type(memory_type)
      encode_limits(memory_type.limits || memory_type)
    end

    def encode_table_type(table_type)
      [table_type.element_type || table_type.ref_type].pack("C") + encode_limits(table_type.limits)
    end

    def encode_global_type(global_type)
      [
        global_type.value_type || global_type.val_type,
        global_type.mutable ? 0x01 : 0x00
      ].pack("C*")
    end

    def encode_import(import_value)
      kind = kind_byte(import_value.kind || import_value.desc&.kind)
      payload = +"".b
      payload << encode_name(import_value.module_name || import_value.mod)
      payload << encode_name(import_value.name)
      payload << [kind].pack("C")

      case kind
      when 0
        type_index = optional_attr(import_value, :type_index) ||
          optional_attr(import_value, :type_info) ||
          optional_attr(import_value, :typeInfo) ||
          import_value.desc&.type_idx
        raise WasmEncodeError, "function imports require an integer type index" unless type_index.is_a?(Integer)

        payload << u32(type_index)
      when 1
        table_type = optional_attr(import_value, :type_info) || optional_attr(import_value, :typeInfo)
        table_type ||= CodingAdventures::WasmTypes::TableType.new(import_value.desc.ref_type, import_value.desc.limits) if import_value.respond_to?(:desc) && import_value.desc
        raise WasmEncodeError, "table imports require TableType metadata" unless table_type

        payload << encode_table_type(table_type)
      when 2
        memory_type = optional_attr(import_value, :type_info) || optional_attr(import_value, :typeInfo)
        memory_type ||= CodingAdventures::WasmTypes::MemoryType.new(import_value.desc.limits) if import_value.respond_to?(:desc) && import_value.desc
        raise WasmEncodeError, "memory imports require MemoryType metadata" unless memory_type

        payload << encode_memory_type(memory_type)
      when 3
        global_type = optional_attr(import_value, :type_info) || optional_attr(import_value, :typeInfo)
        if global_type.nil? && import_value.respond_to?(:desc) && import_value.desc
          global_type = CodingAdventures::WasmTypes::GlobalType.new(import_value.desc.val_type, import_value.desc.mutable)
        end
        raise WasmEncodeError, "global imports require GlobalType metadata" unless global_type

        payload << encode_global_type(global_type)
      else
        raise WasmEncodeError, "unsupported import kind: #{import_value.kind.inspect}"
      end

      payload
    end

    def encode_export(export_value)
      kind = kind_byte(export_value.kind || export_value.desc&.kind)
      index = export_value.index
      index = export_value.desc.idx if export_value.respond_to?(:desc) && export_value.desc

      encode_name(export_value.name) + [kind].pack("C") + u32(index)
    end

    def encode_global(global_value)
      encode_global_type(global_value.global_type || global_value.type_info || global_value) +
        bytes_from_value(global_value.init_expr || global_value.init)
    end

    def encode_element(element)
      payload = +"".b
      payload << u32(element.table_index)
      payload << bytes_from_value(element.offset_expr)
      payload << u32((element.function_indices || []).length)
      (element.function_indices || []).each do |func_index|
        payload << u32(func_index)
      end
      payload
    end

    def encode_data_segment(segment)
      data = bytes_from_value(segment.data)
      u32(segment.memory_index) + bytes_from_value(segment.offset_expr) + u32(data.bytesize) + data
    end

    def encode_function_body(body)
      local_groups = group_locals(body.locals || [])
      payload = +"".b
      payload << u32(local_groups.length)
      local_groups.each do |count, value_type|
        payload << u32(count)
        payload << [value_type].pack("C")
      end
      payload << bytes_from_value(body.code || body.body)
      u32(payload.bytesize) + payload
    end

    def group_locals(locals_)
      return [] if locals_.empty?

      groups = []
      current_type = locals_.first
      count = 1

      locals_[1..].each do |value_type|
        if value_type == current_type
          count += 1
        else
          groups << [count, current_type]
          current_type = value_type
          count = 1
        end
      end
      groups << [count, current_type]
      groups
    end

    def encode_custom(custom)
      encode_name(custom.name) + bytes_from_value(custom.data)
    end

    def kind_byte(kind)
      return kind if kind.is_a?(Integer)

      mapped = IMPORT_KIND_BY_NAME[kind]
      raise WasmEncodeError, "unsupported external kind: #{kind.inspect}" if mapped.nil?

      mapped
    end

    def bytes_from_value(value)
      return +"".b if value.nil?
      return value.b if value.is_a?(String)
      return value.pack("C*").b if value.is_a?(Array)

      value.to_s.b
    end

    def optional_attr(object, name)
      object.public_send(name) if object.respond_to?(name)
    end
  end
end
