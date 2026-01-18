use smallvec::SmallVec;
use ultraviolet::Vec3;

pub type Length = f32;
pub type Points = SmallVec<[Vec3; 2]>;

//Defines abstraction for safe and fast intersection checking
pub trait IDeclaration {
    fn length(&self) -> bool;
    fn all_points(&self) -> bool;
    fn closest_point(&self) -> bool;
    fn finalize_declaration(&self, length: Length, points: Points);
}
