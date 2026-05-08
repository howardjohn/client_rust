use criterion::{black_box, criterion_group, criterion_main, Criterion};
use prometheus_client::metrics::histogram::{
    exponential_buckets, Histogram, NativeHistogramConfig,
};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

const OBSERVATION: f64 = 64.0;
const THREAD_COUNTS: &[usize] = &[2, 4, 8, 16];

pub fn histogram(c: &mut Criterion) {
    let mut group = c.benchmark_group("observe");

    group.bench_function("histogram", |b| {
        let histogram = Histogram::new(exponential_buckets(1.0, 2.0, 10));

        b.iter(|| {
            histogram.observe(black_box(OBSERVATION));
        })
    });

    group.bench_function("native histogram", |b| {
        let histogram = Histogram::new_native(NativeHistogramConfig::with_schema(0));

        b.iter(|| {
            histogram.observe(black_box(OBSERVATION));
        })
    });

    group.bench_function("classic and native histogram", |b| {
        let histogram = Histogram::new_classic_and_native(
            exponential_buckets(1.0, 2.0, 10),
            NativeHistogramConfig::with_schema(0),
        );

        b.iter(|| {
            histogram.observe(black_box(OBSERVATION));
        })
    });

    for threads in THREAD_COUNTS {
        group.bench_function(format!("histogram/{threads} threads"), |b| {
            b.iter_custom(|iters| {
                let histogram = Arc::new(Histogram::new(exponential_buckets(1.0, 2.0, 10)));
                observe_parallel(histogram, *threads, iters)
            })
        });

        group.bench_function(format!("native histogram/{threads} threads"), |b| {
            b.iter_custom(|iters| {
                let histogram =
                    Arc::new(Histogram::new_native(NativeHistogramConfig::with_schema(0)));
                observe_parallel(histogram, *threads, iters)
            })
        });

        group.bench_function(
            format!("classic and native histogram/{threads} threads"),
            |b| {
                b.iter_custom(|iters| {
                    let histogram = Arc::new(Histogram::new_classic_and_native(
                        exponential_buckets(1.0, 2.0, 10),
                        NativeHistogramConfig::with_schema(0),
                    ));
                    observe_parallel(histogram, *threads, iters)
                })
            },
        );
    }

    group.finish();
}

fn observe_parallel(histogram: Arc<Histogram>, threads: usize, iters: u64) -> Duration {
    let barrier = Arc::new(Barrier::new(threads + 1));
    let iterations_per_thread = iters / threads as u64;
    let remainder = iters % threads as u64;

    thread::scope(|scope| {
        let mut handles = Vec::with_capacity(threads);
        for thread_index in 0..threads {
            let histogram = Arc::clone(&histogram);
            let barrier = Arc::clone(&barrier);
            let iterations = iterations_per_thread + u64::from(thread_index == 0) * remainder;

            handles.push(scope.spawn(move || {
                barrier.wait();
                for _ in 0..iterations {
                    histogram.observe(black_box(OBSERVATION));
                }
            }));
        }

        barrier.wait();
        let start = Instant::now();
        for handle in handles {
            handle.join().expect("worker thread should not panic");
        }
        start.elapsed()
    })
}

criterion_group!(benches, histogram);
criterion_main!(benches);
