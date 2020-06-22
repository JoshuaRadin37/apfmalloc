use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lrmalloc_rs::apf::trace::{ Trace, Event };
use lrmalloc_rs::apf::histogram::Histogram;

fn trace_add(c: &mut Criterion) {
    let mut t = Trace::new();
    c.bench_function("trace add", |b| {
        b.iter(|| {
            t.add(Event::Alloc(0));
        });
    });
}

fn histogram_inc(c: &mut Criterion) {
    let mut h = Histogram::new();
    c.bench_function("histogram add", |b| {
        b.iter(|| {
            h.increment(3);
        });
    });
}

fn histogram_add(c: &mut Criterion) {
    let mut h = Histogram::new();
    c.bench_function("histogram add", |b| {
        b.iter(|| {
            h.add(3, 10);
        });
    });
}

criterion_group!(
    apf_functions,
    trace_add,
    histogram_inc,
    histogram_add
);
criterion_main!(apf_functions);
