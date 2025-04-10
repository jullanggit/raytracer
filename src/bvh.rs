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

            // skip node if it isnt closer than closest
            if let Some((_, closest)) = closest
                && closest <= entry.tmin
            {
                continue;
            }

            match node.kind {
                Branch { children } => {
                    for child_node_index in children {
                        let child = &nodes[child_node_index as usize];

                        if let Some(intersection) = child.intersects(ray) {
                            // push if intersection is closer than closest
                            if let Some((_, closest)) = closest {
                                if intersection < closest {
                                    heap.push(HeapEntry::new(intersection, child_node_index));
                                }
                                // or closest is not yet initialized
                            } else {
                                heap.push(HeapEntry::new(intersection, child_node_index));
                            }
                        }
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
