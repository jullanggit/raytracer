use std::{array, cmp::Ordering, collections::BinaryHeap, marker::PhantomData, range::Range};

use self::BvhNodeKind::{Branch, Leaf};
use crate::{
    Ray,
    shapes::{Intersects, Shape},
    vec3::{NormalizedVec3, Vec3},
};

#[derive(Debug)]
pub struct BvhNode<T: Shape> {
    kind: BvhNodeKind,
    // aabb
    min: Vec3,
    max: Vec3,
    _type: PhantomData<T>,
}

impl<T: Shape> Intersects for BvhNode<T> {
    fn intersects(&self, ray: &Ray) -> Option<f32> {
        let t1 = (self.min - ray.origin) / *ray.direction.inner();
        let t2 = (self.max - ray.origin) / *ray.direction.inner();

        let tmin = t1.x.min(t2.x).max(t1.y.min(t2.y)).max(t1.z.min(t2.z));
        let tmax = t1.x.max(t2.x).min(t1.y.max(t2.y)).min(t1.z.max(t2.z));

        (tmax >= tmin && tmax > 0.).then_some(tmin)
    }
}

impl<T: Shape> BvhNode<T> {
    pub fn new(shapes: &mut [T]) -> Vec<Self> {
        // init root node
        let mut nodes = vec![Self {
            kind: Leaf {
                shapes_range: Range::from(0..shapes.len().try_into().unwrap()),
            },
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
            _type: PhantomData,
        }];

        nodes[0].update_bounds(shapes);

        Self::subdivide(0, shapes, &mut nodes);

        nodes
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
    fn subdivide(index: usize, shapes: &mut [T], nodes: &mut Vec<Self>) {
        let extent = nodes[index].max - nodes[index].min;

        // get the longest axis
        let axis = {
            let mut axis = u8::from(extent.y > extent.x);
            if extent.z > extent.get(axis) {
                axis = 2;
            }
            axis
        };

        let split = nodes[index].min.get(axis) + extent.get(axis) * 0.5;

        match nodes[index].kind {
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

                let children = array::from_fn(|child_range_index| {
                    let mut child = Self {
                        kind: Leaf {
                            shapes_range: child_ranges[child_range_index],
                        },
                        min: Vec3::splat(f32::INFINITY),
                        max: Vec3::splat(f32::NEG_INFINITY),
                        _type: PhantomData,
                    };
                    child.update_bounds(shapes);

                    let child_index = nodes.len();

                    nodes.push(child);

                    if recurse[child_range_index] {
                        Self::subdivide(child_index, shapes, nodes);
                    }

                    child_index.try_into().unwrap()
                });

                nodes[index].kind = Branch { children };
            }
            Branch { .. } => unreachable!(),
        }
    }
    /// Returns the closest shape that intersects with the ray, alongside the distance
    pub fn closest_shape(
        ray: &Ray,
        shapes: &[T],
        nodes: &[Self],
        heap: &mut BinaryHeap<HeapEntry>,
    ) -> Option<(f32, Vec3, NormalizedVec3, u16)> {
        heap.clear();

        // stack is ordered from far to near
        heap.push(HeapEntry::new(0., 0));

        let mut closest = None; // (index, time)

        // we always push the closest child last, so node is always the closest node
        while let Some(entry) = heap.pop() {
            let node = &nodes[entry.node_index as usize];

            match node.kind {
                Branch { ref children } => {
                    // push the closer child second. Only push if the child's min is closer than closest
                    match (
                        nodes[children[0] as usize].intersects(ray),
                        nodes[children[1] as usize].intersects(ray),
                    ) {
                        (Some(t0), Some(t1)) => {
                            if let Some((_, closest)) = closest {
                                if t0 <= closest {
                                    heap.push(HeapEntry::new(t0, children[0]));
                                }
                                if t1 <= closest {
                                    heap.push(HeapEntry::new(t1, children[1]));
                                }
                            } else {
                                heap.push(HeapEntry::new(t0, children[0]));
                                heap.push(HeapEntry::new(t1, children[1]));
                            }
                        }
                        (Some(time), None) => {
                            if let Some((_, closest)) = closest {
                                if time <= closest {
                                    heap.push(HeapEntry::new(time, children[0]));
                                }
                            } else {
                                heap.push(HeapEntry::new(time, children[0]));
                            }
                        }
                        (None, Some(time)) => {
                            if let Some((_, closest)) = closest {
                                if time <= closest {
                                    heap.push(HeapEntry::new(time, children[1]));
                                }
                            } else {
                                heap.push(HeapEntry::new(time, children[1]));
                            }
                        }
                        (None, None) => {}
                    }
                }
                Leaf { shapes_range } => {
                    for index in shapes_range {
                        let intersection = shapes[index as usize].intersects(ray);

                        // update if new intersection is closer
                        match (closest, intersection) {
                            // new intersection is closer
                            (Some((_, closest_time)), Some(time)) if time < closest_time => {
                                closest = Some((index, time));
                            }
                            // first intersection
                            (None, Some(time)) => closest = Some((index, time)),
                            _ => {}
                        }
                    }
                }
            }
        }

        closest.map(|(index, time)| {
            let hit_point = ray.origin + *ray.direction.inner() * time;

            (
                time,
                hit_point,
                shapes[index as usize].normal(&hit_point),
                shapes[index as usize].material_index(),
            )
        })
    }
}

#[derive(Debug)]
enum BvhNodeKind {
    Branch { children: [u16; 2] },
    Leaf { shapes_range: Range<u32> },
}

#[derive(PartialEq)]
pub struct HeapEntry {
    tmin: f32,
    node_index: u16,
}

impl HeapEntry {
    const fn new(tmin: f32, node_index: u16) -> Self {
        Self { tmin, node_index }
    }
}

impl Eq for HeapEntry {}

#[expect(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse the ordering so that a smaller tmin is considered "greater"
        other.tmin.partial_cmp(&self.tmin)
    }
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
