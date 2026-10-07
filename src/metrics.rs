use opentelemetry::global;

pub(crate) fn init_metrics() {
    let meter = global::meter("vetis");
}
