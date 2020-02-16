use cow_utils::CowUtils;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn bench_replace(c: &mut Criterion) {
    let input = "a".repeat(40);

    let mut g = c.benchmark_group(format!("Replace in {:?}", input));
    for params in [("a", ""), ("b", "c"), ("a", "b")].iter() {
        g.bench_with_input(
            BenchmarkId::new("replace", format_args!("{:?}", params)),
            params,
            |b, &(from, to)| b.iter(|| input.replace(from, to)),
        );
        g.bench_with_input(
            BenchmarkId::new("cow_replace", format_args!("{:?}", params)),
            params,
            |b, &(from, to)| b.iter(|| input.cow_replace(from, to)),
        );
    }
    g.finish();
}

criterion_group!(benches, bench_replace);
criterion_main!(benches);
