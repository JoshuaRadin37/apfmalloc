use criterion::{ criterion_group, criterion_main, BenchmarkId, Criterion, Throughput };
use lrmalloc_rs::apf::trace::{ Trace, Event };
use lrmalloc_rs::apf::histogram::Histogram;
use lrmalloc_rs::apf::liveness_counter::LivenessCounter;
use lrmalloc_rs::apf::reuse_counter::ReuseCounter;
use lrmalloc_rs::apf::ApfTuner;

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

fn liveness_counter_alloc(c: &mut Criterion) {
    let mut lc = LivenessCounter::new();
    c.bench_function("lc alloc", |b| {
        b.iter(|| {
            lc.inc_timer();
            lc.alloc();
        });
    });
}

fn liveness_counter_free(c: &mut Criterion) {
    let mut lc = LivenessCounter::new();
    c.bench_function("lc free", |b| {
        b.iter(|| {
            lc.free();
        });
    });
}

fn reuse_counter_alloc(c: &mut Criterion) {
    let mut rc = ReuseCounter::new(20000, 40000);
    c.bench_function("rc alloc", |b| {
        b.iter(|| {
            rc.alloc(0);
            rc.inc_timer();
        });
    });
}

fn reuse_counter_free(c: &mut Criterion) {
    let mut rc = ReuseCounter::new(20000, 40000);
    c.bench_function("rc free", |b| {
        b.iter(|| {
            rc.free(0);
        });
    });
}

fn check(id: usize) -> u32 {
    0
}

fn get(id: usize, val: usize) -> bool {
    true
}

fn ret(id: usize, val: u32) -> bool {
    true
}

fn apf_tuner_alloc(c: &mut Criterion) {
    let mut apf = ApfTuner::new(0, check, get, ret, false);

    c.bench_function("apf alloc", |b| {
        b.iter(|| {
            apf.malloc(0 as *mut u8);
        });
    });
}

fn apf_tuner_free(c: &mut Criterion) {
    let mut apf = ApfTuner::new(0, check, get, ret, false);
    apf.malloc(0 as *mut u8);

    c.bench_function("apf free", |b| {
        b.iter(|| {
            apf.free(0 as *mut u8);
        });
    });
}

criterion_group!(
    apf_functions,
    trace_add,
    histogram_inc,
    histogram_add,
    liveness_counter_alloc,
    liveness_counter_free,
    reuse_counter_alloc,
    reuse_counter_free,
    apf_tuner_alloc,
    apf_tuner_free
);
criterion_main!(apf_functions);
