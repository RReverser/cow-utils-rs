use cow_utils::CowUtils;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn bench_replace(c: &mut Criterion) {
    let input = "a".repeat(40);

    let mut g = c.benchmark_group(format!("Replace in {:?}", input));
    for params in &[("a", ""), ("b", "c"), ("a", "b")] {
        g.bench_with_input(
            BenchmarkId::new("replace", format_args!("{params:?}")),
            params,
            |b, (from, to)| b.iter(|| input.replace(from, to)),
        );
        g.bench_with_input(
            BenchmarkId::new("cow_replace", format_args!("{params:?}")),
            params,
            |b, (from, to)| b.iter(|| input.cow_replace(from, to)),
        );
    }
    g.finish();
}

fn bench_to_lowercase(c: &mut Criterion) {
    let mut g = c.benchmark_group("To Lowercase");
    for (name, ref input) in [
        ("Ax40", "A".repeat(40)),
        ("ax40", "a".repeat(40)),
        ("ax20 + Ax20", "a".repeat(20) + &"A".repeat(20)),
    ] {
        g.bench_with_input(BenchmarkId::new("to_lowercase", name), input, |b, input| {
            b.iter(|| input.to_lowercase())
        });
        g.bench_with_input(
            BenchmarkId::new("cow_to_lowercase", name),
            input,
            |b, input| b.iter(|| input.cow_to_lowercase()),
        );
    }
    g.finish();
}

criterion_group!(benches, bench_replace, bench_to_lowercase);
criterion_main!(benches);
