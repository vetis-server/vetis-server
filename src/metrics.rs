use opentelemetry::global;

pub(super) fn init_metrics() {
    let meter = global::meter("vetis");
}
