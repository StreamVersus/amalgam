use rand::{Rng, RngExt};
use ultraviolet::{f32x8, Vec3, Vec3x8};

pub trait AABB {
    fn box_min(&self) -> &Vec3;
    fn box_max(&self) -> &Vec3;
}

#[derive(Default)]
pub struct SimpleAABox {
    pub box_min: Vec3,
    pub box_max: Vec3,
}

impl AABB for SimpleAABox {
    #[inline(always)]
    fn box_min(&self) -> &Vec3 {
        &self.box_min
    }
    #[inline(always)]
    fn box_max(&self) -> &Vec3 {
        &self.box_max
    }
}

impl SimpleAABox {
    pub fn new(box_min: Vec3, box_max: Vec3) -> Self {
        Self { box_min, box_max }
    }

    pub fn new_rand<R: Rng>(rng: &mut R) -> Self {
        Self::new(
            Vec3::new(rng.random_range(0f32..100f32), rng.random_range(0f32..100f32), rng.random_range(0f32..100f32)),
            Vec3::new(rng.random_range(0f32..100f32), rng.random_range(0f32..100f32), rng.random_range(0f32..100f32))
        )
    }
}

pub struct AABB4(pub Vec3x8);

impl AABB4 {
    pub fn new<A: AABB>(a: &A, b: &A, c: &A, d: &A) -> Self {
        Self(Vec3x8 {
                x: f32x8::new([a.box_min().x, a.box_min().x, b.box_min().x, b.box_min().x, c.box_min().x, c.box_min().x, d.box_min().x, d.box_min().x]),
                y: f32x8::new([a.box_min().y, a.box_min().y, b.box_min().y, b.box_min().y, c.box_min().y, c.box_min().y, d.box_min().y, d.box_min().y]),
                z: f32x8::new([a.box_min().z, a.box_min().z, b.box_min().z, b.box_min().z, c.box_min().z, c.box_min().z, d.box_min().z, d.box_min().z]),
            })
    }

    pub fn from_arr<A: AABB>(arr: [&A; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }
}