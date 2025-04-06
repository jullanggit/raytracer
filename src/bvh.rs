use std::{array, marker::PhantomData, range::Range};

use self::BvhNodeKind::{Branch, Leaf};
use crate::{Ray, shapes::Shape, vec3::Vec3};

#[derive(Debug)]
pub struct BvhNode<T: Shape> {
    kind: BvhNodeKind<T>,
    // aabb
    min: Vec3,
    max: Vec3,
    _type: PhantomData<T>,
}

impl<T: Shape> BvhNode<T> {
    pub fn new(shapes: &mut [T]) -> Self {
        // init root node
        let mut root = Self {
            kind: Leaf {
                shapes_range: Range::from(0..shapes.len().try_into().unwrap()),
            },
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
            _type: PhantomData,
        };

        root.update_bounds(shapes);

        root.subdivide(shapes);

        root
    }
    fn update_bounds(&mut self, shapes: &[T]) {
        match self.kind {
            Leaf { shapes_range } => {
                for index in shapes_range {
                    let (min, max) = (shapes[index as usize].min(), shapes[index as usize].max());

                    self.min = self.min.min(min);
                    self.max = self.max.max(max);
                }
            }
            Branch { .. } => unreachable!(),
        }
    }
    fn subdivide(&mut self, shapes: &mut [T]) {
        let extent = self.max - self.min;

        // get the longest axis
        let axis = {
            let mut axis = u8::from(extent.y > extent.x);
            if extent.z > extent.get(axis) {
                axis = 2;
            }
            axis
        };

        let split = self.min.get(axis) + extent.get(axis) * 0.5;

        match self.kind {
            Leaf { shapes_range } => {
                let partition_point = u32::try_from(
                    shapes[shapes_range.start as usize..shapes_range.end as usize]
                        .iter_mut()
                        .partition_in_place(|shape| shape.centroid().get(axis) > split),
                )
                .unwrap()
                    + shapes_range.start;

                // split box
                let child_ranges = [
                    shapes_range.start..partition_point,
                    partition_point..shapes_range.end,
                ]
                .map(Range::from);

                // assert valid ranges
                for child_range in &child_ranges {
                    debug_assert!(
                        child_range.start <= child_range.end,
                        "parent_range: {shapes_range:?}, partition_point: {partition_point:?} child-ranges: {child_ranges:?}"
                    );
                }

                // whether recursion should continue
                let recurse =
                    child_ranges.map(|range| range.end - range.start > 2 && range != shapes_range);

                let children = array::from_fn(|index| {
                    let mut child = Self {
                        kind: Leaf {
                            shapes_range: child_ranges[index],
                        },
                        min: Vec3::splat(f32::INFINITY),
                        max: Vec3::splat(f32::NEG_INFINITY),
                        _type: PhantomData,
                    };
                    child.update_bounds(shapes);
                    if recurse[index] {
                        child.subdivide(shapes);
                    }

                    child
                });

                self.kind = Branch {
                    children: Box::new(children),
                };
            }
            Branch { .. } => unreachable!(),
        }
    }
    fn intersects(&self, ray: &Ray) -> bool {
        let t1 = (self.min - ray.origin) / *ray.direction.inner();
        let t2 = (self.max - ray.origin) / *ray.direction.inner();

        let tmin = t1.x.min(t2.x).max(t1.y.min(t2.y)).max(t1.z.min(t2.z));
        let tmax = t1.x.max(t2.x).min(t1.y.max(t2.y)).min(t1.z.max(t2.z));

        tmax >= tmin && tmax > 0.
    }

    pub fn items(&self, ray: &Ray, vec: &mut Vec<Range<u32>>) {
        if self.intersects(ray) {
            match self.kind {
                Branch { ref children } => {
                    children[0].items(ray, vec);
                    children[1].items(ray, vec);
                }
                Leaf { shapes_range } => vec.push(shapes_range),
            }
        }
    }
}

#[derive(Debug)]
enum BvhNodeKind<T: Shape> {
    Branch { children: Box<[BvhNode<T>; 2]> },
    Leaf { shapes_range: Range<u32> },
}
