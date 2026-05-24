//! Composable stop conditions for LLM agent loops.
//!
//! Build a snapshot of the loop state each iteration, hand it to an
//! [`Evaluator`], and stop as soon as any registered condition fires.
//!
//! # Example
//!
//! ```
//! use std::time::{Duration, Instant};
//! use llm_stop_conditions::{Evaluator, LoopSnapshot, MaxIters, MaxUsd};
//!
//! let evaluator = Evaluator::new()
//!     .with(MaxIters::new(50))
//!     .with(MaxUsd::new(5.00));
//!
//! let snapshot = LoopSnapshot {
//!     iters: 50,
//!     elapsed: Duration::from_secs(1),
//!     tokens_used: 0,
//!     usd_used: 0.0,
//!     last_progress: Some(Instant::now()),
//! };
//!
//! let stop = evaluator.evaluate(&snapshot).expect("should fire");
//! assert_eq!(stop.name, "max_iters");
//! ```
//!
//! Custom predicates plug in via [`Custom`]:
//!
//! ```
//! use std::time::{Duration, Instant};
//! use llm_stop_conditions::{Custom, Evaluator, LoopSnapshot};
//!
//! let evaluator = Evaluator::new().with(Custom::new(
//!     "user_canceled",
//!     |snap: &LoopSnapshot| snap.tokens_used > 1_000_000,
//! ));
//!
//! let snapshot = LoopSnapshot {
//!     iters: 1,
//!     elapsed: Duration::from_millis(10),
//!     tokens_used: 2_000_000,
//!     usd_used: 0.0,
//!     last_progress: None,
//! };
//!
//! assert!(evaluator.evaluate(&snapshot).is_some());
//! ```

use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct LoopSnapshot {
    pub iters: u32,
    pub elapsed: Duration,
    pub tokens_used: u64,
    pub usd_used: f64,
    pub last_progress: Option<Instant>,
}

impl LoopSnapshot {
    pub fn new() -> Self {
        Self {
            iters: 0,
            elapsed: Duration::ZERO,
            tokens_used: 0,
            usd_used: 0.0,
            last_progress: None,
        }
    }
}

impl Default for LoopSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopReason {
    pub name: String,
    pub message: String,
}

impl StopReason {
    pub fn new(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.message)
    }
}

pub trait StopCondition {
    fn should_stop(&self, snapshot: &LoopSnapshot) -> bool;
    fn name(&self) -> &str;
    fn message(&self, snapshot: &LoopSnapshot) -> String {
        let _ = snapshot;
        format!("{} tripped", self.name())
    }
}

#[derive(Debug, Clone)]
pub struct MaxIters {
    limit: u32,
    name: String,
}

impl MaxIters {
    pub fn new(limit: u32) -> Self {
        assert!(limit >= 1, "MaxIters limit must be >= 1");
        Self {
            limit,
            name: "max_iters".to_string(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn limit(&self) -> u32 {
        self.limit
    }
}

impl StopCondition for MaxIters {
    fn should_stop(&self, snapshot: &LoopSnapshot) -> bool {
        snapshot.iters >= self.limit
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn message(&self, snapshot: &LoopSnapshot) -> String {
        format!("iters={} reached limit={}", snapshot.iters, self.limit)
    }
}

#[derive(Debug, Clone)]
pub struct MaxSeconds {
    pub deadline: Instant,
    name: String,
}

impl MaxSeconds {
    pub fn at(deadline: Instant) -> Self {
        Self {
            deadline,
            name: "max_seconds".to_string(),
        }
    }

    pub fn after(duration: Duration) -> Self {
        Self::at(Instant::now() + duration)
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl StopCondition for MaxSeconds {
    fn should_stop(&self, _snapshot: &LoopSnapshot) -> bool {
        Instant::now() >= self.deadline
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn message(&self, snapshot: &LoopSnapshot) -> String {
        format!("elapsed={:.3}s deadline reached", snapshot.elapsed.as_secs_f64())
    }
}

#[derive(Debug, Clone)]
pub struct MaxTokens {
    limit: u64,
    name: String,
}

impl MaxTokens {
    pub fn new(limit: u64) -> Self {
        assert!(limit >= 1, "MaxTokens limit must be >= 1");
        Self {
            limit,
            name: "max_tokens".to_string(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn limit(&self) -> u64 {
        self.limit
    }
}

impl StopCondition for MaxTokens {
    fn should_stop(&self, snapshot: &LoopSnapshot) -> bool {
        snapshot.tokens_used >= self.limit
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn message(&self, snapshot: &LoopSnapshot) -> String {
        format!(
            "tokens_used={} reached limit={}",
            snapshot.tokens_used, self.limit
        )
    }
}

#[derive(Debug, Clone)]
pub struct MaxUsd {
    limit: f64,
    name: String,
}

impl MaxUsd {
    pub fn new(limit: f64) -> Self {
        assert!(
            limit.is_finite() && limit > 0.0,
            "MaxUsd limit must be finite and > 0"
        );
        Self {
            limit,
            name: "max_usd".to_string(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn limit(&self) -> f64 {
        self.limit
    }
}

impl StopCondition for MaxUsd {
    fn should_stop(&self, snapshot: &LoopSnapshot) -> bool {
        snapshot.usd_used >= self.limit
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn message(&self, snapshot: &LoopSnapshot) -> String {
        format!(
            "usd_used={:.4} reached limit={:.4}",
            snapshot.usd_used, self.limit
        )
    }
}

#[derive(Debug, Clone)]
pub struct NoProgress {
    pub since: Instant,
    pub max_idle: Duration,
    name: String,
}

impl NoProgress {
    pub fn new(since: Instant, max_idle: Duration) -> Self {
        Self {
            since,
            max_idle,
            name: "no_progress".to_string(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl StopCondition for NoProgress {
    fn should_stop(&self, snapshot: &LoopSnapshot) -> bool {
        // If snapshot has its own last_progress marker, trust that; otherwise
        // fall back to `self.since`. The window we measure against is always
        // `max_idle`.
        let anchor = snapshot.last_progress.unwrap_or(self.since);
        Instant::now().saturating_duration_since(anchor) >= self.max_idle
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn message(&self, snapshot: &LoopSnapshot) -> String {
        let anchor = snapshot.last_progress.unwrap_or(self.since);
        let idle = Instant::now().saturating_duration_since(anchor);
        format!(
            "idle={:.3}s exceeded max_idle={:.3}s",
            idle.as_secs_f64(),
            self.max_idle.as_secs_f64()
        )
    }
}

pub struct Custom<F>
where
    F: Fn(&LoopSnapshot) -> bool,
{
    pub predicate: F,
    name: String,
}

impl<F> Custom<F>
where
    F: Fn(&LoopSnapshot) -> bool,
{
    pub fn new(name: impl Into<String>, predicate: F) -> Self {
        Self {
            predicate,
            name: name.into(),
        }
    }
}

impl<F> StopCondition for Custom<F>
where
    F: Fn(&LoopSnapshot) -> bool,
{
    fn should_stop(&self, snapshot: &LoopSnapshot) -> bool {
        (self.predicate)(snapshot)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn message(&self, _snapshot: &LoopSnapshot) -> String {
        format!("custom condition '{}' tripped", self.name)
    }
}

pub struct Evaluator {
    conditions: Vec<Box<dyn StopCondition>>,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
        }
    }

    pub fn with<C>(mut self, condition: C) -> Self
    where
        C: StopCondition + 'static,
    {
        self.conditions.push(Box::new(condition));
        self
    }

    pub fn add<C>(&mut self, condition: C) -> &mut Self
    where
        C: StopCondition + 'static,
    {
        self.conditions.push(Box::new(condition));
        self
    }

    pub fn len(&self) -> usize {
        self.conditions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
    }

    pub fn evaluate(&self, snapshot: &LoopSnapshot) -> Option<StopReason> {
        for condition in &self.conditions {
            if condition.should_stop(snapshot) {
                return Some(StopReason::new(
                    condition.name(),
                    condition.message(snapshot),
                ));
            }
        }
        None
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "serde")]
mod serde_impls {
    use super::StopReason;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct StopReasonRepr {
        name: String,
        message: String,
    }

    impl Serialize for StopReason {
        fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
            StopReasonRepr {
                name: self.name.clone(),
                message: self.message.clone(),
            }
            .serialize(ser)
        }
    }

    impl<'de> Deserialize<'de> for StopReason {
        fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
            let repr = StopReasonRepr::deserialize(de)?;
            Ok(StopReason {
                name: repr.name,
                message: repr.message,
            })
        }
    }
}
