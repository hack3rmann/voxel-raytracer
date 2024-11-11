use bytemuck::{Pod, Zeroable};
use glam::*;
use serde::{Deserialize, Serialize};

#[repr(transparent)]
#[derive(
    Clone,
    Default,
    Debug,
    PartialEq,
    Copy,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Pod,
    Zeroable,
    Serialize,
    Deserialize,
)]
pub struct Color(pub u32);

impl Color {
    pub const TRANSPARENT_BLACK: Self = Self(0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self(u32::from_le_bytes([
            (255.0 * r) as u8,
            (255.0 * g) as u8,
            (255.0 * b) as u8,
            (255.0 * a) as u8,
        ]))
    }

    pub const fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Self(u32::from_le_bytes([r, g, b, 255]))
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    pub fn from_vec4(color: Vec4) -> Self {
        Self::rgba(color.x, color.y, color.z, color.w)
    }

    pub const fn from_vec3(color: Vec3) -> Self {
        Self::rgb(color.x, color.y, color.z)
    }
}

impl From<Vec3> for Color {
    fn from(value: Vec3) -> Self {
        Self::from_vec3(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    pub colors: [Color; Chunk::VOLUME],
}

impl Default for Chunk {
    fn default() -> Self {
        Self::all_same(Color::BLACK)
    }
}

impl Chunk {
    pub const SIZE: usize = 16;
    pub const VOLUME: usize = Self::SIZE * Self::SIZE * Self::SIZE;

    pub const fn all_same(color: Color) -> Self {
        Self {
            colors: [color; Self::VOLUME],
        }
    }

    pub fn new_sphere() -> Self {
        Self {
            colors: std::array::from_fn(|index| {
                let x = index % Self::SIZE;
                let yz = index / Self::SIZE;
                let y = yz % Self::SIZE;
                let z = yz / Self::SIZE;

                let center = 0.5 * Vec3::splat(Self::SIZE as f32);

                if Vec3::new(x as f32, y as f32, z as f32).distance(center) < 5.0 {
                    Color::rgb8(20 * x as u8, 20 * y as u8, 20 * z as u8)
                } else {
                    Color::TRANSPARENT_BLACK
                }
            }),
        }
    }
}
