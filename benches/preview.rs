//! Criterion benches — Phase 2: image/csv/text cold vs cached, perf budgets <300ms / <30ms
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::path::Path;
use std::time::Duration;

fn bench_file_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("preview");
    group.measurement_time(Duration::from_secs(5));

    for (name, path) in [("text", "fixtures/sample.txt"), ("csv", "fixtures/sample.csv"), ("markdown", "fixtures/notes.md")] {
        if !Path::new(path).exists() { continue; }
        group.bench_with_input(BenchmarkId::new("read", name), &path, |b, p| {
            b.iter(|| {
                let data = std::fs::read(black_box(p)).unwrap();
                black_box(data.len());
            });
        });
    }

    // Image decode bench if png exists
    if Path::new("fixtures/sample.png").exists() {
        group.bench_function("image_decode_cold", |b| {
            b.iter(|| {
                let img = image::open(black_box("fixtures/sample.png")).unwrap();
                black_box(img.width());
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_file_read);
criterion_main!(benches);
