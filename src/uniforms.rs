// Copyright (c) 2017 Remco Kranenburg
//
// GNU GENERAL PUBLIC LICENSE
//    Version 3, 29 June 2007
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use glium::texture::SrgbTexture2d;
use glium::uniforms::Uniforms;
use glium::uniforms::UniformValue;

use light::Light;

pub const MAX_NUM_LIGHTS: usize = 32usize;

pub struct ObjectUniforms<'a> {
  pub projection: [[f32; 4]; 4],
  pub view: [[f32; 4]; 4],
  pub model: [[f32; 4]; 4],
  pub albedo_map: &'a SrgbTexture2d,
  pub metalness: f32,
  pub reflectivity: f32,
  pub num_lights: i32,
  pub lights: [Light; MAX_NUM_LIGHTS],
  pub eye_i: usize,
  pub is_anaglyph: bool,
}

impl<'a> Uniforms for ObjectUniforms<'a> {
  fn visit_values<'b, F: FnMut(&str, UniformValue<'b>)>(&'b self, mut f: F) {
    f("projection", UniformValue::Mat4(self.projection));
    f("view", UniformValue::Mat4(self.view));
    f("model", UniformValue::Mat4(self.model));
    f("albedo_map", UniformValue::SrgbTexture2d(self.albedo_map, None));
    f("metalness", UniformValue::Float(self.metalness));
    f("reflectivity", UniformValue::Float(self.reflectivity));
    f("num_lights", UniformValue::SignedInt(self.num_lights));

    for i in 0..MAX_NUM_LIGHTS {
      f(&format!("lights[{}].color", i)[..], UniformValue::Vec3(self.lights[i].color));
      f(&format!("lights[{}].position", i)[..], UniformValue::Vec3(self.lights[i].position));
    }

    f("eye_i", UniformValue::UnsignedInt(self.eye_i as u32));
    f("is_anaglyph", UniformValue::Bool(self.is_anaglyph));
  }
}
