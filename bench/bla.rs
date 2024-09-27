use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;

fn hashmap_lookup(c: &mut Criterion) {
    let size = 10_000;
    let mut map: HashMap<usize, usize> = HashMap::with_capacity(size);

    for i in 0..size {
        map.insert(i, i);
    }

    c.bench_function("hashmap_lookup", |b| {
        b.iter(|| {
            for i in 0..size {
                black_box(map.get(&i));
            }
        })
    });
}

fn vec_lookup(c: &mut Criterion) {
    let size = 10_000;
    let mut vec: Vec<usize> = Vec::with_capacity(size);

    for i in 0..size {
        vec.push(i);
    }

    c.bench_function("vec_lookup", |b| {
        b.iter(|| {
            for i in 0..size {
                black_box(vec.iter().find(|&&x| x == i));
            }
        })
    });
}

criterion_group!(benches, hashmap_lookup, vec_lookup);
criterion_main!(benches);

