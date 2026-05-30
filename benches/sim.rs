use criterion::{BatchSize, BenchmarkGroup, Criterion, PlotConfiguration, criterion_group, criterion_main, measurement::WallTime};
use elevator::{
    Building, policies,
    policy::{Decision, Policy},
    stats::Stats,
    traffic::{Random, Traffic},
};
use std::hint::black_box;

fn bench_for<P: Policy, T: Traffic>(g: &mut BenchmarkGroup<'_, WallTime>, name: &str, until: u64, mut traffic: T) {
    let building = Building::builder().floors(24).elevators(8).build();
    let decision = Decision::new(building.elevators.len());
    let stats = Stats::new(1_000, 0, 1024);

    g.bench_function(&*name, move |b| {
        b.iter_batched(
            || (building.clone(), P::new(&building), stats.clone(), decision.clone()),
            |(mut building, mut policy, mut stats, mut decision): (Building, _, _, _)| {
                black_box(building.run(until, &mut policy, &mut decision, &mut traffic, &mut stats))
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("Sim Speed");
    group.measurement_time(std::time::Duration::from_secs(10));
    group.plot_config(PlotConfiguration::default());
    let traffic = Random::new(24, vec![10.0], vec![10.0], 1.0);
    bench_for::<policies::Simple, Random>(&mut group, "Simple", 10_000_000, traffic.clone());
    group.finish();
}
criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench
);
criterion_main!(benches);
