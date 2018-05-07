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

use cgmath::Matrix4;
use glium::DrawParameters;
use glium::Program;
use glium::backend::Facade;
use glium::framebuffer::SimpleFrameBuffer;
use glium::index::NoIndices;
use glium::index::PrimitiveType;
use glium::Surface;
use std::cell::RefCell;
use std::rc::Rc;

use geometry::Geometry;
use gui::Action;
use light::Light;
use material::Material;
use math;
use object::Drawable;
use resources::ResourceManager;
use uniforms;
use uniforms::ObjectUniforms;

pub struct Mesh {
  pub geometry: Geometry,
  pub material: Rc<RefCell<Material>>,
  pub program: Rc<RefCell<Program>>,
}

impl Mesh {
  pub fn new<F>(display: &F, geometry: Geometry, material: Rc<RefCell<Material>>,
      resource_manager: &ResourceManager) -> Mesh
      where F: Facade {
    let program = resource_manager.get_program("programs/mesh_program", &|| {
      construct_program(display)
    }).unwrap();

    Mesh {
      geometry: geometry,
      material: material,
      program: Rc::clone(&program),
    }
  }
}

impl Drawable for Mesh {
  fn draw(&mut self, target: &mut SimpleFrameBuffer,
      projection: [[f32; 4]; 4], view: [[f32; 4]; 4], model_transform: Matrix4<f32>,
      render_params: &DrawParameters, num_lights: i32, lights: &[Light; 32], eye_i: usize,
      is_anaglyph: bool) {
    let albedo_map = &self.material.borrow().albedo_map;

    let uniforms = ObjectUniforms {
      projection: projection,
      view: view,
      model: math::matrix_to_uniform(model_transform),
      albedo_map: &albedo_map.borrow(),
      metalness: self.material.borrow().metalness,
      reflectivity: self.material.borrow().reflectivity,
      num_lights: num_lights,
      lights: *lights,
      eye_i: eye_i,
      is_anaglyph: is_anaglyph,
    };

    match self.geometry.indices {
      Some(ref indices) => target.draw(
        (&self.geometry.vertices, &self.geometry.normals, &self.geometry.texcoords),
        indices,
        &self.program.borrow(),
        &uniforms,
        render_params).unwrap(),
      None => target.draw(
        (&self.geometry.vertices, &self.geometry.normals, &self.geometry.texcoords),
        NoIndices(PrimitiveType::TrianglesList),
        &self.program.borrow(),
        &uniforms,
        render_params).unwrap(),
    }
  }

  fn update(&mut self, _: &Facade, _: Matrix4<f32>, _: &Vec<Action>) {}
}

fn construct_program<F>(display: &F) -> Program
    where F: Facade {
  Program::from_source(
      display,
      &r#"
        #version 140

        uniform mat4 projection;
        uniform mat4 view;
        uniform mat4 model;

        in vec3 position;
        in vec3 normal;
        in vec2 texcoord;

        out vec3 v_normal;
        out vec2 v_texcoord;
        out vec3 v_vertex_position;

        void main() {
          mat4 normal_matrix = transpose(inverse(model)); // TODO: put this in host code
          vec4 position_global = model * vec4(position, 1.0);
          vec4 position_eye = view * position_global;

          v_texcoord = texcoord;
          v_normal = vec3(normal_matrix * vec4(normal, 1.0));
          v_vertex_position = vec3(position_global);
          gl_Position = projection * position_eye;
        }
      "#,
      &str::replace(r#"
        #version 330
        layout(std140) uniform;

        const float SCREEN_GAMMA = 2.2;
        const float INTENSITY = 20.0;

        struct Light {
          vec3 color;
          vec3 position;
        };

        uniform sampler2D albedo_map;
        uniform int num_lights;
        uniform Light lights[MAX_NUM_LIGHTS];
        uniform uint eye_i;
        uniform bool is_anaglyph;

        in vec3 v_normal;
        in vec2 v_texcoord;
        in vec3 v_vertex_position;

        out vec4 color;

        vec3 attenuate(vec3 color, vec3 light_position, float radius, vec3 surface_position) {
          float dist = distance(light_position, surface_position);
          float attenuation_factor = clamp(1.0 - dist * dist / (radius * radius), 0.0, 1.0);
          attenuation_factor *= attenuation_factor;
          return color * attenuation_factor;
        }

        vec3 calculate_lighting(
            vec3 light_position,
            vec3 normal,
            vec3 combined_color) {
          vec3 light_direction = normalize(light_position - v_vertex_position);
          float lambertian = max(dot(light_direction, normal), 0.0);
          return lambertian * combined_color;
        }

        vec3 make_anaglyph(vec3 color, uint eye_i, bool is_anaglyph) {
          if(is_anaglyph) {
            if(eye_i == 0u) {
              vec3 coefficients = vec3(0.7, 0.15, 0.15);
              return vec3(dot(color, coefficients), 0.0, 0.0);
            } else {
              return vec3(0.0, color.g, color.b);
            }
          } else {
            return color;
          }
        }

        void main() {
          vec3 normal = normalize(v_normal);
          vec3 material_color = vec3(texture(albedo_map, v_texcoord));
          vec3 color_linear = vec3(0.0);

          for(int i = 0; i < num_lights; i++) {
            vec3 color_one_light = calculate_lighting(lights[i].position, normal, lights[i].color * material_color);
            color_one_light = attenuate(color_one_light, lights[i].position, INTENSITY, v_vertex_position);
            color_linear += color_one_light;
          }

          vec3 color_gamma_corrected = pow(color_linear, vec3(1.0 / SCREEN_GAMMA)); // assumes textures are linearized (i.e. not sRGB))
          color = vec4(make_anaglyph(color_gamma_corrected, eye_i, is_anaglyph), 1.0);
        }
      "#, "MAX_NUM_LIGHTS", &format!("{}", uniforms::MAX_NUM_LIGHTS)),
      None).unwrap()
}
