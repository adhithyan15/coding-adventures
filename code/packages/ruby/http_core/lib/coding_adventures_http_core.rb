# frozen_string_literal: true

require_relative "coding_adventures/http_core/version"

module CodingAdventures
  # Shared HTTP message types and helpers.
  module HttpCore
    Header = Data.define(:name, :value)

    class HttpVersion < Data.define(:major, :minor)
      def self.parse(text)
        raise ArgumentError, "invalid HTTP version: #{text}" unless text.start_with?("HTTP/")

        major_text, minor_text = text.delete_prefix("HTTP/").split(".", 2)
        unless major_text&.match?(/\A\d+\z/) && minor_text&.match?(/\A\d+\z/)
          raise ArgumentError, "invalid HTTP version: #{text}"
        end

        new(major_text.to_i, minor_text.to_i)
      end

      def to_s
        "HTTP/#{major}.#{minor}"
      end
    end

    class BodyKind < Data.define(:mode, :length)
      def self.none
        new("none", nil)
      end

      def self.content_length(length)
        new("content-length", length)
      end

      def self.until_eof
        new("until-eof", nil)
      end

      def self.chunked
        new("chunked", nil)
      end
    end

    class RequestHead < Data.define(:method, :target, :version, :headers)
      def header(name)
        HttpCore.find_header(headers, name)
      end

      def content_length
        HttpCore.parse_content_length(headers)
      end

      def content_type
        HttpCore.parse_content_type(headers)
      end
    end

    class ResponseHead < Data.define(:version, :status, :reason, :headers)
      def header(name)
        HttpCore.find_header(headers, name)
      end

      def content_length
        HttpCore.parse_content_length(headers)
      end

      def content_type
        HttpCore.parse_content_type(headers)
      end
    end

    module_function

    def find_header(headers, name)
      headers.find { |header| header.name.casecmp?(name) }&.value
    end

    def parse_content_length(headers)
      value = find_header(headers, "Content-Length")
      return nil unless value&.match?(/\A\d+\z/)

      value.to_i
    end

    def parse_content_type(headers)
      value = find_header(headers, "Content-Type")
      return nil unless value

      pieces = value.split(";").map(&:strip)
      media_type = pieces.shift
      return nil if media_type.nil? || media_type.empty?

      charset = nil
      pieces.each do |piece|
        key, raw_value = piece.split("=", 2)
        next unless raw_value && key.strip.casecmp?("charset")

        charset = raw_value.strip.delete_prefix('"').delete_suffix('"')
        break
      end

      [media_type, charset]
    end
  end
end
