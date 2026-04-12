# frozen_string_literal: true

require "coding_adventures_hash_map"
require "coding_adventures_hash_set"
require "coding_adventures_heap"
require "coding_adventures_skip_list"
require "coding_adventures_hyperloglog"
require "coding_adventures_in_memory_data_store_protocol"

module CodingAdventures
  module InMemoryDataStoreEngine
    class CommandError < StandardError; end
    class UnknownCommandError < CommandError; end
    class WrongTypeError < CommandError; end

    Entry = Struct.new(:kind, :value, :expires_at, keyword_init: true)

    class HashValue
      attr_reader :fields

      def initialize
        @fields = ::CodingAdventures::HashMap::HashMap.new
      end

      def set(field, value)
        is_new = !@fields.key?(field)
        @fields[field] = value
        is_new ? 1 : 0
      end

      def get(field)
        @fields[field]
      end

      def delete(field)
        @fields.delete(field)
      end

      def size
        @fields.size
      end

      def to_a
        pairs = []
        @fields.each do |field, value|
          pairs << [field, value]
        end
        pairs.sort_by { |field, _value| field }.flatten(1)
      end
    end

    class SetValue
      attr_reader :members

      def initialize
        @members = ::CodingAdventures::HashSet::HashSet.new
      end

      def add(member)
        added = !@members.include?(member)
        @members.add(member)
        added ? 1 : 0
      end

      def include?(member)
        @members.include?(member)
      end

      def delete(member)
        @members.delete(member) ? 1 : 0
      end

      def size
        @members.size
      end

      def to_a
        @members.to_a.sort
      end

      def union(other)
        self.class.new.tap do |result|
          each_member { |member| result.add(member) }
          other.each_member { |member| result.add(member) }
        end
      end

      def intersection(other)
        self.class.new.tap do |result|
          each_member { |member| result.add(member) if other.include?(member) }
        end
      end

      def difference(other)
        self.class.new.tap do |result|
          each_member { |member| result.add(member) unless other.include?(member) }
        end
      end

      def each_member
        return enum_for(:each_member) unless block_given?

        @members.each { |member| yield member }
      end
    end

    class SortedSetValue
      attr_reader :scores, :order

      def initialize
        @scores = ::CodingAdventures::HashMap::HashMap.new
        @order = ::CodingAdventures::SkipList::SkipList.new
      end

      def add(member, score)
        score = score.to_f
        key = composite_key(member, score)
        existing = @scores[member]
        @order.delete(composite_key(member, existing)) unless existing.nil?
        @scores[member] = score
        inserted = existing.nil? ? 1 : 0
        @order.insert(key, member)
        inserted
      end

      def remove(member)
        score = @scores.delete(member)
        return 0 if score.nil?

        @order.delete(composite_key(member, score))
        1
      end

      def include?(member)
        @scores.key?(member)
      end

      def score(member)
        @scores[member]
      end

      def rank(member)
        score = @scores[member]
        return nil if score.nil?

        @order.rank(composite_key(member, score))
      end

      def size
        @scores.size
      end

      def range(start_index, stop_index, with_scores: false)
        entries = @order.to_a
        start_index, stop_index = normalize_range(start_index, stop_index, entries.length)
        return [] if start_index > stop_index || start_index >= entries.length

        slice = entries[start_index..stop_index] || []
        if with_scores
          slice.flat_map { |(score_member, member)| [member, score_member.first] }
        else
          slice.map { |(_score_member, member)| member }
        end
      end

      def each_member
        return enum_for(:each_member) unless block_given?

        @order.each { |_key, member| yield member }
      end

      private

      def composite_key(member, score)
        [score, member]
      end

      def normalize_range(start_index, stop_index, length)
        start_index = start_index.to_i
        stop_index = stop_index.to_i
        start_index += length if start_index.negative?
        stop_index += length if stop_index.negative?
        start_index = 0 if start_index.negative?
        stop_index = length - 1 if stop_index >= length
        [start_index, stop_index]
      end
    end

    class HyperLogLogValue
      def initialize(precision: 10)
        @hll = ::CodingAdventures::HyperLogLog::HyperLogLog.new(precision: precision)
      end

      def add(value)
        @hll.add(value)
      end

      def count
        @hll.count
      end

      def merge!(other)
        @hll.merge!(other.hll)
      end

      protected

      attr_reader :hll
    end

    class Database
      attr_reader :entries

      def initialize
        @entries = ::CodingAdventures::HashMap::HashMap.new
        @expiry_queue = ::CodingAdventures::Heap::MinHeap.new { |left, right| left[0] <=> right[0] }
      end

      def purge_expired!(now = current_time)
        while (head = @expiry_queue.peek) && head[0] <= now
          expires_at, key = @expiry_queue.pop
          entry = @entries[key]
          next if entry.nil?
          next if entry.expires_at.nil?
          next if entry.expires_at > now
          next if entry.expires_at != expires_at

          @entries.delete(key)
        end
      end

      def fetch_entry(key, now = current_time)
        purge_expired!(now)
        @entries[key]
      end

      def store_entry(key, entry)
        @entries[key] = entry
        @expiry_queue << [entry.expires_at, key] unless entry.expires_at.nil?
        entry
      end

      def delete(key)
        @entries.delete(key)
      end

      def clear
        @entries.clear
        @expiry_queue = ::CodingAdventures::Heap::MinHeap.new { |left, right| left[0] <=> right[0] }
      end

      def size
        purge_expired!
        @entries.size
      end

      def current_time
        Time.now.to_f
      end

      def expire(key, expires_at)
        entry = fetch_entry(key)
        return false if entry.nil?

        entry.expires_at = expires_at
        @expiry_queue << [expires_at, key]
        true
      end

      def persist(key)
        entry = fetch_entry(key)
        return false if entry.nil? || entry.expires_at.nil?

        entry.expires_at = nil
        true
      end
    end

    class Engine
      attr_reader :databases, :selected_db

      def initialize(database_count: 16)
        raise ArgumentError, "database_count must be positive" if database_count <= 0

        @databases = Array.new(database_count) { Database.new }
        @selected_db = 0
        @commands = ::CodingAdventures::HashMap::HashMap.new
        register_builtin_commands
      end

      def register_command(name, &block)
        raise ArgumentError, "command handler is required" unless block

        @commands[name.to_s.upcase] = block
        self
      end

      def register_module(_name, handlers)
        handlers.each do |name, handler|
          register_command(name, &handler)
        end
        self
      end

      def execute(command)
        handler = @commands[command.name.to_s.upcase]
        raise UnknownCommandError, "unknown command '#{command.name}'" if handler.nil?

        current_database.purge_expired!
        instance_exec(command, &handler)
      end

      def current_database
        @databases[@selected_db]
      end

      def selected_database_number
        @selected_db
      end

      private

      def register_builtin_commands
        register_command("PING") { |command| ping(command) }
        register_command("ECHO") { |command| echo(command) }
        register_command("SET") { |command| set(command) }
        register_command("GET") { |command| get(command) }
        register_command("DEL") { |command| delete(command) }
        register_command("EXISTS") { |command| exists(command) }
        register_command("TYPE") { |command| type(command) }
        register_command("HSET") { |command| hset(command) }
        register_command("HGET") { |command| hget(command) }
        register_command("HGETALL") { |command| hgetall(command) }
        register_command("HDEL") { |command| hdel(command) }
        register_command("SADD") { |command| sadd(command) }
        register_command("SISMEMBER") { |command| sismember(command) }
        register_command("SMEMBERS") { |command| smembers(command) }
        register_command("SCARD") { |command| scard(command) }
        register_command("SUNION") { |command| sunion(command) }
        register_command("SINTER") { |command| sinter(command) }
        register_command("SDIFF") { |command| sdiff(command) }
        register_command("ZADD") { |command| zadd(command) }
        register_command("ZRANGE") { |command| zrange(command) }
        register_command("ZRANK") { |command| zrank(command) }
        register_command("ZCARD") { |command| zcard(command) }
        register_command("PFADD") { |command| pfadd(command) }
        register_command("PFCOUNT") { |command| pfcount(command) }
        register_command("PFMERGE") { |command| pfmerge(command) }
        register_command("EXPIRE") { |command| expire(command) }
        register_command("TTL") { |command| ttl(command) }
        register_command("PERSIST") { |command| persist(command) }
        register_command("SELECT") { |command| select_db(command) }
        register_command("FLUSHDB") { |command| flushdb(command) }
        register_command("DBSIZE") { |command| dbsize(command) }
        register_command("INFO") { |command| info(command) }
      end

      def ping(command)
        command.argv.empty? ? "PONG" : command.argv.first
      end

      def echo(command)
        command.argv.first || ""
      end

      def set(command)
        key = require_argument(command, 0)
        value = require_argument(command, 1)
        options = parse_set_options(command.argv.drop(2))
        entry = current_database.fetch_entry(key)

        if options[:nx] && !entry.nil?
          return nil
        end

        if options[:xx] && entry.nil?
          return nil
        end

        expires_at = expiration_timestamp(options, entry)
        current_database.store_entry(key, Entry.new(kind: :string, value: value, expires_at: expires_at))
        "OK"
      end

      def get(command)
        entry = fetch_string_entry(command, 0)
        entry&.value
      end

      def delete(command)
        command.argv.count { |key| !current_database.delete(key).nil? }
      end

      def exists(command)
        command.argv.count { |key| !current_database.fetch_entry(key).nil? }
      end

      def type(command)
        entry = current_database.fetch_entry(require_argument(command, 0))
        entry ? entry.kind.to_s : "none"
      end

      def hset(command)
        key = require_argument(command, 0)
        hash = fetch_or_create_hash(key)
        count = 0
        pairs = command.argv.drop(1)

        raise CommandError, "HSET requires field/value pairs" if pairs.length.odd?

        pairs.each_slice(2) do |field, value|
          count += hash.set(field, value)
        end

        store_typed_value(key, :hash, hash)
        count
      end

      def hget(command)
        hash = fetch_hash(command, 0)
        hash&.get(require_argument(command, 1))
      end

      def hgetall(command)
        hash = fetch_hash(command, 0)
        hash ? hash.to_a : []
      end

      def hdel(command)
        hash = fetch_hash(command, 0)
        return 0 if hash.nil?

        command.argv.drop(1).sum { |field| hash.delete(field).nil? ? 0 : 1 }
      end

      def sadd(command)
        key = require_argument(command, 0)
        set = fetch_or_create_set(key)
        added = command.argv.drop(1).inject(0) { |sum, member| sum + set.add(member) }
        store_typed_value(key, :set, set)
        added
      end

      def sismember(command)
        set = fetch_set(command, 0)
        return 0 if set.nil?

        set.include?(require_argument(command, 1)) ? 1 : 0
      end

      def smembers(command)
        set = fetch_set(command, 0)
        set ? set.to_a.sort : []
      end

      def scard(command)
        set = fetch_set(command, 0)
        set ? set.size : 0
      end

      def sunion(command)
        sets = command.argv.map.with_index { |_key, index| fetch_set(command, index) }.compact
        result = ::CodingAdventures::HashSet::HashSet.new
        sets.each { |set| result = result.union(set) }
        result.to_a.sort
      end

      def sinter(command)
        sets = command.argv.map.with_index { |_key, index| fetch_set(command, index) }.compact
        return [] if sets.empty?

        result = sets.first
        sets.drop(1).each { |set| result = result.intersection(set) }
        result.to_a.sort
      end

      def sdiff(command)
        base = fetch_set(command, 0)
        return [] if base.nil?

        result = base
        command.argv.drop(1).each_with_index do |_key, index|
          other = fetch_set(command, index + 1)
          result = result.difference(other) unless other.nil?
        end
        result.to_a.sort
      end

      def zadd(command)
        key = require_argument(command, 0)
        zset = fetch_or_create_zset(key)
        pairs = command.argv.drop(1)
        raise CommandError, "ZADD requires score/member pairs" if pairs.length.odd?

        added = 0
        pairs.each_slice(2) do |score, member|
          added += zset.add(member, score)
        end
        store_typed_value(key, :zset, zset)
        added
      end

      def zrange(command)
        zset = fetch_zset(command, 0)
        return [] if zset.nil?

        start_index = require_argument(command, 1).to_i
        stop_index = require_argument(command, 2).to_i
        with_scores = command.argv.drop(3).map(&:upcase).include?("WITHSCORES")
        zset.range(start_index, stop_index, with_scores: with_scores)
      end

      def zrank(command)
        zset = fetch_zset(command, 0)
        return nil if zset.nil?

        zset.rank(require_argument(command, 1))
      end

      def zcard(command)
        zset = fetch_zset(command, 0)
        zset ? zset.size : 0
      end

      def pfadd(command)
        key = require_argument(command, 0)
        hll = fetch_or_create_hll(key)
        command.argv.drop(1).each { |member| hll.add(member) }
        store_typed_value(key, :hyperloglog, hll)
        command.argv.length > 1 ? 1 : 0
      end

      def pfcount(command)
        hlls = command.argv.map.with_index { |_key, index| fetch_hll(command, index) }.compact
        return 0 if hlls.empty?

        result = hlls.first
        hlls.drop(1).each { |other| result.merge!(other) }
        result.count.round
      end

      def pfmerge(command)
        destination = require_argument(command, 0)
        sources = command.argv.drop(1)
        merged = fetch_or_create_hll(destination)
        sources.each_with_index do |_key, index|
          source = fetch_hll(command, index + 1)
          next if source.nil?

          merged.merge!(source)
        end
        store_typed_value(destination, :hyperloglog, merged)
        "OK"
      end

      def expire(command)
        key = require_argument(command, 0)
        seconds = require_argument(command, 1).to_i
        entry = current_database.fetch_entry(key)
        return 0 if entry.nil?

        current_database.expire(key, current_database.current_time + seconds)
        1
      end

      def ttl(command)
        entry = current_database.fetch_entry(require_argument(command, 0))
        return -2 if entry.nil?
        return -1 if entry.expires_at.nil?

        remaining = (entry.expires_at - current_database.current_time).ceil
        remaining.negative? ? -2 : remaining
      end

      def persist(command)
        current_database.persist(require_argument(command, 0)) ? 1 : 0
      end

      def select_db(command)
        index = require_argument(command, 0).to_i
        raise CommandError, "database index out of range" unless index.between?(0, @databases.length - 1)

        @selected_db = index
        "OK"
      end

      def flushdb(_command)
        current_database.clear
        "OK"
      end

      def dbsize(_command)
        current_database.size
      end

      def info(_command)
        "databases=#{@databases.length}\nselected_db=#{@selected_db}\nkeys=#{current_database.size}"
      end

      def require_argument(command, index)
        value = command.argv[index]
        raise CommandError, "missing argument at position #{index}" if value.nil?

        value
      end

      def parse_set_options(arguments)
        options = { nx: false, xx: false, ex: nil, px: nil, keepttl: false }
        i = 0

        while i < arguments.length
          token = arguments[i].to_s.upcase
          case token
          when "NX"
            options[:nx] = true
          when "XX"
            options[:xx] = true
          when "EX"
            options[:ex] = arguments.fetch(i + 1).to_i
            i += 1
          when "PX"
            options[:px] = arguments.fetch(i + 1).to_i
            i += 1
          when "KEEPTTL"
            options[:keepttl] = true
          else
            raise CommandError, "unknown SET option '#{arguments[i]}'"
          end
          i += 1
        end

        options
      end

      def expiration_timestamp(options, existing_entry)
        return existing_entry.expires_at if options[:keepttl] && !existing_entry.nil?
        return nil if options[:ex].nil? && options[:px].nil?

        delta = options[:ex] || (options[:px].to_f / 1000.0)
        current_database.current_time + delta.to_f
      end

      def fetch_string_entry(command, index)
        entry = current_database.fetch_entry(require_argument(command, index))
        return nil if entry.nil?
        raise WrongTypeError, "WRONGTYPE Operation against a key holding the wrong kind of value" unless entry.kind == :string

        entry
      end

      def fetch_hash(command, index)
        entry = current_database.fetch_entry(require_argument(command, index))
        return nil if entry.nil?
        raise WrongTypeError, "WRONGTYPE Operation against a key holding the wrong kind of value" unless entry.kind == :hash

        entry.value
      end

      def fetch_or_create_hash(key)
        existing = current_database.fetch_entry(key)
        if existing.nil?
          HashValue.new
        else
          raise WrongTypeError, "WRONGTYPE Operation against a key holding the wrong kind of value" unless existing.kind == :hash

          existing.value
        end
      end

      def fetch_set(command, index)
        entry = current_database.fetch_entry(require_argument(command, index))
        return nil if entry.nil?
        raise WrongTypeError, "WRONGTYPE Operation against a key holding the wrong kind of value" unless entry.kind == :set

        entry.value
      end

      def fetch_or_create_set(key)
        existing = current_database.fetch_entry(key)
        if existing.nil?
          SetValue.new
        else
          raise WrongTypeError, "WRONGTYPE Operation against a key holding the wrong kind of value" unless existing.kind == :set

          existing.value
        end
      end

      def fetch_zset(command, index)
        entry = current_database.fetch_entry(require_argument(command, index))
        return nil if entry.nil?
        raise WrongTypeError, "WRONGTYPE Operation against a key holding the wrong kind of value" unless entry.kind == :zset

        entry.value
      end

      def fetch_or_create_zset(key)
        existing = current_database.fetch_entry(key)
        if existing.nil?
          SortedSetValue.new
        else
          raise WrongTypeError, "WRONGTYPE Operation against a key holding the wrong kind of value" unless existing.kind == :zset

          existing.value
        end
      end

      def fetch_hll(command, index)
        entry = current_database.fetch_entry(require_argument(command, index))
        return nil if entry.nil?
        raise WrongTypeError, "WRONGTYPE Operation against a key holding the wrong kind of value" unless entry.kind == :hyperloglog

        entry.value
      end

      def fetch_or_create_hll(key)
        existing = current_database.fetch_entry(key)
        if existing.nil?
          HyperLogLogValue.new
        else
          raise WrongTypeError, "WRONGTYPE Operation against a key holding the wrong kind of value" unless existing.kind == :hyperloglog

          existing.value
        end
      end

      def store_typed_value(key, kind, value)
        entry = Entry.new(kind: kind, value: value, expires_at: current_database.fetch_entry(key)&.expires_at)
        current_database.store_entry(key, entry)
      end

    end
  end
end
