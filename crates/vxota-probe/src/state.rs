//! Live/aggregate state for repeated fetch attempts — same shape as
//! vxprobe's ProbeState: atomics for the hot counters, a histogram for the
//! metric that actually needs percentiles (here: delivery latency, in ms —
//! OTA operates on a seconds scale, unlike vxprobe's RTT microseconds).

use hdrhistogram::Histogram;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use vxota_common::prom;

pub struct OtaState {
    attempts: AtomicU64,
    succeeded: AtomicU64,
    failed: AtomicU64,
    histogram: Mutex<Histogram<u64>>,
}

impl OtaState {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            attempts: AtomicU64::new(0),
            succeeded: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            // 1 ms .. 1 h, 3 significant figures — plenty for OTA-scale latency.
            histogram: Mutex::new(
                Histogram::new_with_bounds(1, 3_600_000, 3).expect("valid histogram bounds"),
            ),
        })
    }

    fn lock_histogram(&self) -> std::sync::MutexGuard<'_, Histogram<u64>> {
        self.histogram.lock().expect("histogram lock poisoned")
    }

    /// Record a successful fetch's delivery latency. A failed attempt isn't
    /// a "0ms delivery" — it's a non-delivery — so failures go through
    /// `record_failure` instead and never touch the latency histogram.
    pub fn record_success(&self, duration: Duration) {
        self.attempts.fetch_add(1, Ordering::Relaxed);
        self.succeeded.fetch_add(1, Ordering::Relaxed);
        let ms = duration.as_millis().max(1) as u64;
        self.lock_histogram().record(ms).ok();
    }

    pub fn record_failure(&self) {
        self.attempts.fetch_add(1, Ordering::Relaxed);
        self.failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Prometheus text exposition of the live state.
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(512);
        prom::line(
            &mut out,
            "vxota_deployments_total",
            &[("status", "success")],
            self.succeeded.load(Ordering::Relaxed) as f64,
        );
        prom::line(
            &mut out,
            "vxota_deployments_total",
            &[("status", "failure")],
            self.failed.load(Ordering::Relaxed) as f64,
        );
        let h = self.lock_histogram();
        if !h.is_empty() {
            for (q, label) in [(0.50, "0.5"), (0.95, "0.95"), (0.99, "0.99")] {
                prom::line(
                    &mut out,
                    "vxota_delivery_latency_seconds",
                    &[("quantile", label)],
                    h.value_at_quantile(q) as f64 / 1000.0,
                );
            }
            prom::line(
                &mut out,
                "vxota_delivery_latency_seconds_mean",
                &[],
                h.mean() / 1000.0,
            );
        }
        out
    }

    pub fn summary(&self) -> Summary {
        let attempts = self.attempts.load(Ordering::Relaxed);
        let succeeded = self.succeeded.load(Ordering::Relaxed);
        let failed = self.failed.load(Ordering::Relaxed);
        let h = self.lock_histogram();
        Summary {
            attempts,
            succeeded,
            failed,
            success_rate_pct: if attempts > 0 {
                succeeded as f64 * 100.0 / attempts as f64
            } else {
                0.0
            },
            delivery_latency_ms: LatencySummary {
                p50: h.value_at_quantile(0.50) as f64,
                p95: h.value_at_quantile(0.95) as f64,
                p99: h.value_at_quantile(0.99) as f64,
                mean: h.mean(),
                max: h.max() as f64,
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub attempts: u64,
    pub succeeded: u64,
    pub failed: u64,
    pub success_rate_pct: f64,
    pub delivery_latency_ms: LatencySummary,
}

#[derive(Debug, Serialize)]
pub struct LatencySummary {
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub mean: f64,
    pub max: f64,
}
