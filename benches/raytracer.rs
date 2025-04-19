use criterion::{Criterion, criterion_group, criterion_main};
use raytracer::{SCENE, config};

fn config_parsing(c: &mut Criterion) {
    let string = "screen(-1 2.5 10, 2 0 0, 0 -2 0, 1000, 1000, 10, 10)
camera(0 1.5 20)
spheres()
planes()
obj((bunny))
triangles()";

    c.bench_function("config_parsing", |b| b.iter(|| config::parse(string)));
}

fn rendering(c: &mut Criterion) {
    let string = "screen(-1.15 1.75 3, 2 0 0, 0 -2 0, 1000, 1000, 10, 10)
camera(0.15 0.75 30)
spheres()
planes()
obj((bunny))
triangles()";

    let scene = SCENE.get_or_init(|| config::parse(string));

    c.bench_function("rendering", |b| b.iter(|| scene.render()));
}

fn incremental_rendering(c: &mut Criterion) {
    let string = "incremental(2)
screen(-1.15 1.75 3, 2 0 0, 0 -2 0, 1000, 1000, 10, 10)
camera(0.15 0.75 30)
spheres()
planes()
obj((bunny))
triangles()";

    let scene = SCENE.get_or_init(|| config::parse(string));

    c.bench_function("incremental_rendering", |b| b.iter(|| scene.render()));
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = rendering, incremental_rendering
}

criterion_main!(benches);
