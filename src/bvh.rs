use std::{array, cmp::Ordering, collections::BinaryHeap, f32, marker::PhantomData, range::Range};

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
    #[inline(always)]
    fn intersects(&self, ray: &Ray) -> Option<f32> {
        let t1 = (self.min - ray.origin) / *ray.direction.inner();
        let t2 = (self.max - ray.origin) / *ray.direction.inner();

        let tmin = t1.x.min(t2.x).max(t1.y.min(t2.y)).max(t1.z.min(t2.z));
        let tmax = t1.x.max(t2.x).min(t1.y.max(t2.y)).min(t1.z.max(t2.z));

        (tmax >= tmin && tmax > 0.).then_some(tmin)
    }
}

impl<T: Shape> BvhNode<T> {
    #[inline(always)]
    pub fn new(shapes: &mut [T]) -> Vec<Self> {
        let shapes_range = Range::from(0..shapes.len().try_into().unwrap());
        let (min, max) = Self::smallest_bounds(shapes, shapes_range.iter());

        // init root node
        let mut nodes = vec![Self {
            kind: Leaf { shapes_range },
            min,
            max,
            _type: PhantomData,
        }];

        Self::subdivide(0, shapes, &mut nodes, f32::INFINITY);

        nodes
    }
    #[inline(always)]
    fn smallest_bounds(shapes: &[T], indices: impl Iterator<Item = u32>) -> (Vec3, Vec3) {
        indices.fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(prev_min, prev_max), index| {
                let (min, max) = (shapes[index as usize].min(), shapes[index as usize].max());

                (prev_min.min(min), prev_max.max(max))
            },
        )
    }
    /// uses surface area heuristic, returns (axis, value, [cost lt, cost ge]). Returns `f32::INFINITY` when range is empty
    fn get_split(&self, shapes: &[T]) -> (u8, f32, [f32; 2]) {
        let BvhNodeKind::Leaf { shapes_range } = self.kind else {
            unreachable!()
        };

        let mut best_split = (0, 0., [f32::INFINITY, f32::INFINITY]); // (axis, value, cost)

        let extent = self.max - self.min;
        let bins_per_axis: u8 = 16;
        let offset_per_bin = extent / f32::from(bins_per_axis);

        // iterate over num_bins shapes, approximately evenly spaced
        for offset_num in 1..bins_per_axis {
            let offsets = self.min + offset_per_bin * offset_num.into();

            for axis in 0..3 {
                let candidate_split = offsets.get(axis);

                let cost = array::from_fn(|child| {
                    let comparison = if child == 0 { f32::lt } else { f32::ge };

                    let mut num = 0;
                    let indices = shapes_range
                        .iter()
                        .filter(|&index| {
                            comparison(
                                &shapes[index as usize].centroid().get(axis),
                                &candidate_split,
                            )
                        })
                        .inspect(|_| {
                            num += 1;
                        });

                    let (min, max) = Self::smallest_bounds(shapes, indices);

                    if num == 0 {
                        return f32::INFINITY;
                    }

                    let extent = max - min;

                    let surface_area =
                        2. * (extent.x * extent.y + extent.x * extent.z + extent.y * extent.z);

                    #[expect(clippy::cast_precision_loss)] // should be fine
                    (surface_area * num as f32)
                });

                if cost[0] + cost[1] < best_split.2[0] + best_split.2[1] {
                    best_split = (axis, candidate_split, cost);
                }
            }
        }

        best_split
    }

    fn subdivide(index: usize, shapes: &mut [T], nodes: &mut Vec<Self>, current_cost: f32) {
        let Leaf { shapes_range } = nodes[index].kind else {
            unreachable!()
        };

        // limit min shapes
        if shapes_range.end - shapes_range.start < 5 {
            return;
        }

        let (axis, split, cost) = nodes[index].get_split(shapes);

        // stop if the cost is more than 90% of the parent cost
        if cost[0] + cost[1] >= (current_cost * 0.9) {
            return;
        }

        let partition_point = u32::try_from(
            shapes[shapes_range.start as usize..shapes_range.end as usize]
                .iter_mut()
                .partition_in_place(|shape| shape.centroid().get(axis) < split),
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

        let children = array::from_fn(|child_range_index| {
            let shapes_range = child_ranges[child_range_index];
            let (min, max) = Self::smallest_bounds(shapes, shapes_range.iter());

            let child = Self {
                kind: Leaf { shapes_range },
                min,
                max,
                _type: PhantomData,
            };

            let child_index = nodes.len();

            nodes.push(child);

            Self::subdivide(child_index, shapes, nodes, cost[child_range_index]);

            child_index.try_into().unwrap()
        });

        nodes[index].kind = Branch { children };
    }
    /// Returns the closest shape that intersects with the ray, alongside the distance
    #[inline(always)]
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

            // break if node isnt closer than closest
            if let Some((_, closest)) = closest
                && closest <= entry.tmin
            {
                break;
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
    Branch { children: [u32; 2] },
    Leaf { shapes_range: Range<u32> },
}

#[derive(PartialEq)]
pub struct HeapEntry {
    tmin: f32,
    node_index: u32,
}

impl HeapEntry {
    const fn new(tmin: f32, node_index: u32) -> Self {
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
