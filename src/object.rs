// Copyright (c) 2018 Remco Kranenburg
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
use glium::Display;
use glium::DrawParameters;
use glium::backend::Facade;
use glium::framebuffer::SimpleFrameBuffer;
use glium::index::PrimitiveType;
use glium::IndexBuffer;
use glium::VertexBuffer;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::f32;
use std::path::MAIN_SEPARATOR;
use std::path::Path;
use std::rc::Rc;
use tobj;

use drawable::Drawable;
use geometry::Geometry;
use geometry::Normal;
use geometry::Vertex;
use geometry::Texcoord;
use light::Light;
use material::Material;
use mesh::Mesh;
use resources::ResourceManager;

pub struct Object {
  pub children: Vec<Object>,
  pub drawable: Option<Box<Drawable>>,
  pub transform: Matrix4<f32>,
  pub size: f32,
}

impl Object {
  pub fn from_file<F>(context: &F, resource_manager: &ResourceManager, filename: &str) -> Object
      where F: Facade {
    let mut objects = Vec::new();
    let mut materials = Vec::new();

    let obj_file = Path::new(filename);
    let obj_path = obj_file.parent().unwrap();

    let (objs, mtls) = tobj::load_obj(&obj_file).unwrap(); // TODO: propagate error

    for mtl in mtls {
      let texture_filename = mtl.diffuse_texture;//.replace("\\", &MAIN_SEPARATOR.to_string());
      let texture_file = obj_path.join(&texture_filename);
      let albedo_map = resource_manager.get_texture(texture_file.to_str().unwrap()).unwrap();

      materials.push(Rc::new(RefCell::new(Material {
        albedo_map: Rc::clone(&albedo_map),
        ambient_color: mtl.ambient,
        diffuse_color: mtl.diffuse,
        specular_color: mtl.specular,
        shininess: mtl.shininess,
        metalness: 0.0,
        reflectivity: 0.0,
      })));
    }

    let mut global_bounding_box = (
      [f32::INFINITY; 3],
      [f32::NEG_INFINITY; 3],
    );

    for obj in objs {
      let mut bounding_box = (
        [f32::INFINITY; 3],
        [f32::NEG_INFINITY; 3],
      );

      for idx in &obj.mesh.indices {
        let i = *idx as usize;
        let pos = [
            obj.mesh.positions[3 * i],
            obj.mesh.positions[3 * i + 1],
            obj.mesh.positions[3 * i + 2]
        ];

        for i in 0..pos.len() {
          bounding_box.0[i] = bounding_box.0[i].min(pos[i]);
          bounding_box.1[i] = bounding_box.1[i].max(pos[i]);
        }

      }

      let size = (0..3).map(|i| (bounding_box.1[i] - bounding_box.0[i]).powi(2)).sum();

      for i in 0..bounding_box.0.len() {
        global_bounding_box.0[i] = global_bounding_box.0[i].min(bounding_box.0[i]);
        global_bounding_box.1[i] = global_bounding_box.1[i].max(bounding_box.1[i]);
      }

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
        children: Vec::new(),
        drawable: Some(Box::new(Mesh::new(
            context,
            Geometry {
              bounding_box,
              indices: indices,
              normals: normals,
              vertices: vertices,
              texcoords: texcoords,
            },
            Rc::clone(&materials[obj.mesh.material_id.unwrap()]),
            resource_manager))),
        transform: Matrix4::<f32>::identity(),
        size: size,
      });
    }

    objects.sort_unstable_by(|a, b| {
      if b.size < a.size {
        Ordering::Less
      } else if b.size > a.size {
        Ordering::Greater
      } else {
        Ordering::Equal
      }
    });

    let lengths = global_bounding_box.1.iter().zip(global_bounding_box.0.iter()).map(|x| x.0 - x.1);
    let target_length = 450.0; // 2²+2²+2²
    let current_length = lengths.fold(0.0, |result, x| result + f32::powf(x, 2.0));

    let scale = Matrix4::from_scale(f32::sqrt(target_length / current_length));
    let translation = Matrix4::from_translation(Vector3::new(0.0, 1.0, 0.0));

    Object {
      children: objects,
      drawable: None,
      transform: translation * scale,
      size: 0.0,
    }
  }

  pub fn new_plane<F>(context: &F, resource_manager: &ResourceManager,
      material: Rc<RefCell<Material>>, size: [f32;2], pos: [f32;3], rot: [f32;3], scale: [f32;3])
      -> Object
      where F: Facade {
    let rotation = Matrix4::from(Euler { x: Rad(rot[0]), y: Rad(rot[1]), z: Rad(rot[2]) });
    let scale_mat = Matrix4::from_nonuniform_scale(scale[0], scale[1], scale[2]);
    let translation = Matrix4::from_translation(Vector3::new(pos[0], pos[1], pos[2]));
    let matrix = translation * scale_mat * rotation;

    Object {
      children: Vec::new(),
      drawable: Some(Box::new(Mesh::new(
          context,
          Geometry::new_quad(context, size, false),
          material,
          resource_manager))),
      transform: matrix,
      size: size[0] * scale[0] * size[1] * scale[1],
    }
  }

  pub fn new_triangle<F>(context: &F, resource_manager: &ResourceManager,
      material: Rc<RefCell<Material>>, size: [f32;2], pos: [f32;3], rot: [f32;3], scale: [f32;3])
      -> Object
      where F: Facade{
    let rotation = Matrix4::from(Euler { x: Rad(rot[0]), y: Rad(rot[1]), z: Rad(rot[2]) });
    let scale_mat = Matrix4::from_nonuniform_scale(scale[0], scale[1], scale[2]);
    let translation = Matrix4::from_translation(Vector3::new(pos[0], pos[1], pos[2]));
    let matrix = translation * scale_mat * rotation;

    Object {
      children: Vec::new(),
      drawable: Some(Box::new(Mesh::new(
          context,
          Geometry::new_triangle(context, size),
          material,
          resource_manager))),
      transform: matrix,
      size: (size[0] * scale[0] * size[1] * scale[1]).sqrt(),
    }
  }

  pub fn draw(&mut self, quality_level: f32, i: u32, num_objects: u32,
      target: &mut SimpleFrameBuffer, context: &Display, projection: [[f32; 4]; 4],
      view: [[f32; 4]; 4], render_params: &DrawParameters, num_lights: i32, lights: &[Light; 32],
      eye_i: usize, is_anaglyph: bool, show_bbox: bool) -> u32 {
    let root = Matrix4::<f32>::identity();
    self.draw_recurse(quality_level, i, num_objects, target, context, projection, view, root, render_params,
        num_lights, lights, eye_i, is_anaglyph, show_bbox)
  }

  fn draw_recurse(&mut self, quality_level: f32, i: u32, num_objects: u32, target: &mut SimpleFrameBuffer, context: &Display,
      projection: [[f32; 4]; 4], view: [[f32; 4]; 4], group: Matrix4<f32>,
      render_params: &DrawParameters, num_lights: i32, lights: &[Light; 32], eye_i: usize,
      is_anaglyph: bool, show_bbox: bool) -> u32 {
    let model_transform = group * self.transform;

    match self.drawable {
      Some(ref mut d) => d.draw(target, context, projection, view, model_transform, render_params,
          num_lights, lights, eye_i, is_anaglyph, show_bbox),
      None => (),
    }

    let mut result = i + 1;

    if i == (1 * num_objects / 4) || i == (2 * num_objects / 4) || i == (3 * num_objects / 4) {
      context.flush();
    }

    for object in &mut self.children {
      if quality_level > (result as f32 / num_objects as f32) {
        result = object.draw_recurse(quality_level, result, num_objects, target, context, projection, view,
            model_transform, render_params, num_lights, lights, eye_i, is_anaglyph, show_bbox);
      }
    }
    result
  }
}
