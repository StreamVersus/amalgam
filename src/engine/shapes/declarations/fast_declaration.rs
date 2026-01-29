use crate::engine::shapes::declarations::intersection::{IDeclaration, Length, Points};
#[derive(Default)]
pub struct FastDeclaration {
}
#[allow(unused_variables)]
impl IDeclaration for FastDeclaration {
    fn length(&self) -> bool {
        false
    }

    fn all_points(&self) -> bool {
        false
    }

    fn closest_point(&self) -> bool {
        false
    }

    fn finalize_declaration(&self, length: Length, points: Points) {}
}