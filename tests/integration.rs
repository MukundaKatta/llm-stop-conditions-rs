use std::thread;
use std::time::{Duration, Instant};

use llm_stop_conditions::{
    Custom, Evaluator, LoopSnapshot, MaxIters, MaxSeconds, MaxTokens, MaxUsd, NoProgress,
    StopCondition, StopReason,
};

fn snap() -> LoopSnapshot {
    LoopSnapshot::new()
}

// ---- LoopSnapshot ---------------------------------------------------------

#[test]
fn snapshot_default_is_zeroed() {
    let s = LoopSnapshot::default();
    assert_eq!(s.iters, 0);
    assert_eq!(s.tokens_used, 0);
    assert_eq!(s.usd_used, 0.0);
    assert!(s.last_progress.is_none());
    assert_eq!(s.elapsed, Duration::ZERO);
}

// ---- MaxIters -------------------------------------------------------------

#[test]
fn max_iters_below_limit_does_not_stop() {
    let c = MaxIters::new(5);
    let mut s = snap();
    s.iters = 4;
    assert!(!c.should_stop(&s));
}

#[test]
fn max_iters_at_limit_stops() {
    let c = MaxIters::new(5);
    let mut s = snap();
    s.iters = 5;
    assert!(c.should_stop(&s));
}

#[test]
fn max_iters_above_limit_stops() {
    let c = MaxIters::new(5);
    let mut s = snap();
    s.iters = 9_999;
    assert!(c.should_stop(&s));
}

#[test]
fn max_iters_message_includes_numbers() {
    let c = MaxIters::new(10);
    let mut s = snap();
    s.iters = 10;
    let msg = c.message(&s);
    assert!(msg.contains("10"));
    assert!(msg.contains("limit"));
}

#[test]
#[should_panic(expected = "MaxIters limit must be >= 1")]
fn max_iters_zero_panics() {
    let _ = MaxIters::new(0);
}

#[test]
fn max_iters_custom_name() {
    let c = MaxIters::new(3).with_name("ceiling");
    assert_eq!(c.name(), "ceiling");
}

// ---- MaxTokens ------------------------------------------------------------

#[test]
fn max_tokens_below_limit_does_not_stop() {
    let c = MaxTokens::new(1_000);
    let mut s = snap();
    s.tokens_used = 999;
    assert!(!c.should_stop(&s));
}

#[test]
fn max_tokens_at_or_above_stops() {
    let c = MaxTokens::new(1_000);
    let mut s = snap();
    s.tokens_used = 1_000;
    assert!(c.should_stop(&s));
    s.tokens_used = 5_000;
    assert!(c.should_stop(&s));
}

#[test]
#[should_panic(expected = "MaxTokens limit must be >= 1")]
fn max_tokens_zero_panics() {
    let _ = MaxTokens::new(0);
}

// ---- MaxUsd ---------------------------------------------------------------

#[test]
fn max_usd_below_limit_does_not_stop() {
    let c = MaxUsd::new(5.00);
    let mut s = snap();
    s.usd_used = 4.99;
    assert!(!c.should_stop(&s));
}

#[test]
fn max_usd_at_or_above_stops() {
    let c = MaxUsd::new(5.00);
    let mut s = snap();
    s.usd_used = 5.00;
    assert!(c.should_stop(&s));
    s.usd_used = 9.99;
    assert!(c.should_stop(&s));
}

#[test]
#[should_panic(expected = "MaxUsd limit must be finite and > 0")]
fn max_usd_zero_panics() {
    let _ = MaxUsd::new(0.0);
}

#[test]
#[should_panic(expected = "MaxUsd limit must be finite and > 0")]
fn max_usd_negative_panics() {
    let _ = MaxUsd::new(-1.0);
}

#[test]
#[should_panic(expected = "MaxUsd limit must be finite and > 0")]
fn max_usd_nan_panics() {
    let _ = MaxUsd::new(f64::NAN);
}

#[test]
fn max_usd_message_formatting() {
    let c = MaxUsd::new(5.0);
    let mut s = snap();
    s.usd_used = 5.5;
    let msg = c.message(&s);
    assert!(msg.contains("5.5") || msg.contains("5.5000"));
}

// ---- MaxSeconds -----------------------------------------------------------

#[test]
fn max_seconds_before_deadline_does_not_stop() {
    let c = MaxSeconds::after(Duration::from_secs(60));
    assert!(!c.should_stop(&snap()));
}

#[test]
fn max_seconds_after_deadline_stops() {
    let c = MaxSeconds::at(Instant::now() - Duration::from_millis(1));
    assert!(c.should_stop(&snap()));
}

#[test]
fn max_seconds_real_wait() {
    let c = MaxSeconds::after(Duration::from_millis(20));
    assert!(!c.should_stop(&snap()));
    thread::sleep(Duration::from_millis(40));
    assert!(c.should_stop(&snap()));
}

// ---- NoProgress -----------------------------------------------------------

#[test]
fn no_progress_within_window_does_not_stop() {
    let now = Instant::now();
    let c = NoProgress::new(now, Duration::from_secs(60));
    let mut s = snap();
    s.last_progress = Some(now);
    assert!(!c.should_stop(&s));
}

#[test]
fn no_progress_past_window_stops() {
    let past = Instant::now() - Duration::from_millis(500);
    let c = NoProgress::new(past, Duration::from_millis(100));
    let mut s = snap();
    s.last_progress = Some(past);
    assert!(c.should_stop(&s));
}

#[test]
fn no_progress_uses_snapshot_marker_when_present() {
    // Anchor on `since` would not have tripped, but snapshot's
    // `last_progress` is older, so it should trip.
    let fresh = Instant::now();
    let stale = fresh - Duration::from_secs(10);
    let c = NoProgress::new(fresh, Duration::from_secs(1));
    let mut s = snap();
    s.last_progress = Some(stale);
    assert!(c.should_stop(&s));
}

#[test]
fn no_progress_falls_back_to_since() {
    let stale = Instant::now() - Duration::from_secs(5);
    let c = NoProgress::new(stale, Duration::from_secs(1));
    // No `last_progress` in snapshot, must fall back to `since`.
    assert!(c.should_stop(&snap()));
}

// ---- Custom ---------------------------------------------------------------

#[test]
fn custom_predicate_can_trip() {
    let c = Custom::new("cancel", |s: &LoopSnapshot| s.iters > 3);
    let mut s = snap();
    s.iters = 5;
    assert!(c.should_stop(&s));
}

#[test]
fn custom_predicate_can_decline() {
    let c = Custom::new("cancel", |_: &LoopSnapshot| false);
    assert!(!c.should_stop(&snap()));
}

#[test]
fn custom_message_includes_name() {
    let c = Custom::new("user_canceled", |_: &LoopSnapshot| true);
    assert!(c.message(&snap()).contains("user_canceled"));
}

// ---- Evaluator ------------------------------------------------------------

#[test]
fn evaluator_returns_none_when_empty() {
    let e = Evaluator::new();
    assert!(e.evaluate(&snap()).is_none());
    assert!(e.is_empty());
    assert_eq!(e.len(), 0);
}

#[test]
fn evaluator_returns_first_match_in_order() {
    let e = Evaluator::new()
        .with(MaxIters::new(10))
        .with(MaxUsd::new(5.0));
    let mut s = snap();
    s.iters = 100;
    s.usd_used = 100.0;
    let r = e.evaluate(&s).unwrap();
    assert_eq!(r.name, "max_iters");
}

#[test]
fn evaluator_falls_through_to_later_condition() {
    let e = Evaluator::new()
        .with(MaxIters::new(10))
        .with(MaxUsd::new(5.0));
    let mut s = snap();
    s.iters = 1;
    s.usd_used = 10.0;
    let r = e.evaluate(&s).unwrap();
    assert_eq!(r.name, "max_usd");
}

#[test]
fn evaluator_none_when_all_pass() {
    let e = Evaluator::new()
        .with(MaxIters::new(10))
        .with(MaxUsd::new(5.0))
        .with(MaxTokens::new(100));
    assert!(e.evaluate(&snap()).is_none());
}

#[test]
fn evaluator_add_grows_len() {
    let mut e = Evaluator::new();
    e.add(MaxIters::new(1));
    e.add(MaxUsd::new(1.0));
    assert_eq!(e.len(), 2);
}

#[test]
fn evaluator_mixes_builtins_and_custom() {
    let e = Evaluator::new()
        .with(MaxIters::new(1_000_000))
        .with(Custom::new("flag", |s: &LoopSnapshot| s.tokens_used > 42));
    let mut s = snap();
    s.tokens_used = 100;
    let r = e.evaluate(&s).unwrap();
    assert_eq!(r.name, "flag");
}

#[test]
fn evaluator_reports_message() {
    let e = Evaluator::new().with(MaxIters::new(2));
    let mut s = snap();
    s.iters = 2;
    let r = e.evaluate(&s).unwrap();
    assert!(r.message.contains("iters=2"));
    assert!(r.message.contains("limit=2"));
}

// ---- StopReason -----------------------------------------------------------

#[test]
fn stop_reason_display() {
    let r = StopReason::new("foo", "bar");
    assert_eq!(format!("{}", r), "foo: bar");
}

#[test]
fn stop_reason_equality() {
    let a = StopReason::new("foo", "bar");
    let b = StopReason::new("foo", "bar");
    let c = StopReason::new("foo", "baz");
    assert_eq!(a, b);
    assert_ne!(a, c);
}
