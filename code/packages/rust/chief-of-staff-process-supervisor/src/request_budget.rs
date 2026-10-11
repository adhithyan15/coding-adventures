//! Per-agent request rate limits (D18S S-K5, step 6 P2.6b).
//!
//! The supervisor acts on every data-plane request a host sends. A host that
//! loops as fast as it can would otherwise get as much of the daemon's time
//! as it asks for. Each host therefore gets its own token bucket:
//!
//! ```text
//!   tokens  ▲
//!   burst ──┤████████▄▄                    ▄▄████ full again after
//!           │          ▀▀▄▄            ▄▄▀▀       burst / per_second s
//!           │              ▀▀▄▄    ▄▄▀▀
//!       1 ──┤                  ▀▀▀▀   ← below one token: the request is
//!           └──────────────────────────────►      refused, not queued
//!                       time
//! ```
//!
//! Each request takes one token; tokens come back at `per_second`, up to
//! `burst`. The arithmetic is in fixed point, one token being a billion
//! units, so the refill is exact in nanoseconds: `per_second` tokens a
//! second is `per_second` units a nanosecond.

/// How many requests a host may make: `burst` at once, `per_second` after
/// that.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestBudget {
    /// Requests a host may make at once, from a full bucket.
    pub burst: u32,
    /// Requests a second the bucket refills by, up to `burst`.
    pub per_second: u32,
}

impl RequestBudget {
    /// The default: far above a legitimate host (an idle one polls 4 times
    /// a second; a busy turn sends a few dozen requests), and a cap on one
    /// that loops.
    pub const DEFAULT: Self = Self {
        burst: 128,
        per_second: 64,
    };
}

impl Default for RequestBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// One token, in the bucket's fixed-point units.
const TOKEN: u128 = 1_000_000_000;

/// A host's bucket.
#[derive(Clone, Debug)]
pub(crate) struct TokenBucket {
    budget: RequestBudget,
    /// Tokens held, in units of `1 / TOKEN`.
    units: u128,
    /// When `units` was last brought up to date.
    last_ns: Option<u64>,
}

impl TokenBucket {
    /// A full bucket.
    pub(crate) fn new(budget: RequestBudget) -> Self {
        Self {
            budget,
            units: u128::from(budget.burst) * TOKEN,
            last_ns: None,
        }
    }

    /// Take one token for a request that arrived at `now_ns`. `false` means
    /// the request is over the budget and must be refused.
    pub(crate) fn take(&mut self, now_ns: u64) -> bool {
        let capacity = u128::from(self.budget.burst) * TOKEN;
        if let Some(last) = self.last_ns {
            // A clock that went backwards refills nothing.
            let elapsed = u128::from(now_ns.saturating_sub(last));
            self.units =
                capacity
                    .min(self.units.saturating_add(
                        elapsed.saturating_mul(u128::from(self.budget.per_second)),
                    ));
        }
        self.last_ns = Some(match self.last_ns {
            Some(last) => last.max(now_ns),
            None => now_ns,
        });
        if self.units >= TOKEN {
            self.units -= TOKEN;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: u64 = 1_000_000_000;

    fn budget(burst: u32, per_second: u32) -> TokenBucket {
        TokenBucket::new(RequestBudget { burst, per_second })
    }

    #[test]
    fn a_full_bucket_allows_the_burst_then_refuses() {
        let mut bucket = budget(3, 1);
        assert!((0..3).all(|_| bucket.take(SECOND)));
        assert!(!bucket.take(SECOND));
        assert!(!bucket.take(SECOND));
    }

    #[test]
    fn tokens_come_back_at_the_refill_rate() {
        let mut bucket = budget(2, 4);
        assert!(bucket.take(0) && bucket.take(0));
        assert!(!bucket.take(0));
        // A quarter second at 4 a second is exactly one token.
        assert!(!bucket.take(SECOND / 4 - 1));
        assert!(bucket.take(SECOND / 4));
        assert!(!bucket.take(SECOND / 4));
    }

    #[test]
    fn the_refill_never_exceeds_the_burst() {
        let mut bucket = budget(2, 1000);
        assert!(bucket.take(0));
        // An hour idle: still only two tokens.
        let later = 3600 * SECOND;
        assert!(bucket.take(later) && bucket.take(later));
        assert!(!bucket.take(later));
    }

    #[test]
    fn a_clock_going_backwards_refills_nothing() {
        let mut bucket = budget(1, 1);
        assert!(bucket.take(10 * SECOND));
        assert!(!bucket.take(SECOND));
        // And the earlier reading does not count as a refill point either.
        assert!(!bucket.take(10 * SECOND + SECOND / 2));
        assert!(bucket.take(11 * SECOND));
    }

    #[test]
    fn a_host_looping_flat_out_is_held_to_the_refill_rate() {
        // 10,000 requests over one simulated second, evenly spaced, against
        // the default budget: the burst plus one second of refill.
        let mut bucket = TokenBucket::new(RequestBudget::DEFAULT);
        let allowed = (0..10_000u64)
            .filter(|index| bucket.take(index * SECOND / 10_000))
            .count();
        let expected = (RequestBudget::DEFAULT.burst + RequestBudget::DEFAULT.per_second) as usize;
        assert!(
            allowed.abs_diff(expected) <= 1,
            "allowed {allowed}, expected about {expected}"
        );
    }

    #[test]
    fn extreme_values_do_not_overflow() {
        let mut bucket = budget(u32::MAX, u32::MAX);
        assert!(bucket.take(0));
        assert!(bucket.take(u64::MAX));
        let mut empty = budget(0, 0);
        assert!(!empty.take(u64::MAX));
    }
}
