use std::{array, f32, marker::PhantomData, range::Range};

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

        let extent = max - min;
        let surface_area = 2. * (extent.x * extent.y + extent.x * extent.z + extent.y * extent.z);

        // init root node
        let mut nodes = vec![Self {
            kind: Leaf { shapes_range },
            min,
            max,
            _type: PhantomData,
        }];

        Self::subdivide(0, shapes, &mut nodes, surface_area);

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
    /// uses surface area heuristic, returns (axis, value, cost, [surface area lt, surface area ge]). Returns `f32::INFINITY` when range is empty
    fn get_split(&self, shapes: &[T], parent_surface_area: f32) -> (u8, f32, f32, [f32; 2]) {
        let BvhNodeKind::Leaf { shapes_range } = self.kind else {
            unreachable!()
        };

        let mut best_split = (0, 0., f32::INFINITY, [f32::NAN, f32::NAN]); // (axis, value, cost, surface areas)

        let extent = self.max - self.min;
        let bins_per_axis: u8 = 16;
        let offset_per_bin = extent / f32::from(bins_per_axis);

        // iterate over num_bins shapes, approximately evenly spaced
        for offset_num in 1..bins_per_axis {
            let offsets = self.min + offset_per_bin * offset_num.into();

            for axis in 0..3 {
                let candidate_split = offsets.get(axis);

                let [[cost_lt, sa_lt], [cost_ge, sa_ge]] = array::from_fn(|child| {
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
                        return [f32::INFINITY, f32::NAN];
                    }

                    let extent = max - min;

                    let surface_area =
                        2. * (extent.x * extent.y + extent.x * extent.z + extent.y * extent.z);

                    [
                        #[expect(clippy::cast_precision_loss)] // should be fine
                        ((surface_area / parent_surface_area) * 2. * num as f32),
                        surface_area,
                    ]
                });

                let cost = cost_lt + cost_ge;

                if cost < best_split.2 {
                    best_split = (axis, candidate_split, cost, [sa_lt, sa_ge]);
                }
            }
        }

        best_split
    }

    fn subdivide(index: usize, shapes: &mut [T], nodes: &mut Vec<Self>, surface_area: f32) {
        let Leaf { shapes_range } = nodes[index].kind else {
            unreachable!()
        };

        let num = shapes_range.end - shapes_range.start;

        let (axis, split, cost, child_surface_areas) = nodes[index].get_split(shapes, surface_area);

        // (cost of traversal + child costs) vs leaf cost
        #[expect(clippy::cast_precision_loss)] // should be fine
        if 15. + cost >= 2. * num as f32 {
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

            Self::subdivide(
                child_index,
                shapes,
                nodes,
                child_surface_areas[child_range_index],
            );

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
        stack: &mut Vec<(f32, u32)>,
    ) -> Option<(f32, Vec3, NormalizedVec3, u16)> {
        stack.clear();

        let mut closest_hit = (f32::INFINITY, u32::MAX); // distance, shapes_index

        // stack is ordered from far to near
        stack.push((0., 0));

        // we always push the closest child last, so node is almost always the closest node
        while let Some(entry) = stack.pop() {
            // skip if closest is closer than node
            // there are some edge cases where the entry isnt the closest ones, so we dont just break here
            if closest_hit.0 <= entry.0 {
                continue;
            }

            let node = &nodes[entry.1 as usize];

            match node.kind {
                Branch { children } => {
                    match children.map(|child_node_index| {
                        let child = &nodes[child_node_index as usize];

                        child.intersects(ray).and_then(|intersection| {
                            // push if intersection is closer than closest
                            (intersection < closest_hit.0).then_some(intersection)
                        })
                    }) {
                        [Some(distance_0), Some(distance_1)] => {
                            if distance_0 < distance_1 {
                                // push further child first
                                stack.push((distance_1, children[1]));
                                stack.push((distance_0, children[0]));
                            } else {
                                // push further child first
                                stack.push((distance_0, children[0]));
                                stack.push((distance_1, children[1]));
                            }
                        }
                        [Some(distance), None] => {
                            stack.push((distance, children[0]));
                        }
                        [None, Some(distance)] => {
                            stack.push((distance, children[1]));
                        }
                        [None, None] => {}
                    }
                }
                Leaf { shapes_range } => {
                    for index in shapes_range {
                        if let Some(time) = shapes[index as usize].intersects(ray) {
                            if time < closest_hit.0 {
                                closest_hit = (time, index);
                            }
                        }
                    }
                }
            }
        }

        closest_hit.0.is_finite().then(|| {
            let (time, index) = closest_hit;

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
