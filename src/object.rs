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
use cgmath::SquareMatrix;
use cgmath::Vector3;
use glium::Program;
use glium::Surface;
use glium::DrawParameters;
use glium::backend::Facade;
use glium::index::NoIndices;
use glium::index::PrimitiveType;
use glium::IndexBuffer;
use glium::texture::SrgbTexture2d;
use glium::texture::RawImage2d;
use glium::VertexBuffer;
use image;
use std::path::Path;
use std::rc::Rc;
use std::env;
use tobj;

use geometry::Geometry;
use geometry::Normal;
use geometry::Vertex;
use geometry::Texcoord;
use light::Light;
use material::Material;
use math;
use mesh::Mesh;
use uniforms::ObjectUniforms;

pub struct Object {
  pub mesh: Option<Mesh>,
  pub transform: Matrix4<f32>,
}

impl Object {
  pub fn from_file(context: &Facade, filename: &str) -> Vec<Object> {
    // TODO: put this in a 'system integration' module
    let executable_string = env::args().nth(0).unwrap();
    let executable_path = Path::new(&executable_string).parent().unwrap();
    let project_path = executable_path.parent().unwrap().parent().unwrap();

    let mut objects = Vec::<Object>::new();
    let mut materials = Vec::<Material>::new();
    let (objs, mtls) = tobj::load_obj(&Path::new(filename)).unwrap(); // TODO: propagate error

    let mut materials = Vec::new();

    for mtl in mtls {
      let albedo_map_filename = mtl.diffuse_texture;

      let image = image::open(&Path::new(&albedo_map_filename))
        .unwrap_or(image::open(&project_path.join("data").join("empty.bmp"))
          .expect(&format!("Could not open: {} and also not its empty replacement",
              albedo_map_filename)))
        .to_rgba();
      let image_dimensions = image.dimensions();
      let image = RawImage2d::from_raw_rgba_reversed(&image.into_raw(), image_dimensions);
      let albedo_map = SrgbTexture2d::new(context, image).unwrap();

      materials.push(Rc::new(Material {
        albedo_map: albedo_map,
        metalness: 0.0,
        reflectivity: 0.0,
      }));
    }

    for obj in objs {
      let indices = if obj.mesh.indices.len() > 0 {
        Some(IndexBuffer::new(context, PrimitiveType::TrianglesList, &obj.mesh.indices).unwrap())
      } else {
        None
      };

      let mut normals = VertexBuffer::empty(context, obj.mesh.normals.len()).unwrap();
      {
        let mut mapped = normals.map();
        for i in 0..obj.mesh.normals.len() / 3 {
          mapped[i] = Normal { normal: (
            obj.mesh.normals[i * 3 + 0],
            obj.mesh.normals[i * 3 + 1],
            obj.mesh.normals[i * 3 + 2],
          )};
        }
      }

      let mut vertices = VertexBuffer::empty(context, obj.mesh.positions.len()).unwrap();
      {
        let mut mapped = vertices.map();
        for i in 0..obj.mesh.positions.len() / 3 {
          mapped[i] = Vertex { position: (
            obj.mesh.positions[i * 3 + 0],
            obj.mesh.positions[i * 3 + 1],
            obj.mesh.positions[i * 3 + 2],
          )};
        }
      }

      let mut texcoords = VertexBuffer::empty(context, obj.mesh.texcoords.len()).unwrap();
      {
        let mut mapped = texcoords.map();
        for i in 0..obj.mesh.texcoords.len() / 2 {
          mapped[i] = Texcoord { texcoord: (
            obj.mesh.texcoords[i * 2 + 0],
            obj.mesh.texcoords[i * 2 + 1],
          )};
        }
      }

      objects.push(Object {
        mesh: Some(Mesh {
          geometry: Geometry {
            indices: indices,
            normals: normals,
            vertices: vertices,
            texcoords: texcoords,
          },
          material: Rc::clone(&materials[obj.mesh.material_id.unwrap()]),
        }),
        transform: Matrix4::<f32>::identity(),
      });
    }

    objects
  }

  pub fn new_plane(context: &Facade, material: Rc<Material>, size: [f32;2], pos: [f32;3],
      rot: [f32;3], scale: [f32;3]) -> Object {
    let rotation = Matrix4::from(Euler { x: Rad(rot[0]), y: Rad(rot[1]), z: Rad(rot[2]) });
    let scale = Matrix4::from_nonuniform_scale(scale[0], scale[1], scale[2]);
    let translation = Matrix4::from_translation(Vector3::new(pos[0], pos[1], pos[2]));
    let matrix = translation * scale * rotation;

    Object {
      mesh: Some(Mesh {
        geometry: Geometry::new_quad(context, size, false),
        material: Rc::clone(&material),
      }),
      transform: matrix,
    }
  }

  pub fn new_triangle(context: &Facade, material: Rc<Material>, size: [f32;2], pos: [f32;3],
      rot: [f32;3], scale: [f32;3]) -> Object {
    let rotation = Matrix4::from(Euler { x: Rad(rot[0]), y: Rad(rot[1]), z: Rad(rot[2]) });
    let scale = Matrix4::from_nonuniform_scale(scale[0], scale[1], scale[2]);
    let translation = Matrix4::from_translation(Vector3::new(pos[0], pos[1], pos[2]));
    let matrix = translation * scale * rotation;

    Object {
      mesh: Some(Mesh {
        geometry: Geometry::new_triangle(context, size),
        material: Rc::clone(&material),
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
          albedo_map: &m.material.albedo_map,
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
