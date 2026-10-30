//! The timer of the speaker view, without the page: elapsed time, time spent on each slide, the
//! schedule (lead or delay on the slides' planned durations, `## Title ⏳95🏃3`) and the rehearsal
//! in progress. Times in ms; `now` comes from the caller.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Timer {
    /// Elapsed time when it last started or stopped
    base: f64,
    /// Running since this instant (ms), or stopped
    since: Option<f64>,
    /// Time spent on each slide (rank in reading order), the current one up to `at.1`
    pub spent: BTreeMap<usize, f64>,
    /// The current slide and the elapsed time when it was last accounted
    pub at: (usize, f64),
    /// A rehearsal is timed, from this slide
    pub rehearsal: Option<usize>,
}

impl Timer {
    /// Stopped at zero, on slide `leaf`
    pub fn new(leaf: usize) -> Timer {
        Timer { base: 0.0, since: None, spent: BTreeMap::new(), at: (leaf, 0.0), rehearsal: None }
    }

    pub fn elapsed(&self, now: f64) -> f64 {
        self.base + self.since.map_or(0.0, |s| (now - s).max(0.0))
    }

    pub fn running(&self) -> bool {
        self.since.is_some()
    }

    /// The time since the last account goes to the current slide; `leaf`: the slide we are on now
    pub fn account(&mut self, now: f64, leaf: usize) {
        let t = self.elapsed(now);
        let (cur, from) = self.at;
        *self.spent.entry(cur).or_default() += (t - from).max(0.0);
        self.at = (leaf, t);
    }

    pub fn start(&mut self, now: f64) {
        if self.since.is_none() {
            self.since = Some(now);
        }
    }

    pub fn stop(&mut self, now: f64) {
        self.account(now, self.at.0);
        self.base = self.elapsed(now);
        self.since = None;
    }

    /// Back to `elapsed`, slides forgotten; running or stopped as asked
    pub fn reset(&mut self, now: f64, leaf: usize, elapsed: f64, running: bool) {
        self.base = elapsed;
        self.since = running.then_some(now);
        self.spent.clear();
        self.at = (leaf, elapsed);
    }

    /// Starts the current slide again: its time is taken off the timer
    pub fn restart_slide(&mut self, now: f64) {
        self.account(now, self.at.0);
        let cur = self.at.0;
        let spent = self.spent.remove(&cur).unwrap_or(0.0);
        self.base = self.elapsed(now) - spent;
        self.since = self.since.map(|_| now);
        self.at.1 = self.base;
    }

    /// Lead (negative) or delay (positive) on the schedule: the elapsed time against the planned
    /// durations of the slides before the current one, plus the current slide's time up to its
    /// own duration. A slide without a planned duration counts for the time spent on it. None if
    /// no slide is timed
    pub fn delta(&self, now: f64, planned: &[Option<f64>]) -> Option<f64> {
        if planned.iter().all(Option::is_none) {
            return None;
        }
        let (cur, from) = self.at;
        let elapsed = self.elapsed(now);
        let spent = |i: usize| self.spent.get(&i).copied().unwrap_or(0.0) + if i == cur { elapsed - from } else { 0.0 };
        let before: f64 = (0..cur).map(|i| planned.get(i).copied().flatten().unwrap_or_else(|| spent(i))).sum();
        let here = spent(cur);
        Some(elapsed - before - planned.get(cur).copied().flatten().map_or(here, |p| here.min(p)))
    }

    /// The elapsed time planned on arriving on slide `leaf`: where "rehearse from here" starts
    pub fn planned_before(leaf: usize, planned: &[Option<f64>]) -> f64 {
        planned.iter().take(leaf).map(|p| p.unwrap_or(0.0)).sum()
    }

    /// Time spent on slide `i`, the current one included up to now
    pub fn spent_on(&self, now: f64, i: usize) -> f64 {
        let extra = if i == self.at.0 { self.elapsed(now) - self.at.1 } else { 0.0 };
        self.spent.get(&i).copied().unwrap_or(0.0) + extra.max(0.0)
    }

    /// Kept by the browser (with `saved`, the wall clock): a reload finds the timer as it was
    pub fn to_json(&self, now: f64) -> serde_json::Value {
        serde_json::json!({
            "elapsed": self.elapsed(now),
            "running": self.running(),
            "at": [self.at.0, self.at.1],
            "spent": self.spent.iter().collect::<Vec<_>>(),
            "rehearsal": self.rehearsal,
        })
    }

    /// The timer kept, `away` ms later (it ran meanwhile if it was running)
    pub fn from_json(v: &serde_json::Value, now: f64, away: f64) -> Option<Timer> {
        let running = v["running"].as_bool()?;
        let elapsed = v["elapsed"].as_f64()? + if running { away.max(0.0) } else { 0.0 };
        let spent = v["spent"].as_array()?.iter().filter_map(|p| Some((p[0].as_u64()? as usize, p[1].as_f64()?))).collect();
        let at = (v["at"][0].as_u64()? as usize, v["at"][1].as_f64()?);
        let rehearsal = v["rehearsal"].as_u64().map(|r| r as usize);
        Some(Timer { base: elapsed, since: running.then_some(now), spent, at, rehearsal })
    }
}

/// `1:05`, `1:02:03`
pub fn short(ms: f64) -> String {
    let s = (ms.max(0.0) / 1000.0).round() as u64;
    if s >= 3600 { format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60) } else { format!("{}:{:02}", s / 60, s % 60) }
}

/// `00:12:34`
pub fn clock(ms: f64) -> String {
    let s = (ms.max(0.0) / 1000.0) as u64;
    format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: f64 = 1000.0;

    #[test]
    fn time_goes_to_the_current_slide_and_stops_when_stopped() {
        let mut t = Timer::new(0);
        t.start(0.0);
        t.account(10.0 * S, 1);
        t.account(25.0 * S, 2);
        t.stop(30.0 * S);
        assert_eq!(t.elapsed(99.0 * S), 30.0 * S);
        assert_eq!(t.spent, BTreeMap::from([(0, 10.0 * S), (1, 15.0 * S), (2, 5.0 * S)]));
        t.start(100.0 * S);
        assert_eq!(t.spent_on(104.0 * S, 2), 9.0 * S);
    }

    #[test]
    fn the_schedule_counts_the_finished_slides() {
        let planned = [Some(20.0 * S), Some(30.0 * S), Some(40.0 * S)];
        let mut t = Timer::new(0);
        t.start(0.0);
        // 5 s late on the first slide: 5 s behind as soon as it is over its duration
        assert_eq!(t.delta(10.0 * S, &planned), Some(0.0));
        assert_eq!(t.delta(25.0 * S, &planned), Some(5.0 * S));
        t.account(25.0 * S, 1);
        // the delay stays while the second slide is under its duration, then grows
        assert_eq!(t.delta(40.0 * S, &planned), Some(5.0 * S));
        assert_eq!(t.delta(60.0 * S, &planned), Some(10.0 * S));
        t.account(60.0 * S, 2);
        // ahead: the third slide left after 10 s of its 40
        t.account(70.0 * S, 3);
        assert_eq!(t.delta(70.0 * S, &planned), Some(-20.0 * S));
        assert_eq!(Timer::new(0).delta(5.0, &[None, None]), None);
    }

    #[test]
    fn a_slide_without_duration_counts_for_its_time() {
        let planned = [Some(20.0 * S), None, Some(30.0 * S)];
        let mut t = Timer::new(0);
        t.start(0.0);
        t.account(20.0 * S, 1);
        t.account(50.0 * S, 2);
        assert_eq!(t.delta(55.0 * S, &planned), Some(0.0));
    }

    #[test]
    fn from_here_and_restart_this_slide() {
        let planned = [Some(20.0 * S), Some(30.0 * S), Some(40.0 * S)];
        let mut t = Timer::new(2);
        t.reset(0.0, 2, Timer::planned_before(2, &planned), true);
        assert_eq!(t.elapsed(10.0 * S), 60.0 * S);
        assert_eq!(t.delta(10.0 * S, &planned), Some(0.0));
        // a false start on slide 2: its 10 s go
        t.restart_slide(10.0 * S);
        assert_eq!(t.elapsed(10.0 * S), 50.0 * S);
        assert_eq!(t.spent_on(10.0 * S, 2), 0.0);
        assert_eq!(t.spent_on(15.0 * S, 2), 5.0 * S);
    }

    #[test]
    fn the_timer_survives_a_reload() {
        let mut t = Timer::new(1);
        t.start(0.0);
        t.rehearsal = Some(1);
        t.account(10.0 * S, 2);
        let kept = t.to_json(12.0 * S);
        let back = Timer::from_json(&kept, 500.0 * S, 3.0 * S).unwrap();
        assert_eq!(back.elapsed(500.0 * S), 15.0 * S);
        assert_eq!((back.at, back.rehearsal, back.spent[&1]), ((2, 10.0 * S), Some(1), 10.0 * S));
        assert_eq!((short(65.0 * S), short(3725.0 * S), clock(3725.0 * S)), ("1:05".into(), "1:02:05".into(), "01:02:05".into()));
    }
}
