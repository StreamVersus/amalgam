use crate::engine::shapes::ray::Ray;
use ultraviolet::{Mat3, Mat4, Rotor3, Vec3, Vec4};

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: Vec3,
    pub pitch: f32,
    pub yaw: f32,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near_plane: f32,
    pub far_plane: f32,

    cached_rot_matrix: Mat3,
    rot_matrix_dirty: bool,
    cached_inv_rot_matrix: Mat3,
    inv_rot_matrix_dirty: bool,
    
    pub speed: Vec3,
    speed_cap: f32,
}

impl Camera {
    pub fn new(position: Vec3, pitch: f32, yaw: f32, aspect_ratio: f32, speed_cap: f32) -> Camera {
        Camera {
            position,
            pitch,
            yaw,
            fov: 60.0,
            aspect_ratio,
            near_plane: 0.1,
            far_plane: 1000.0,
            cached_rot_matrix: Default::default(),
            rot_matrix_dirty: true,
            cached_inv_rot_matrix: Default::default(),
            inv_rot_matrix_dirty: true,
            speed: Vec3::zero(),
            speed_cap,
        }
    }

    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
    }

    pub fn translate(&mut self, translation: Vec3) {
        self.position += translation;
    }

    pub fn rotate(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch += delta_pitch;

        self.rot_matrix_dirty = true;
        self.inv_rot_matrix_dirty = true;
    }
    
    pub fn add_speed(&mut self, speed: Vec3) {
        self.speed += speed;
    }
    
    pub fn remove_speed(&mut self, speed: Vec3) {
        self.speed -= speed;
    }
    
    pub fn set_speed(&mut self, speed: Vec3) {
        self.speed = speed;
    }
    
    pub fn tick_speed(&mut self, delta: f64) -> bool {
        if self.speed.mag_sq() < f32::EPSILON {
            return false;
        }
        let rot_mat = self.get_rotation_matrix();
        let speed_vec = self.speed / (self.speed.mag() / self.speed_cap);
        let speed_vec = Vec3::new((speed_vec.x as f64 * delta) as f32, (speed_vec.y as f64 * delta) as f32, (speed_vec.z as f64 * delta) as f32);
        self.position += rot_mat * speed_vec;
        true
    }

    pub fn get_rotor(&self) -> Rotor3 {
        Rotor3::from_euler_angles(0.0, self.pitch.to_radians(), self.yaw.to_radians())
    }

    pub fn forward_direction(&mut self) -> Vec3 {
        self.get_rotation_matrix() * -Vec3::unit_z()
    }

    pub fn right_direction(&mut self) -> Vec3 {
        self.get_rotation_matrix() * Vec3::unit_x()
    }

    pub fn up_direction(&mut self) -> Vec3 {
        self.get_rotation_matrix() * Vec3::unit_y()
    }

    pub fn get_rotation_matrix(&mut self) -> Mat3 {
        if self.rot_matrix_dirty {
            self.cached_rot_matrix = self.get_rotor().into_matrix();
            self.rot_matrix_dirty = false;
        }

        self.cached_rot_matrix
    }

    pub fn look_at(&mut self, target: Vec3) -> Mat4 {
        let forward = (target - self.position).normalized();
        let right = forward.cross(Vec3::new(0.0, 1.0, 0.0)).normalized();
        let up = right.cross(forward).normalized();

        Mat4::new(
            Vec4::new(right.x, right.y, right.z, 0.0),
            Vec4::new(up.x, up.y, up.z, 0.0),
            Vec4::new(-forward.x, -forward.y, -forward.z, 0.0),
            Vec4::new(-right.dot(self.position),
                      -up.dot(self.position),
                      forward.dot(self.position),
                      1.0),
        )
    }

    pub fn view_matrix(&mut self) -> Mat4 {
        if self.inv_rot_matrix_dirty {
            let inv_rotor = self.get_rotor().reversed();
            self.cached_inv_rot_matrix = inv_rotor.into_matrix();
            self.inv_rot_matrix_dirty = false;
        }
        let inv_translation = -(self.cached_inv_rot_matrix * self.position);

        Mat4::new(
            Vec4::new(self.cached_inv_rot_matrix.cols[0].x, self.cached_inv_rot_matrix.cols[0].y, self.cached_inv_rot_matrix.cols[0].z, 0.0),
            Vec4::new(self.cached_inv_rot_matrix.cols[1].x, self.cached_inv_rot_matrix.cols[1].y, self.cached_inv_rot_matrix.cols[1].z, 0.0),
            Vec4::new(self.cached_inv_rot_matrix.cols[2].x, self.cached_inv_rot_matrix.cols[2].y, self.cached_inv_rot_matrix.cols[2].z, 0.0),
            Vec4::new(inv_translation.x, inv_translation.y, inv_translation.z, 1.0),
        )
    }

    pub fn projection_matrix(&mut self) -> Mat4 {
        let cot = Self::cotan(self.fov * 0.5);
        let range_inv = (self.near_plane - self.far_plane).recip();

        Mat4::new(
            Vec4::new(cot / self.aspect_ratio, 0.0, 0.0, 0.0),
            Vec4::new(0.0, -cot, 0.0, 0.0),
            Vec4::new(0.0, 0.0, (self.far_plane + self.near_plane) * range_inv, -1.0),
            Vec4::new(0.0, 0.0, 2.0 * self.far_plane * self.near_plane * range_inv, 0.0),
        )
    }
    
    pub fn as_ray(&mut self) -> Ray {
        Ray::new(self.position, self.forward_direction())
    }

    fn cotan(a: f32) -> f32 {
        a.to_radians().tan().recip()
    }
}

impl Default for Camera {
    fn default() -> Camera {
        Camera::new(Vec3::zero(), 0.0, 0.0, 16f32 / 9f32, 1.0)
    }
}