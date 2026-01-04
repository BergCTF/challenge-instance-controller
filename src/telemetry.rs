use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct Metrics {
    pub reconcile_count: AtomicU64,
    pub reconcile_errors: AtomicU64,
    pub active_instances: AtomicU64,
    pub timeouts: AtomicU64,
}

impl Metrics {
    pub fn record_reconcile(&self) {
        self.reconcile_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.reconcile_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_active_instances(&self) {
        self.active_instances.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decr_active_instances(&self) {
        self.active_instances.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn record_timeout(&self) {
        self.timeouts.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(not(debug_assertions))]
pub fn init() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("berg_operator=info".parse().unwrap())
                .add_directive("kube=info".parse().unwrap()),
        )
        .json()
        .init();
}

#[cfg(debug_assertions)]
pub fn init() {
    tracing_subscriber::fmt()
        .pretty()
        .without_time()
        .with_file(false)
        .with_line_number(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("berg_operator=info".parse().unwrap())
                .add_directive("kube=info".parse().unwrap()),
        )
        .init();
}
