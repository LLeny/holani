
use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark_timers(c: &mut Criterion) {
    let mut timers = holani::mikey::timers::Timers::new();
    c.bench_function("timers tick", |b| b.iter(|| timers.tick_all(10)));
}

criterion_group!(benches, criterion_benchmark_timers);
criterion_main!(benches);