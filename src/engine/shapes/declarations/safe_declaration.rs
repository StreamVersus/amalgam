use crate::engine::shapes::declarations::intersection::{IDeclaration, Length, Points};
use std::sync::{Arc, OnceLock};

pub struct SafeDeclaration {
    length: bool,
    all_points: bool,
    closest_point: bool,

    length_promise: Option<Resolver<Length>>,
    points_promise: Option<Resolver<Points>>,
}
pub struct Promise<T> {
    value: Arc<OnceLock<T>>,
}

impl<T> Promise<T> {
    pub fn new() -> (Self, Resolver<T>) {
        let value = Arc::new(OnceLock::new());
        let promise = Self { value: value.clone() };
        let resolver = Resolver { value };
        (promise, resolver)
    }

    pub fn get(&self) -> Option<&T> {
        self.value.get()
    }

    pub fn is_ready(&self) -> bool {
        self.value.get().is_some()
    }
}

pub struct Resolver<T> {
    value: Arc<OnceLock<T>>,
}

impl<T> Resolver<T> {
    pub fn resolve(&self, val: T) -> Result<(), T> {
        self.value.set(val)
    }
}

impl SafeDeclaration {
    pub fn new() -> Self {
        Self {
            length: false,
            all_points: false,
            closest_point: false,

            length_promise: None,
            points_promise: None,
        }
    }

    pub fn ask_length(&mut self) -> Promise<Length> {
        self.length = true;
        let (promise, resolver) = Promise::<Length>::new();
        self.length_promise = Some(resolver);
        promise
    }

    pub fn ask_closest_points(&mut self) -> Promise<Points> {
        self.closest_point = true;
        self.all_points = false;

        let (promise, resolver) = Promise::<Points>::new();
        self.points_promise = Some(resolver);
        promise
    }

    pub fn ask_all_points(&mut self) -> Promise<Points> {
        self.all_points = true;
        self.closest_point = false;

        let (promise, resolver) = Promise::<Points>::new();
        self.points_promise = Some(resolver);
        promise
    }
}

impl IDeclaration for SafeDeclaration {
    fn length(&self) -> bool {
        self.length
    }

    fn all_points(&self) -> bool {
        self.all_points
    }

    fn closest_point(&self) -> bool {
        self.closest_point
    }

    fn finalize_declaration(&self, length: Length, points: Points) {
        if self.length {
            self.length_promise.as_ref().unwrap().resolve(length).unwrap();
        }

        if self.all_points || self.closest_point {
            self.points_promise.as_ref().unwrap().resolve(points).unwrap();
        }
    }
}