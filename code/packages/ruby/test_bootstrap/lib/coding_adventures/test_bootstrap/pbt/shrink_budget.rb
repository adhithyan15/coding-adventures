# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Pbt
      # The budget of a shrink, as a TREE of scopes (VER05h 3.1, ported from the
      # Java engine's VER05e 4.1 and C# VER05g 3.1).
      #
      # Why a tree?  A property may itself call `Runner.check`, a custom shrinker
      # may call `Gen#shrink`, a filter predicate may do either.  If each of those
      # got a fresh budget the work would multiply with the nesting depth.  Here
      # every public entry point that runs user code opens a CHILD of the ambient
      # scope with its own local cap, and a charge walks from the node up to the
      # outermost root: every node on the path must have room, then every counter
      # is incremented, all or nothing.  Whatever the nesting, the total work of
      # one top-level check is capped by the outermost root.
      #
      # Two things are charged: ATTEMPTS (one per property evaluation) and WORK
      # (elements copied, at least 1 per charge).  One Mutex, owned by the outermost
      # root and shared by the whole tree, guards every counter.  User code never
      # runs while it is held (a Ruby Mutex is not re-entrant).
      #
      # The current node is carried in Fiber storage, which Thread.new and
      # Fiber.new copy from their creator, so a thread created inside a scope
      # charges the same tree.  A thread that already existed has empty storage
      # and gets a fresh root (documented, pinned by a test).
      class ShrinkBudget
        # Most nested owning scopes; deeper scopes join, keeping the walk short.
        MAX_DEPTH = 64
        KEY = :coding_adventures_pbt_budget

        @default_max_work = Limits::MAX_SHRINK_WORK
        @last_work = 0
        @last_attempts = 0
        @stats_lock = Mutex.new

        class << self
          # Test hook, read only when a root is created (Hooks.reset restores it).
          attr_accessor :default_max_work

          def last_work = @stats_lock.synchronize { @last_work }

          def last_attempts = @stats_lock.synchronize { @last_attempts }

          def record(work, attempts)
            @stats_lock.synchronize do
              @last_work = work
              @last_attempts = attempts
            end
          end

          # The nearest open node from `node` upward whose root is open, or nil.
          def usable(node)
            node = node.parent while node&.closed?
            (node && !node.root.closed?) ? node : nil
          end

          # The ambient open budget of this fiber, or nil outside any shrink.
          def current = usable(Fiber[KEY])

          # Joins the ambient scope when there is one, else opens a fresh root.
          # With a block: yields it and always closes an owning scope.
          def enter(max_steps, &)
            open = usable(Fiber[KEY])
            scope = open ? new(joined: open) : new_root(Limits.attempt_budget(max_steps), default_max_work, nil)
            run(scope, &)
          end

          # The scope of a public entry point: a CHILD of the ambient scope with a
          # fresh local cap, or a fresh root when none is open.  Past MAX_DEPTH it joins.
          def enter_child(max_steps, max_work = Limits::MAX_SHRINK_WORK, &)
            ambient = Fiber[KEY]
            open = usable(ambient)
            scope =
              if open.nil?
                cap = (max_work == Limits::MAX_SHRINK_WORK) ? default_max_work : max_work
                new_root(Limits.attempt_budget(max_steps), cap, nil)
              elsif open.depth + 1 >= MAX_DEPTH
                new(joined: open)
              else
                make_child(open, Limits.attempt_budget(max_steps), max_work, ambient)
              end
            run(scope, &)
          end

          # The scope of a whole Runner.check (case loop included): a root with no
          # open ambient scope, a child otherwise.  Its attempt cap is num_tests plus
          # the shrink attempt cap, so a nested check's cases and shrink are all
          # charged up the chain.
          def enter_check(num_tests, max_steps, &)
            cap = num_tests + Limits.attempt_budget(max_steps)
            ambient = Fiber[KEY]
            open = usable(ambient)
            scope =
              if open.nil?
                new_root(cap, default_max_work, nil)
              elsif open.depth + 1 >= MAX_DEPTH
                new(joined: open)
              else
                make_child(open, cap, Limits::MAX_SHRINK_WORK, ambient)
              end
            run(scope, &)
          end

          private

          def new_root(max_attempts, max_work, previous)
            new(parent: nil, max_attempts: max_attempts, max_work: max_work, previous: previous)
          end

          def make_child(parent, max_attempts, max_work, previous)
            new(parent: parent, max_attempts: max_attempts, max_work: max_work, previous: previous)
          end

          # The ambient value is set INSIDE the begin that guarantees close (or, with
          # no block, immediately before returning, with no call in between), so an
          # exception in that window cannot leave an orphaned ambient scope.
          def run(scope)
            unless block_given?
              Fiber[KEY] = scope unless scope.joined?
              return scope
            end

            begin
              Fiber[KEY] = scope unless scope.joined?
              yield scope
            ensure
              scope.close
            end
          end
        end

        attr_reader :parent, :root, :depth

        def initialize(parent: nil, max_attempts: 0, max_work: 0, previous: nil, joined: nil)
          if joined
            # A joined handle: charges and queries go to the owner it aliases.
            @target = joined.owner
            @root = joined.root
            @sync = joined.sync_object
            @depth = joined.depth
          else
            @parent = parent
            @root = parent ? parent.root : self
            @sync = parent ? parent.sync_object : Mutex.new
            @previous = previous
            @depth = parent ? parent.depth + 1 : 0
            @max_attempts = max_attempts
            @max_work = max_work
            @attempts = 0
            @work = 0
            @closed = false
            @refused = false
          end
        end

        # @api private
        def owner = @target || self

        # @api private
        def sync_object = @sync

        # A joined handle reports the state of its owner.
        def closed? = owner.raw_closed

        # @api private
        def raw_closed = @closed

        def raw_refused = @refused

        def raw_parent = @parent

        # True when this scope has an owning ancestor (it is not an outermost root).
        def nested? = !owner.raw_parent.nil?

        # True once any charge anywhere in this scope's tree has been refused.
        def refused? = root.raw_refused

        # True for a handle that joined an existing scope instead of owning one.
        def joined? = !@target.nil?

        def attempts = owner.read_counter(:@attempts)

        def work = owner.read_counter(:@work)

        # @api private
        def read_counter(name)
          @sync.synchronize { instance_variable_get(name) }
        end

        # Charges one attempt over a candidate of `elements` elements (at least 1)
        # to this node and every ancestor.  Returns false, charging nothing, when any
        # of them cannot afford it: callers must check this BEFORE building the candidate.
        def charge(elements) = take(elements, 1)

        # Charges a copy that is not itself a property evaluation: work only.
        def charge_copy(elements) = take(elements, 0)

        # True while at least one more attempt of one element could be afforded
        # everywhere on the path.
        def room?
          start = owner
          @sync.synchronize do
            node = start
            while node
              return false if node.attempts_spent >= node.max_attempts || node.work_spent >= node.max_work

              node = node.raw_parent
            end
            true
          end
        end

        # Wraps a property so each evaluation is charged one attempt first (false
        # once refused).
        def charged(falsifies)
          ->(value) { charge(1) && falsifies.call(value) }
        end

        # Ends an owning scope.  The ambient value is restored FIRST so nothing
        # below can leave a stale one.
        def close
          return if @target

          begin
            Fiber[KEY] = @previous
          ensure
            @closed = true
            work, attempts = @sync.synchronize { [@work, @attempts] }
            ShrinkBudget.record(work, attempts)
          end
        end

        # @api private (read under the shared lock by room?/take)
        def attempts_spent = @attempts

        def work_spent = @work

        attr_reader :max_attempts

        attr_reader :max_work

        # @api private
        def add(attempt_cost, cost)
          @attempts += attempt_cost
          @work += cost
        end

        private

        def take(elements, attempt_cost)
          start = owner
          cost = [1, elements].max
          @sync.synchronize do
            node = start
            while node
              # Overflow-safe form: work <= max_work always, so the subtraction cannot go negative.
              if node.attempts_spent >= node.max_attempts || cost > node.max_work - node.work_spent
                start.root.mark_refused
                return false
              end
              node = node.raw_parent
            end
            node = start
            while node
              node.add(attempt_cost, cost)
              node = node.raw_parent
            end
            true
          end
        end

        public

        # @api private
        def mark_refused
          @refused = true
        end
      end
    end
  end
end
