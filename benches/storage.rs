use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

use tezedge_debugger::storage::MessageStore;

fn open_database() -> Result<MessageStore, failure::Error> {
    use std::{
        path::{Path, Component},
        sync::Arc,
        io::{Error, ErrorKind},
    };
    use tezedge_debugger::storage::cfs;
    use storage::persistent::open_kv;

    // iterate entries in the directory
    // take only entries that are u128 numbers (timestamp)
    // choose maximal
    let (_, entry) = std::fs::read_dir(Path::new("target/ocaml-shared-data"))?
        .filter_map(|entry| {
            entry.ok().and_then(|entry| {
                let p = entry.path();
                match p.components().last().unwrap() {
                    Component::Normal(name) =>
                        name.to_str().unwrap().parse::<u128>().ok().map(|ts| (ts, entry)),
                    _ => None,
                }
            })
        })
        .max_by(|&(ref ts_left, _), &(ref ts_right, _)| ts_left.cmp(ts_right))
        .ok_or(Error::from(ErrorKind::NotFound))?;

    let schemas = cfs();
    let rocksdb = Arc::new(open_kv(entry.path(), schemas)?);
    Ok(MessageStore::new(rocksdb))
}

fn metric(storage: &MessageStore) {
    use tezedge_debugger::storage::MetricFilters;

    let r = storage.metric().get_cursor(Some(0), 100, MetricFilters::default()).unwrap();
    black_box(r);
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage");

    let storage = open_database().unwrap();

    group.bench_function(BenchmarkId::new("metric", 0), |b| b.iter(|| metric(&storage)));
    group.finish()
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench
);

criterion_main!(benches);
