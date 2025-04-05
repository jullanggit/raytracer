use std::slice;

use self::BvhNodeKind::{Branch, Leaf};
use crate::{shapes::Shape, vec3::Vec3};

#[derive(Debug)]
pub struct BvhNode<T: Shape> {
    kind: BvhNodeKind<T>,
    // aabb
    min: Vec3,
    max: Vec3,
}

impl<T: Shape> BvhNode<T> {
    pub fn new(shapes: Box<[T]>) -> Self {
        // init root node
        let mut root = Self {
            kind: Leaf { shapes },
            min: Vec3::splat(f32::NEG_INFINITY),
            max: Vec3::splat(f32::INFINITY),
        };

        root.update_bounds();

        root.subdivide();

        todo!()
    }
    fn update_bounds(&mut self) {
        match self.kind {
            Leaf { ref shapes } => {
                for shape in shapes {
                    let (min, max) = (shape.min(), shape.max());

                    self.min = self.min.min(min);
                    self.max = self.max.max(max);
                }
            }
            Branch { .. } => unreachable!(),
        }
    }
    fn subdivide(&mut self) {
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
            Leaf { ref mut shapes } => {
                let partition_point = shapes
                    .iter_mut()
                    .partition_in_place(|shape| shape.centroid().get(axis) > split);

                // abort if one side is empty
                if partition_point == 0 || partition_point == shapes.len() {
                    return;
                }

                // split box
                let (shapes_a, shapes_b) = {
                    // take ownership of shapes
                    let shapes = std::mem::replace(shapes, Box::new([]));

                    let len = shapes.len();
                    let ptr = Box::into_raw(shapes).cast::<T>();

                    #[expect(clippy::multiple_unsafe_ops_per_block)]
                    // SAFETY: We correctly calculate the slices and wrap them into Boxes
                    unsafe {
                        let a = Box::from_raw(slice::from_raw_parts_mut(ptr, partition_point));
                        let b = Box::from_raw(slice::from_raw_parts_mut(
                            ptr.add(partition_point),
                            len - partition_point,
                        ));

                        (a, b)
                    }
                };

                // whether recursion should continue
                let recurse_a = shapes_a.len() > 2;
                let recurse_b = shapes_b.len() > 2;

                let mut child_a = Self {
                    kind: Leaf { shapes: shapes_a },
                    min: Vec3::splat(f32::NEG_INFINITY),
                    max: Vec3::splat(f32::INFINITY),
                };
                child_a.update_bounds();
                if recurse_a {
                    child_a.subdivide();
                }

                let mut child_b = Self {
                    kind: Leaf { shapes: shapes_b },
                    min: Vec3::splat(f32::NEG_INFINITY),
                    max: Vec3::splat(f32::INFINITY),
                };
                child_b.update_bounds();
                if recurse_b {
                    child_b.subdivide();
                }

                self.kind = Branch {
                    children: Box::new([child_a, child_b]),
                };
            }
            Branch { .. } => unreachable!(),
        }
    }
}

#[derive(Debug)]
enum BvhNodeKind<T: Shape> {
    Branch { children: Box<[BvhNode<T>; 2]> },
    Leaf { shapes: Box<[T]> },
}
