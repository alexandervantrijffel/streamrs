use criterion::BenchmarkId;
use criterion::{Criterion, criterion_group, criterion_main};

async fn new() {}

// async fn heap(levels: u32) {
//   let library = Box::new(Library::new_sample_library());
//   process_heap(library, levels);
// }

// #[inline]
// fn process_heap(library: Box<Library>, levels: u32) -> Box<Library> {
//   match levels {
//     0 => library,
//     x => process_heap(library, x - 1),
//   }
// }

pub fn criterion_benchmark(c: &mut Criterion) {
  let rt = tokio::runtime::Runtime::new().unwrap();
  let mut group = c.benchmark_group("mygroup");
  group.significance_level(0.1).sample_size(1000);
  group.bench_function(BenchmarkId::new("new", "abc"), |b| {
    b.to_async(&rt).iter(new);
  });
  group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
