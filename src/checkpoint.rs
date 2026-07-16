//! 체크포인트 정책 — 언제 봉인 페인트를 데몬에 밀지의 디바운스 상태기계. 출력 이벤트가
//! dirty 를 세우고, 마감 = min(마지막 출력 + idle, 최초 dirty + cap). idle 은 조용해질
//! 때까지 미루고 cap 은 폭주 중에도 상한을 강제한다(값은 SPEC 정책: idle 300ms, cap 5s).
//!
//! 엔진-불가지(시각만 다룬다) — wezterm 엔진 교체와 무관하다.
//!
//! 순수 상태기계(내부 시계 없음 — `now` 를 받는다)라 하니스·sleep 없이 마감 계산과
//! due 전이를 결정적으로 단위 테스트한다. 구동 루프는 tee 출력마다 [`on_output`] 을
//! 부르고 [`deadline`] 까지 기다렸다가 [`is_due`] 면 cold_paint 를 StoreBlob 으로 민다.
//!
//! [`on_output`]: CheckpointPolicy::on_output
//! [`deadline`]: CheckpointPolicy::deadline
//! [`is_due`]: CheckpointPolicy::is_due

use std::time::{Duration, Instant};

/// 출력 이벤트 디바운스: idle 300ms, 상한 5s.
pub const CKPT_IDLE: Duration = Duration::from_millis(300);
pub const CKPT_CAP: Duration = Duration::from_secs(5);

pub struct CheckpointPolicy {
    idle: Duration,
    cap: Duration,
    // 최초 dirty 전이 시각(cap 기준점). None = clean.
    dirty_since: Option<Instant>,
    // 마지막 출력 시각(idle 기준점).
    last_output: Option<Instant>,
}

impl Default for CheckpointPolicy {
    fn default() -> Self {
        Self::with(CKPT_IDLE, CKPT_CAP)
    }
}

impl CheckpointPolicy {
    /// 테스트 주입용 — idle·cap 을 명시한다(기본형은 [`Default`]).
    pub fn with(idle: Duration, cap: Duration) -> Self {
        CheckpointPolicy { idle, cap, dirty_since: None, last_output: None }
    }

    /// 출력 이벤트 — dirty 를 세우고 idle 기준점을 민다. 최초 전이만 cap 기준점을 찍는다.
    pub fn on_output(&mut self, now: Instant) {
        if self.dirty_since.is_none() {
            self.dirty_since = Some(now);
        }
        self.last_output = Some(now);
    }

    /// 현재 dirty 여부.
    pub fn is_dirty(&self) -> bool {
        self.dirty_since.is_some()
    }

    /// 봉인 마감 — dirty 일 때만. min(마지막 출력 + idle, 최초 dirty + cap).
    pub fn deadline(&self) -> Option<Instant> {
        match (self.last_output, self.dirty_since) {
            (Some(last), Some(since)) => Some((last + self.idle).min(since + self.cap)),
            _ => None,
        }
    }

    /// 지금 봉인할 때인가 — dirty 이고 마감 도달.
    pub fn is_due(&self, now: Instant) -> bool {
        self.deadline().map_or(false, |d| now >= d)
    }

    /// 봉인을 밀고 난 뒤 — clean 으로 되돌린다(다음 출력이 새 사이클을 연다).
    pub fn reset(&mut self) {
        self.dirty_since = None;
        self.last_output = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_has_no_deadline_and_is_not_due() {
        let p = CheckpointPolicy::default();
        assert!(!p.is_dirty());
        assert_eq!(p.deadline(), None);
        assert!(!p.is_due(Instant::now()));
    }

    #[test]
    fn idle_deadline_is_last_output_plus_idle() {
        let base = Instant::now();
        let mut p = CheckpointPolicy::with(Duration::from_millis(300), Duration::from_secs(5));
        p.on_output(base);
        assert_eq!(p.deadline(), Some(base + Duration::from_millis(300)));
        assert!(!p.is_due(base + Duration::from_millis(299)));
        assert!(p.is_due(base + Duration::from_millis(300)));
    }

    #[test]
    fn a_later_output_extends_the_idle_window() {
        let base = Instant::now();
        let mut p = CheckpointPolicy::with(Duration::from_millis(300), Duration::from_secs(5));
        p.on_output(base);
        p.on_output(base + Duration::from_millis(200));
        // idle 기준점이 밀렸다 — 마감 = 200ms + 300ms(< cap).
        assert_eq!(p.deadline(), Some(base + Duration::from_millis(500)));
        assert!(!p.is_due(base + Duration::from_millis(499)));
    }

    #[test]
    fn cap_bounds_a_continuous_burst() {
        let base = Instant::now();
        let idle = Duration::from_millis(300);
        let cap = Duration::from_secs(5);
        let mut p = CheckpointPolicy::with(idle, cap);
        p.on_output(base); // dirty_since = base → cap 마감 = base + 5s
                           // 폭주: 100ms 마다 계속 출력 — idle 마감은 계속 밀리지만 cap 이 상한.
        for k in 1..=60u64 {
            p.on_output(base + Duration::from_millis(100 * k));
        }
        // 마지막 출력 = base+6s → idle 마감 = base+6.3s. cap 마감 = base+5s 가 이긴다.
        assert_eq!(p.deadline(), Some(base + cap));
        assert!(p.is_due(base + cap));
    }

    #[test]
    fn reset_returns_to_clean() {
        let base = Instant::now();
        let mut p = CheckpointPolicy::default();
        p.on_output(base);
        assert!(p.is_dirty());
        p.reset();
        assert!(!p.is_dirty());
        assert_eq!(p.deadline(), None);
    }
}
