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
use cgmath::SquareMatrix;
use glium::backend::Facade;
use glium::DrawParameters;
use glium::index::NoIndices;
use glium::index::PrimitiveType;
use glium::PolygonMode;
use glium::Program;
use glium::Surface;
use glium::VertexBuffer;
use std::f32;

use math;

#[derive(Copy, Clone)]
pub struct ConicVertex {
  pub theta: f32,
}

implement_vertex!(ConicVertex, theta);

pub struct Conic {
  pub theta: VertexBuffer<ConicVertex>,
  pub eccentricity: f32,
  pub semi_latus_rectum: f32,
  pub transform: Matrix4<f32>,

  program: Program,
}

impl Conic {
  pub fn new(context: &Facade) -> Conic {
    let mut theta_vertices = Vec::new();

    let num_vertices = 10000i32;
    for i in 0 .. num_vertices {
      theta_vertices.push(ConicVertex { theta: ((i - num_vertices / 2) as f32 * 360.0 / num_vertices as f32) * f32::consts::PI / 180.0 });
    }

    let program = Program::from_source(
      context,
      &r#"
        #version 140

        uniform mat4 projection;
        uniform mat4 view;
        uniform mat4 model;

        uniform float eccentricity;
        uniform float semi_latus_rectum;

        in float theta;

        out vec3 v_color;

        void main() {
          float c = cos(theta);
          float s = sin(theta);
          float r = semi_latus_rectum / (1.0 + eccentricity * c);
          vec4 position = vec4(r * c, r * s, 0.0, 1.0);

          vec4 position_global = model * position;
          vec4 position_eye = view * position_global;

          v_color = vec3(0.4, 8.0, 0.0);

          gl_Position = projection * position_eye;
        }
      "#,
      &r#"
        #version 330

        const float SCREEN_GAMMA = 2.2;
        const float INTENSITY = 20.0;

        in vec3 v_color;

        out vec4 color;

        void main() {
          vec3 color_gamma_corrected = pow(v_color, vec3(1.0 / SCREEN_GAMMA)); // assumes textures are linearized (i.e. not sRGB))
          color = vec4(v_color, 1.0); //vec4(color_gamma_corrected, 1.0);
        }
      "#,
      None).unwrap();

    Conic {
      theta: VertexBuffer::new(context, &theta_vertices).unwrap(),
      eccentricity: 1.0,
      semi_latus_rectum: 1.0,
      transform: Matrix4::identity(),

      program: program,
    }
  }

  pub fn update(&mut self) -> bool {
    return true;
  }

  pub fn draw<S>(&mut self, _context: &Facade, target: &mut S, projection: [[f32; 4]; 4],
      view: [[f32; 4]; 4], render_params: &DrawParameters) where S: Surface {
    let uniforms = uniform! {
      projection: projection,
      view: view,
      eccentricity: self.eccentricity,
      semi_latus_rectum: self.semi_latus_rectum,
      model: math::matrix_to_uniform(self.transform),
    };

    let mut point_render_params = render_params.clone();
    point_render_params.point_size = Some(20.0);
    point_render_params.polygon_mode = PolygonMode::Point;

    target.draw(
        &self.theta,
        NoIndices(PrimitiveType::Points),
        &self.program,
        &uniforms,
        &point_render_params).unwrap();
  }

  pub fn decrease_eccentricity(&mut self) {
    self.eccentricity -= 0.1;
    println!("eccentricity: {}", self.eccentricity);
  }

  pub fn increase_eccentricity(&mut self) {
    self.eccentricity += 0.1;
    println!("eccentricity: {}", self.eccentricity);
  }

  pub fn decrease_slr(&mut self) {
    self.semi_latus_rectum -= 0.1;
    println!("semi latus rectum: {}", self.semi_latus_rectum);
  }

  pub fn increase_slr(&mut self) {
    self.semi_latus_rectum += 0.1;
    println!("semi latus rectum: {}", self.semi_latus_rectum);
  }
}