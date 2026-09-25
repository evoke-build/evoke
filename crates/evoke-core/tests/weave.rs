//! The weave's schedule as the design words it, checked against the core's own planner and runner over plans no
//! one wrote: `reference` is the design's sentences as code and the fixtures both models draw on; `schedule` runs
//! generated requests through `weave::planning::plan`; `run` drives `weave::running::execute` as the hosts drive
//! it, a proptest state machine, one signal mid-stage among its transitions. Each invariant quotes the sentence of
//! the design it checks — the Weave paragraph, kept here. A failure's seed lands in `proptest-regressions/weave/`.

#[path = "weave/reference.rs"]
mod reference;
#[path = "weave/run.rs"]
mod run;
#[path = "weave/schedule.rs"]
mod schedule;
