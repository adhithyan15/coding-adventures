# frozen_string_literal: true

# IMPORTANT: Require dependencies FIRST, before own modules.
# Ruby loads files in require order. If our modules reference
# constants from dependencies, those gems must be loaded first.
require "coding_adventures_http_core"

require_relative "coding_adventures/http1/version"

module CodingAdventures
  # HTTP/1 request and response head parser with body framing detection
  module Http1
    class ParseError < ArgumentError; end

    ParsedRequestHead = Data.define(:head, :body_offset, :body_kind)
    ParsedResponseHead = Data.define(:head, :body_offset, :body_kind)

    module_function

    def parse_request_head(input)
      lines, body_offset = split_head_lines(input)
      start_line = lines.fetch(0) { raise ParseError, "invalid HTTP/1 start line" }
      method, target, version_text = start_line.split(/\s+/)
      raise ParseError, "invalid HTTP/1 start line: #{start_line}" unless method && target && version_text
      raise ParseError, "invalid HTTP/1 start line: #{start_line}" unless start_line.split(/\s+/).length == 3

      version = HttpCore::HttpVersion.parse(version_text)
      headers = parse_headers(lines.drop(1))
      ParsedRequestHead.new(
        HttpCore::RequestHead.new(method, target, version, headers),
        body_offset,
        request_body_kind(headers)
      )
    rescue ArgumentError => error
      raise ParseError, error.message
    end

    def parse_response_head(input)
      lines, body_offset = split_head_lines(input)
      status_line = lines.fetch(0) { raise ParseError, "invalid HTTP/1 status line" }
      parts = status_line.split(/\s+/)
      raise ParseError, "invalid HTTP/1 status line: #{status_line}" if parts.length < 2

      version = HttpCore::HttpVersion.parse(parts[0])
      status = Integer(parts[1], 10)
      headers = parse_headers(lines.drop(1))

      ParsedResponseHead.new(
        HttpCore::ResponseHead.new(version, status, parts.drop(2).join(" "), headers),
        body_offset,
        response_body_kind(status, headers)
      )
    rescue ArgumentError => error
      raise ParseError, error.message
    end

    def split_head_lines(input)
      buffer = input.b
      index = 0

      while buffer.byteslice(index, 2) == "\r\n" || buffer.byteslice(index, 1) == "\n"
        index += buffer.byteslice(index, 2) == "\r\n" ? 2 : 1
      end

      lines = []
      loop do
        raise ParseError, "incomplete HTTP/1 head" if index >= buffer.bytesize

        line_end = buffer.index("\n", index)
        raise ParseError, "incomplete HTTP/1 head" if line_end.nil?

        line = buffer.byteslice(index, line_end - index)
        line = line.delete_suffix("\r")
        index = line_end + 1

        return [lines, index] if line.empty?

        lines << line
      end
    end

    def parse_headers(lines)
      lines.map do |line|
        name, raw_value = line.split(":", 2)
        raise ParseError, "invalid HTTP/1 header: #{line}" if raw_value.nil? || name.strip.empty?

        HttpCore::Header.new(name.strip, raw_value.strip)
      end
    end

    def request_body_kind(headers)
      return HttpCore::BodyKind.chunked if chunked_transfer_encoding?(headers)

      length = declared_content_length(headers)
      return HttpCore::BodyKind.none if length.nil? || length.zero?

      HttpCore::BodyKind.content_length(length)
    end

    def response_body_kind(status, headers)
      return HttpCore::BodyKind.none if (100...200).cover?(status) || [204, 304].include?(status)
      return HttpCore::BodyKind.chunked if chunked_transfer_encoding?(headers)

      length = declared_content_length(headers)
      return HttpCore::BodyKind.until_eof if length.nil?
      return HttpCore::BodyKind.none if length.zero?

      HttpCore::BodyKind.content_length(length)
    end

    def declared_content_length(headers)
      value = headers.find { |header| header.name.casecmp?("Content-Length") }&.value
      return nil if value.nil?
      raise ParseError, "invalid Content-Length: #{value}" unless value.match?(/\A\d+\z/)

      value.to_i
    end

    def chunked_transfer_encoding?(headers)
      headers
        .select { |header| header.name.casecmp?("Transfer-Encoding") }
        .any? do |header|
          header.value.split(",").any? { |piece| piece.strip.casecmp?("chunked") }
        end
    end
  end
end
