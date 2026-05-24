# llm-stop-conditions

Composable stop conditions for LLM agent loops in Rust.

Every agent loop has a few stop conditions: max iterations, max wall time, max
USD, max tokens, no-progress. This crate ships those as `StopCondition` impls
that share a common `LoopSnapshot` and plug into an `Evaluator` that returns
the first condition that fires.

Zero runtime deps. Optional `serde` feature for `StopReason`.

## Install

```toml
[dependencies]
llm-stop-conditions = "0.1"
```

## Usage

```rust
use std::time::{Duration, Instant};
use llm_stop_conditions::{
    Custom, Evaluator, LoopSnapshot, MaxIters, MaxSeconds, MaxTokens, MaxUsd,
    NoProgress,
};

let started = Instant::now();
let evaluator = Evaluator::new()
    .with(MaxIters::new(50))
    .with(MaxUsd::new(5.00))
    .with(MaxTokens::new(200_000))
    .with(MaxSeconds::after(Duration::from_secs(300)))
    .with(NoProgress::new(started, Duration::from_secs(60)))
    .with(Custom::new("user_canceled", |_s: &LoopSnapshot| false));

let mut snap = LoopSnapshot::new();
snap.iters = 1;
if let Some(reason) = evaluator.evaluate(&snap) {
    eprintln!("stopping: {}", reason);
}
```

`Evaluator::evaluate` returns the first `StopReason` in registration order, or
`None` if every condition passes. Build your own by implementing
`StopCondition`.

## License

MIT.
