use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark_timers(c: &mut Criterion) {
    let mut timers = holani::mikey::timers::Timers::new();
    c.bench_function("timers: tick_all", |b| b.iter(|| timers.tick_all()));
}

fn criterion_benchmark_video(c: &mut Criterion) {
    let mut video = holani::mikey::display::Display::new();

    c.bench_function("video: push_pix_buffer", |b| {
        b.iter(|| video.push_video_dma_burst(&[0, 1, 2, 3, 4, 5, 6, 7]));
    });

    c.bench_function("video: reset", |b| {
        b.iter(|| {
            video.draw_buffer().reset();
        });
    });
}

criterion_group!(
    benches,
    criterion_benchmark_timers,
    criterion_benchmark_video
);
criterion_main!(benches);
