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

use cgmath::Euler;
use cgmath::Rad;
use cgmath::Matrix4;
use cgmath::Vector3;
use glium::Program;
use glium::Surface;
use glium::DrawParameters;
use glium::backend::glutin_backend::GlutinFacade;
use glium::index::NoIndices;
use glium::index::PrimitiveType;
use glium::texture::SrgbTexture2d;

use geometry::Geometry;
use light::Light;
use material::Material;
use math;
use mesh::Mesh;
use uniforms::ObjectUniforms;

pub struct Object<'a> {
  pub mesh: Option<Mesh<'a>>,
  pub transform: Matrix4<f32>,
}

impl<'a> Object<'a> {
  pub fn new_plane(context: &GlutinFacade, tex: &'a SrgbTexture2d, size: [f32;2], pos: [f32;3],
      rot: [f32;3], scale: [f32;3]) -> Object<'a> {
    let rotation = Matrix4::from(Euler { x: Rad(rot[0]), y: Rad(rot[1]), z: Rad(rot[2]) });
    let scale = Matrix4::from_nonuniform_scale(scale[0], scale[1], scale[2]);
    let translation = Matrix4::from_translation(Vector3::new(pos[0], pos[1], pos[2]));
    let matrix = translation * scale * rotation;

    Object {
      mesh: Some(Mesh {
        geometry: Geometry::new_quad(context, size, false),
        material: Material { albedo_map: tex, metalness: 0.0, reflectivity: 0.0 },
      }),
      transform: matrix,
    }
  }

  pub fn new_triangle(context: &GlutinFacade, tex: &'a SrgbTexture2d, size: [f32;2], pos: [f32;3],
      rot: [f32;3], scale: [f32;3]) -> Object<'a> {
    let rotation = Matrix4::from(Euler { x: Rad(rot[0]), y: Rad(rot[1]), z: Rad(rot[2]) });
    let scale = Matrix4::from_nonuniform_scale(scale[0], scale[1], scale[2]);
    let translation = Matrix4::from_translation(Vector3::new(pos[0], pos[1], pos[2]));
    let matrix = translation * scale * rotation;

    Object {
      mesh: Some(Mesh {
        geometry: Geometry::new_triangle(context, size),
        material: Material { albedo_map: tex, metalness: 0.0, reflectivity: 0.0 },
      }),
      transform: matrix,
    }
  }

  pub fn draw<S>(&mut self, target: &mut S, projection: [[f32; 4]; 4], view: [[f32; 4]; 4],
      program: &Program, render_params: &DrawParameters, num_lights: i32, lights: [Light; 32])
      where S: Surface {
    match self.mesh {
      Some(ref m) => {

        let uniforms = ObjectUniforms {
          projection: projection,
          view: view,
          model: math::matrix_to_uniform(self.transform),
          albedo_map: m.material.albedo_map,
          metalness: m.material.metalness,
          reflectivity: m.material.reflectivity,
          num_lights: num_lights,
          lights: lights,
        };

        match m.geometry.indices {
          Some(ref indices) => target.draw(
            (&m.geometry.vertices, &m.geometry.normals, &m.geometry.texcoords),
            indices,
            program,
            &uniforms,
            render_params).unwrap(),
          None => target.draw(
            (&m.geometry.vertices, &m.geometry.normals, &m.geometry.texcoords),
            NoIndices(PrimitiveType::TrianglesList),
            program,
            &uniforms,
            render_params).unwrap(),
        }
      },
      None => (),
    }
  }
}
