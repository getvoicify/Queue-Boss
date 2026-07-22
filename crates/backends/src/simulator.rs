//! Lazy synthetic queue. Every job's whole lifecycle is derived deterministically
//! from `(seed, index)`; the *current* state is read off that schedule at the
//! injected clock's `now_ms()`. No mutable store, no background threads — the
//! same `(epoch, seed, now)` always yields the same picture.

use qb_core::{JobId, JobState, JobSummary, TimelineEvent};

const CONTINUOUS_QUEUES: [&str; 3] = ["emails", "webhooks", "reports"];
const BATCH_QUEUE: &str = "batch";
/// Cadence of the synthetic producer for the continuous queues.
const PRODUCE_INTERVAL_MS: u64 = 200;
/// Most-recent continuous jobs kept visible (older ones age out, bounding the app).
const VISIBLE_WINDOW: u64 = 128;
/// Fixed backlog created at the epoch that drains and then sits idle.
const BATCH_SIZE: u64 = 16;
const MAX_ATTEMPTS: u32 = 4;

/// SplitMix64 finalizer — decorrelates the LCG seed per job.
fn splitmix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A tiny linear-congruential stream (Knuth MMIX constants). No `rand` dep.
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }

    /// A value in `[lo, hi)`; `hi` must be `> lo`.
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next_u64() % (hi - lo)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fate {
    Complete,
    RetryComplete,
    DeadLetter,
    Cancel,
}

fn fate_for(slot: u64) -> Fate {
    match slot % 8 {
        0..=3 => Fate::Complete,
        4 | 7 => Fate::RetryComplete,
        5 => Fate::DeadLetter,
        _ => Fate::Cancel,
    }
}

/// A single simulated job: its full ordered lifecycle plus static metadata.
pub(crate) struct Job {
    id: JobId,
    queue: String,
    index: u64,
    created_at: u64,
    priority: i32,
    events: Vec<(u64, JobState)>,
}

fn build_job(queue: &str, index: u64, created_at: u64, seed: u64, salt: u64) -> Job {
    let mut rng = Lcg::new(splitmix(
        splitmix(seed).wrapping_add(splitmix(index.wrapping_add(salt.wrapping_mul(0x9E37_79B9)))),
    ));
    let priority = rng.range(0, 3) as i32;
    let wait = rng.range(100, 500);
    let rot = splitmix(seed) % 8;
    let fate = fate_for(index.wrapping_add(rot));

    let mut events = vec![(created_at, JobState::Created)];
    match fate {
        Fate::Complete => {
            let active = created_at + wait;
            events.push((active, JobState::Active));
            events.push((active + rng.range(200, 600), JobState::Completed));
        }
        Fate::Cancel => {
            events.push((created_at + rng.range(120, 400), JobState::Cancelled));
        }
        Fate::RetryComplete => {
            let mut at = created_at + wait;
            events.push((at, JobState::Active));
            at += rng.range(200, 500);
            events.push((at, JobState::Failed));
            at += rng.range(50, 150);
            events.push((at, JobState::Retry));
            at += rng.range(200, 600);
            events.push((at, JobState::Active));
            at += rng.range(200, 500);
            events.push((at, JobState::Completed));
        }
        Fate::DeadLetter => {
            let mut at = created_at + wait;
            for attempt in 0..MAX_ATTEMPTS {
                events.push((at, JobState::Active));
                at += rng.range(150, 400);
                events.push((at, JobState::Failed));
                at += rng.range(40, 120);
                if attempt + 1 < MAX_ATTEMPTS {
                    events.push((at, JobState::Retry));
                    at += rng.range(150, 500);
                } else {
                    events.push((at, JobState::DeadLetter));
                }
            }
        }
    }

    Job {
        id: JobId(format!("{queue}-{index}")),
        queue: queue.to_owned(),
        index,
        created_at,
        priority,
        events,
    }
}

impl Job {
    pub(crate) fn id(&self) -> &JobId {
        &self.id
    }

    pub(crate) fn queue(&self) -> &str {
        &self.queue
    }

    pub(crate) fn index(&self) -> u64 {
        self.index
    }

    pub(crate) fn max_attempts(&self) -> u32 {
        MAX_ATTEMPTS
    }

    fn state_at(&self, now: u64) -> Option<JobState> {
        let mut state = None;
        for (at, s) in &self.events {
            if *at <= now {
                state = Some(*s);
            } else {
                break;
            }
        }
        state
    }

    pub(crate) fn timeline_at(&self, now: u64) -> Vec<TimelineEvent> {
        self.events
            .iter()
            .filter(|(at, _)| *at <= now)
            .map(|(at, state)| TimelineEvent {
                at: *at,
                state: *state,
            })
            .collect()
    }

    /// The first scheduled `Active` transition strictly after `now`, if any —
    /// the next retry attempt for a job currently backing off.
    pub(crate) fn next_active_after(&self, now: u64) -> Option<u64> {
        self.events
            .iter()
            .find(|(at, state)| *state == JobState::Active && *at > now)
            .map(|(at, _)| *at)
    }

    pub(crate) fn summary_at(&self, now: u64) -> Option<JobSummary> {
        let state = self.state_at(now)?;
        let started_at = self
            .events
            .iter()
            .find(|(at, s)| *s == JobState::Active && *at <= now)
            .map(|(at, _)| *at);
        let completed_at = self
            .events
            .iter()
            .find(|(at, s)| *s == JobState::Completed && *at <= now)
            .map(|(at, _)| *at);
        let attempts = self
            .events
            .iter()
            .filter(|(at, s)| *s == JobState::Active && *at <= now)
            .count() as u32;
        Some(JobSummary {
            id: self.id.clone(),
            queue: self.queue.clone(),
            state,
            created_at: self.created_at,
            started_at,
            completed_at,
            attempts,
            priority: self.priority,
        })
    }
}

/// Produces the set of jobs visible at a given instant.
pub(crate) struct Simulator {
    epoch: u64,
    seed: u64,
}

impl Simulator {
    pub(crate) fn new(epoch: u64, seed: u64) -> Self {
        Self { epoch, seed }
    }

    fn continuous_jobs(&self, now: u64) -> Vec<Job> {
        if now < self.epoch {
            return Vec::new();
        }
        let latest = (now - self.epoch) / PRODUCE_INTERVAL_MS;
        let start = latest.saturating_sub(VISIBLE_WINDOW - 1);
        (start..=latest)
            .map(|i| {
                let queue = CONTINUOUS_QUEUES[(i % CONTINUOUS_QUEUES.len() as u64) as usize];
                let created_at = self.epoch + i * PRODUCE_INTERVAL_MS;
                build_job(queue, i, created_at, self.seed, 0)
            })
            .collect()
    }

    fn batch_jobs(&self) -> Vec<Job> {
        (0..BATCH_SIZE)
            .map(|i| build_job(BATCH_QUEUE, i, self.epoch, self.seed, 1))
            .collect()
    }

    /// Every job that exists at `now`, still carrying its full schedule.
    pub(crate) fn jobs_at(&self, now: u64) -> Vec<Job> {
        let mut jobs = self.continuous_jobs(now);
        jobs.extend(self.batch_jobs());
        jobs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn all_states() -> BTreeSet<JobState> {
        BTreeSet::from([
            JobState::Created,
            JobState::Active,
            JobState::Completed,
            JobState::Failed,
            JobState::Cancelled,
            JobState::Retry,
            JobState::DeadLetter,
        ])
    }

    #[test]
    fn every_lifecycle_state_is_reachable_over_the_batch() {
        let sim = Simulator::new(0, 0xABCD);
        let mut seen = BTreeSet::new();
        for job in sim.batch_jobs() {
            for (_, state) in &job.events {
                seen.insert(*state);
            }
        }
        assert_eq!(seen, all_states(), "batch must exercise all seven states");
    }

    #[test]
    fn all_seven_states_hold_for_any_seed() {
        for seed in [0u64, 1, 7, 42, 0xDEAD_BEEF, u64::MAX] {
            let sim = Simulator::new(1_000, seed);
            let mut seen = BTreeSet::new();
            for job in sim.batch_jobs() {
                for (_, state) in &job.events {
                    seen.insert(*state);
                }
            }
            assert_eq!(seen, all_states(), "seed {seed} failed full coverage");
        }
    }

    #[test]
    fn schedules_are_ordered_and_start_at_created() {
        let sim = Simulator::new(500, 99);
        for job in sim.jobs_at(500 + 5_000) {
            assert_eq!(job.events[0].1, JobState::Created);
            assert_eq!(job.events[0].0, job.created_at);
            for pair in job.events.windows(2) {
                assert!(pair[0].0 <= pair[1].0, "events must be time-ordered");
            }
        }
    }

    #[test]
    fn state_is_derived_from_the_clock() {
        let job = build_job("emails", 0, 1_000, 7, 0);
        // Not yet produced.
        assert_eq!(job.state_at(job.events[0].0 - 1), None);
        // At each scheduled instant the current state is that event's state.
        for (at, state) in &job.events {
            assert_eq!(job.state_at(*at), Some(*state));
        }
        // One millisecond before the first transition, still the prior state.
        assert_eq!(
            job.state_at(job.events[1].0 - 1),
            Some(job.events[0].1),
            "state must not advance before its scheduled instant"
        );
    }

    #[test]
    fn identical_seed_and_epoch_yield_identical_schedules() {
        let a = build_job("emails", 3, 1_000, 7, 0);
        let b = build_job("emails", 3, 1_000, 7, 0);
        assert_eq!(a.events, b.events);
        assert_eq!(a.priority, b.priority);
    }

    #[test]
    fn different_seed_changes_the_schedule() {
        let a = build_job("emails", 0, 1_000, 7, 0);
        let b = build_job("emails", 0, 1_000, 8, 0);
        assert_ne!(a.events, b.events, "seed must perturb the timeline");
    }

    #[test]
    fn window_bounds_the_visible_continuous_jobs() {
        let sim = Simulator::new(0, 1);
        // Far in the future: continuous jobs must stay bounded by the window.
        let jobs = sim.jobs_at(10_000_000);
        let continuous = jobs.iter().filter(|j| j.queue() != BATCH_QUEUE).count() as u64;
        assert_eq!(continuous, VISIBLE_WINDOW);
    }

    fn manual_job(events: &[(u64, JobState)]) -> Job {
        Job {
            id: JobId("q-7".to_owned()),
            queue: "q".to_owned(),
            index: 7,
            created_at: events[0].0,
            priority: 2,
            events: events.to_vec(),
        }
    }

    #[test]
    fn summary_tracks_started_completed_and_attempts() {
        let job = manual_job(&[
            (1_000, JobState::Created),
            (1_200, JobState::Active),
            (1_500, JobState::Completed),
        ]);

        let created = job.summary_at(1_000).unwrap();
        assert_eq!(created.state, JobState::Created);
        assert_eq!(created.started_at, None);
        assert_eq!(created.completed_at, None);
        assert_eq!(created.attempts, 0);

        let active = job.summary_at(1_300).unwrap();
        assert_eq!(active.state, JobState::Active);
        assert_eq!(active.started_at, Some(1_200));
        assert_eq!(active.completed_at, None);
        assert_eq!(active.attempts, 1);

        let done = job.summary_at(1_600).unwrap();
        assert_eq!(done.state, JobState::Completed);
        assert_eq!(done.started_at, Some(1_200));
        assert_eq!(done.completed_at, Some(1_500));
        assert_eq!(done.attempts, 1);

        assert!(
            job.summary_at(999).is_none(),
            "not produced before created_at"
        );
    }

    #[test]
    fn attempts_count_every_active_transition() {
        let job = manual_job(&[
            (0, JobState::Created),
            (100, JobState::Active),
            (200, JobState::Failed),
            (300, JobState::Retry),
            (400, JobState::Active),
            (500, JobState::DeadLetter),
        ]);
        assert_eq!(job.summary_at(150).unwrap().attempts, 1);
        assert_eq!(job.summary_at(450).unwrap().attempts, 2);
    }

    #[test]
    fn next_active_after_returns_the_upcoming_attempt() {
        let job = manual_job(&[
            (0, JobState::Created),
            (100, JobState::Active),
            (200, JobState::Failed),
            (300, JobState::Retry),
            (400, JobState::Active),
            (500, JobState::DeadLetter),
        ]);
        assert_eq!(job.next_active_after(50), Some(100));
        assert_eq!(job.next_active_after(250), Some(400));
        assert_eq!(job.next_active_after(450), None);
        // Strictly-after: querying exactly at an Active instant skips it.
        assert_eq!(job.next_active_after(100), Some(400));
    }

    #[test]
    fn timeline_includes_only_events_up_to_now() {
        let job = manual_job(&[
            (1_000, JobState::Created),
            (1_200, JobState::Active),
            (1_500, JobState::Completed),
        ]);
        let timeline = job.timeline_at(1_300);
        assert_eq!(timeline.len(), 2);
        assert_eq!(timeline[0].state, JobState::Created);
        assert_eq!(timeline[0].at, 1_000);
        assert_eq!(timeline[1].state, JobState::Active);
        assert_eq!(timeline[1].at, 1_200);
    }

    #[test]
    fn metadata_accessors_report_index_and_max_attempts() {
        let job = manual_job(&[(0, JobState::Created)]);
        assert_eq!(job.index(), 7);
        assert_eq!(job.max_attempts(), 4);
        assert_eq!(job.id(), &JobId("q-7".to_owned()));
        assert_eq!(job.queue(), "q");
    }

    #[test]
    fn continuous_production_respects_the_epoch() {
        let sim = Simulator::new(1_000, 1);
        assert!(
            sim.continuous_jobs(999).is_empty(),
            "nothing before the epoch"
        );
        assert_eq!(sim.continuous_jobs(1_000).len(), 1, "one job at the epoch");
        assert_eq!(
            sim.continuous_jobs(1_000 + PRODUCE_INTERVAL_MS).len(),
            2,
            "a second job one interval later"
        );
    }

    #[test]
    fn fate_slots_map_to_expected_outcomes() {
        assert_eq!(fate_for(0), Fate::Complete);
        assert_eq!(fate_for(3), Fate::Complete);
        assert_eq!(fate_for(4), Fate::RetryComplete);
        assert_eq!(fate_for(5), Fate::DeadLetter);
        assert_eq!(fate_for(6), Fate::Cancel);
        assert_eq!(fate_for(7), Fate::RetryComplete);
    }
}
