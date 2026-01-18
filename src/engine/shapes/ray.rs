use crate::engine::shapes::declarations::intersection::IDeclaration;
use crate::engine::shapes::utils::{fast_vec3_recip, vecmuladd};
use crate::engine::shapes::AABB::AABB4;
use smallvec::{smallvec, SmallVec};
use ultraviolet::{Vec3, Vec3x8};

pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,

    inv_direction: Vec3x8,
    neg_origin_scaled: Vec3x8,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        let inv_direction = Vec3x8::splat(fast_vec3_recip(direction));
        Self {
            origin,
            direction,
            inv_direction,
            neg_origin_scaled: Vec3x8::splat(-origin) * inv_direction,
        }
    }

    pub fn aabb_intersection<D: IDeclaration + ?Sized>(&self, aabbs: Vec<(&AABB4, &D)>) -> Vec<bool> {
        let mut results = vec![false; aabbs.len() * 4];

        let inv_direction = self.inv_direction;
        let pre = self.neg_origin_scaled;
        aabbs.iter().enumerate().for_each(|(result_idx, (aabb, decl))| {
            let box_vec = aabb.0;

            let ints = vecmuladd(box_vec, inv_direction, pre);
            let vectors: [Vec3; 8] = ints.into();

            vectors.chunks(2).enumerate().for_each(|(i, vec)| {
                let min = vec[0];
                let max = vec[1];

                let t_near = min.min_by_component(max);
                let t_far = min.max_by_component(max);

                let t_enter = t_near.component_max();
                let t_exit = t_far.component_min();

                // Early exit checks
                if t_exit < t_enter || t_exit < 0.0 {
                    return;
                }

                if !(decl.all_points() || decl.closest_point() || decl.length()) {
                    decl.finalize_declaration(0.0, SmallVec::new());
                    results[result_idx * 4 + i] = true;
                    return;
                }

                let t_hit = if t_enter >= 0.0 { t_enter } else { t_exit };
                let length = if decl.length() { t_hit } else { 0.0 };

                let points: SmallVec<[Vec3; 2]> = if decl.all_points() {
                    smallvec![
                        self.direction * t_enter + self.origin,
                        self.direction * t_exit  + self.origin,
                    ]
                } else if decl.closest_point() {
                    smallvec![self.direction * t_hit + self.origin]
                } else {
                    SmallVec::new()
                };

                decl.finalize_declaration(length, points);
                results[result_idx * 4 + i] = true;
            });
        });

        results
    }
}
#[test]
fn test_aabb() {

}