use criterion::measurement::{Measurement, ValueFormatter};
use criterion::Throughput;

pub struct BytesAllocated;
pub struct BytesFormatter;

impl Measurement for BytesAllocated {
    type Intermediate = usize;
    type Value = usize;

    fn start(&self) -> Self::Intermediate {
        0
    }

    fn end(&self, i: Self::Intermediate) -> Self::Value {
        i
    }

    fn add(&self, v1: &Self::Value, v2: &Self::Value) -> Self::Value {
        v1 + v2
    }

    fn zero(&self) -> Self::Value {
        0
    }

    fn to_f64(&self, value: &Self::Value) -> f64 {
        *value as f64
    }

    fn formatter(&self) -> &dyn ValueFormatter {
        &BytesFormatter
    }
}

impl ValueFormatter for BytesFormatter {
    fn scale_values(&self, _typical_value: f64, _values: &mut [f64]) -> &'static str {
        "bytes"
    }

    fn scale_throughputs(
        &self,
        _typical_value: f64,
        _throughput: &Throughput,
        _values: &mut [f64],
    ) -> &'static str {
        "bytes"
    }

    fn scale_for_machines(&self, _values: &mut [f64]) -> &'static str {
        "bytes"
    }
}
