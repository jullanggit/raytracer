use self::BvhNodeKind::{Branch, Leaf};
use crate::{
    Ray,
    indices::Indexer,
    shapes::{Intersects, MaterialIndexer, Shape},
    vec3::{New as _, NormalizedVector3, Point, Point3},
};
use std::{array, f32, marker::PhantomData, ptr, range::Range};

type BvhNodeIndexerType = u32;
pub type BvhNodeIndexer<Shape> = Indexer<BvhNodeIndexerType, BvhNode<Shape>>;
type ShapesIndexer<Shape> = Indexer<u32, Shape>;

#[derive(Debug)]
pub struct BvhNode<T: Shape> {
    kind: BvhNodeKind<T>,
    // aabb
    min: Point3,
    max: Point3,
    _type: PhantomData<T>,
}

impl<T: Shape> Intersects for BvhNode<T> {
    #[inline(always)]
    fn intersects(&self, ray: &Ray) -> Option<f32> {
        let t1 = (ray.origin.vector_to(self.min)) / ray.direction.to_vector();
        let t2 = (ray.origin.vector_to(self.max)) / ray.direction.to_vector();

        let tmin = t1
            .min(&t2)
            .into_inner()
            .into_iter()
            .reduce(f32::max)
            .unwrap();
        let tmax = t1
            .max(&t2)
            .into_inner()
            .into_iter()
            .reduce(f32::min)
            .unwrap();

        (tmax >= tmin && tmax > 0.).then_some(tmin)
    }
}

impl<T: Shape> BvhNode<T> {
    #[inline(always)]
    pub fn new(shapes: &mut [T]) -> Vec<Self> {
        let shapes_range =
            Range::from(Indexer::new(0_u32)..Indexer::new(shapes.len().try_into().unwrap()));
        let (min, max) = Self::smallest_bounds(shapes, shapes_range.iter());

        let extent = min.vector_to(max);
        let surface_area =
            2. * (extent.x() * extent.y() + extent.x() * extent.z() + extent.y() * extent.z());

        // init root node
        let mut nodes = vec![Self {
            kind: Leaf { shapes_range },
            min,
            max,
            _type: PhantomData,
        }];

        Self::subdivide(Indexer::new(0), shapes, nodes.as_mut(), surface_area);

        nodes
    }
    #[inline(always)]
    fn smallest_bounds(
        shapes: &[T],
        indices: impl Iterator<Item = ShapesIndexer<T>>,
    ) -> (Point3, Point3) {
        indices.fold(
            (
                Point::new([f32::INFINITY; 3]),
                Point::new([f32::NEG_INFINITY; 3]),
            ),
            |(prev_min, prev_max), index| {
                let (min, max) = (index.index(shapes).min(), index.index(shapes).max());

                (prev_min.min(&min), prev_max.max(&max))
            },
        )
    }
    /// uses surface area heuristic, returns (axis, value, cost, [surface area lt, surface area ge]). Returns `f32::INFINITY` when range is empty
    fn get_split(&self, shapes: &[T], parent_surface_area: f32) -> (u8, f32, f32, [f32; 2]) {
        let BvhNodeKind::Leaf { shapes_range } = self.kind else {
            unreachable!()
        };

        let mut best_split = (0, 0., f32::INFINITY, [f32::NAN, f32::NAN]); // (axis, value, cost, surface areas)

        let extent = self.min.vector_to(self.max);
        let bins_per_axis: u8 = 16;
        let offset_per_bin = extent / f32::from(bins_per_axis);

        // iterate over num_bins shapes, approximately evenly spaced
        for offset_num in 1..bins_per_axis {
            let offsets = self.min + offset_per_bin * f32::from(offset_num);

            for axis in 0..3 {
                let candidate_split = offsets.inner()[axis as usize];

                let [[cost_lt, sa_lt], [cost_ge, sa_ge]] = array::from_fn(|child| {
                    let comparison = if child == 0 { f32::lt } else { f32::ge };

                    let mut num = 0;
                    let indices = shapes_range
                        .iter()
                        .filter(|&index| {
                            comparison(
                                &index.index(shapes).centroid().inner()[axis as usize],
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

                    let extent = min.vector_to(max);

                    let surface_area = 2.
                        * (extent.x() * extent.y()
                            + extent.x() * extent.z()
                            + extent.y() * extent.z());

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

    fn subdivide(
        index: BvhNodeIndexer<T>,
        shapes: &mut [T],
        nodes: &mut Vec<Self>,
        surface_area: f32,
    ) {
        let Leaf { shapes_range } = index.index(nodes).kind else {
            unreachable!()
        };

        let num = shapes_range.end.inner() - shapes_range.start.inner();

        let (axis, split, cost, child_surface_areas) =
            index.index(nodes).get_split(shapes.as_ref(), surface_area);

        // (cost of traversal + child costs) vs leaf cost
        #[expect(clippy::cast_precision_loss)] // should be fine
        if 15. + cost >= 2. * num as f32 {
            return;
        }

        let partition_point = Indexer::new(
            u32::try_from(
                Indexer::<u32, T>::index_range_mut(shapes_range, shapes)
                    .iter_mut()
                    .partition_in_place(|shape| shape.centroid().inner()[axis as usize] < split),
            )
            .unwrap()
                + shapes_range.start.inner(),
        );

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

            let child_index = Indexer::new(nodes.len().try_into().unwrap());

            nodes.push(child);

            Self::subdivide(
                child_index,
                shapes,
                nodes,
                child_surface_areas[child_range_index],
            );

            child_index
        });

        index.index_mut(nodes).kind = Branch { children };
    }
    /// Returns the closest shape that intersects with the ray, alongside the distance
    #[inline(always)]
    pub fn closest_shape(
        ray: &Ray,
        shapes: &[T],
        nodes: &[Self],
        stack: &mut Vec<(f32, BvhNodeIndexerType)>,
    ) -> Option<(f32, Point3, (NormalizedVector3, [f32; 2]), MaterialIndexer)> {
        stack.clear();
        // SAFETY:
        // - Indexer is a repr(transparent) wrapper around IndexerType
        //  -> thus they have the same alignment and size
        // - we clear()'ed, so we dont have to worry about dropping / validity of existing items
        // - we shadow the old `stack`, so it can't be reused
        let stack = unsafe {
            &mut *ptr::from_mut(stack).cast::<Vec<(f32, Indexer<BvhNodeIndexerType, Self>)>>()
        };

        let mut closest_hit = (f32::INFINITY, Indexer::new(u32::MAX)); // distance, shapes_index

        // stack is ordered from far to near
        stack.push((0., Indexer::new(0)));

        // we always push the closest child last, so node is almost always the closest node
        while let Some(entry) = stack.pop() {
            // skip if closest is closer than node
            // there are some edge cases where the entry isnt the closest ones, so we dont just break here
            if closest_hit.0 <= entry.0 {
                continue;
            }

            let node = entry.1.index(nodes);

            match node.kind {
                Branch { children } => {
                    match children.map(|child_node_index| {
                        let child = child_node_index.index(nodes);

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
                        if let Some(time) = index.index(shapes).intersects(ray)
                            && time < closest_hit.0
                        {
                            closest_hit = (time, index);
                        }
                    }
                }
            }
        }

        closest_hit.0.is_finite().then(|| {
            let (time, index) = closest_hit;

            let hit_point = ray.origin + ray.direction.to_vector() * time;

            (
                time,
                hit_point,
                index
                    .index(shapes)
                    .normal_and_texture_coordinates(&hit_point),
                index.index(shapes).material_index(),
            )
        })
    }
}

#[derive(Debug)]
enum BvhNodeKind<T: Shape> {
    Branch {
        children: [BvhNodeIndexer<T>; 2],
    },
    Leaf {
        shapes_range: Range<ShapesIndexer<T>>,
    },
}
