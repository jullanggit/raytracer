use std::{marker::PhantomData, range::Range};

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

                for index in 0..2 {
                    let mut child = Self {
                        kind: Leaf {
                            shapes_range: child_ranges[index],
                        },
                        min: Vec3::splat(f32::INFINITY),
                        max: Vec3::splat(f32::NEG_INFINITY),
                        _type: PhantomData,
                    };
                    child.update_bounds(shapes);

                    let child_index = nodes.len();

                    nodes.push(child);

                    if recurse[index] {
                        Self::subdivide(child_index, shapes, nodes);
                    }
                }

                nodes[index].kind = Branch {
                    children: [nodes.len() - 2, nodes.len() - 1]
                        .map(|index| index.try_into().unwrap()),
                };
            }
            Branch { .. } => unreachable!(),
        }
    }
    /// Returns the closest shape that intersects with the ray, alongside the distance
    pub fn closest_shape(
        ray: &Ray,
        shapes: &[T],
        nodes: &[Self],
        stack: &mut Vec<u16>,
    ) -> Option<(f32, Vec3, NormalizedVec3, u16)> {
        stack.clear();

        // stack is ordered from far to near
        stack.push(0);

        let mut closest = None; // (index, time)

        // we always push the closest child last, so node is always the closest node
        while let Some(node_index) = stack.pop() {
            let node = &nodes[node_index as usize];

            match node.kind {
                Branch { ref children } => {
                    // push the closer child second. Only push if the child's min is closer than closest
                    match (
                        nodes[children[0] as usize].intersects(ray),
                        nodes[children[1] as usize].intersects(ray),
                    ) {
                        (Some(t0), Some(t1)) => {
                            let ((closer_child, closer_value), (further_child, further_value)) =
                                if t0 <= t1 {
                                    ((children[0], t0), (children[1], t1))
                                } else {
                                    ((children[1], t1), (children[0], t0))
                                };

                            if let Some((_, closest)) = closest {
                                if further_value <= closest {
                                    stack.push(further_child);
                                }
                                if closer_value <= closest {
                                    stack.push(closer_child);
                                }
                            } else {
                                stack.push(further_child);
                                stack.push(closer_child);
                            }
                        }
                        (Some(time), None) => {
                            if let Some((_, closest)) = closest {
                                if time <= closest {
                                    stack.push(children[0]);
                                }
                            } else {
                                stack.push(children[0]);
                            }
                        }
                        (None, Some(time)) => {
                            if let Some((_, closest)) = closest {
                                if time <= closest {
                                    stack.push(children[1]);
                                }
                            } else {
                                stack.push(children[0]);
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
