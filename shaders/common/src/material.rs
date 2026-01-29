pub struct MaterialBinary<'a> {
    bytes: &'a [u8]
}
impl<'a> MaterialBinary<'a> {
    pub fn parse(&self) -> UncheckedMaterial {
        let mat_type = MaterialType::from_int(*bytemuck::from_bytes(&self.bytes[0..3]));

        UncheckedMaterial {
            mat_type,

            diffuse: None,
            specular: None,
        }
    }
}
pub enum MaterialType {
    Diffuse = 0,
    Specular = 1,
}

impl MaterialType {
    pub fn from_int(id: u32) -> MaterialType {
        match id {
            0 => MaterialType::Diffuse,
            1 => MaterialType::Specular,
            _ => panic!("Invalid material type"),
        }
    }
}

pub struct UncheckedMaterial {
    mat_type: MaterialType,

    diffuse: Option<DiffuseMaterial>,
    specular: Option<SpecularMaterial>,
}

pub struct DiffuseMaterial {

}

pub struct SpecularMaterial {

}