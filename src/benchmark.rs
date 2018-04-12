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
use csv::Reader;
use glium::backend::Facade;
use glium::DrawParameters;
use glium::framebuffer::SimpleFrameBuffer;
use glium::index::NoIndices;
use glium::index::PrimitiveType;
use glium::PolygonMode;
use glium::Program;
use glium::Surface;
use glium::vertex::BufferCreationError;
use glium::VertexBuffer;
use std::fs::File;

use geometry::Vertex;
use gui::Action;
use light::Light;
use math;
use object::Drawable;
use object::Object;

fn normalize(x: Vec<f32>) -> Vec<f32> {
  let sum: f32 = x.iter().sum();
  x.iter().map(|e| e / sum).collect()
}

#[derive(Copy, Clone)]
pub struct Color {
  pub color: (f32, f32, f32),
}

implement_vertex!(Color, color);

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct BenchmarkEntry {
  pub draw_time: u32,
  pub target_quality: [f32; 3],
  pub normalized_weights: [f32; 3],
}

pub struct Benchmark {
  pub entries: Vec<BenchmarkEntry>,
  pub num_samples_per_weight: u32,

  grid: VertexBuffer<Vertex>,
  grid_program: Program,

  points: VertexBuffer<Vertex>,
  colors: VertexBuffer<Color>,
  program: Program,
}

impl Benchmark {
  pub fn from_file<F>(context: &F, filename: &str) -> Benchmark
      where F: Facade {

    let mut vertices = Vec::new();
    let mut colors = Vec::new();
    let file = File::open(filename).unwrap();
    let mut reader = Reader::from_reader(file);

    let scale_factor = 1.0 / 64_000_000f32;

    for result in reader.records() {
        match result {
          Ok(record) => {
            vertices.push(Vertex {
              position: (
                record[7].parse::<f32>().unwrap(),
                record[4].parse::<f32>().unwrap() / 64_000_000f32,
                record[9].parse::<f32>().unwrap(),
              ),
            });

            let draw_time = record[4].parse::<f32>().unwrap();

            colors.push(Color {
              color: (
                draw_time * scale_factor,
                0.0,
                1.0 - draw_time * scale_factor,
              )
            })
          },
          Err(e) => println!("Could not parse line from benchmark: {}", e),
        }
    }

    Benchmark {
      entries: Vec::new(),
      num_samples_per_weight: 0,
      grid: Benchmark::construct_grid(context).unwrap(),
      grid_program: construct_grid_program(context),
      points: VertexBuffer::new(context, &vertices).unwrap(),
      colors: VertexBuffer::new(context, &colors).unwrap(),
      program: construct_program(context),
    }
  }

  pub fn as_object(self) -> Object {
    Object {
      children: Vec::new(),
      drawable: Some(Box::new(self)),
      transform: Matrix4::identity(),
    }
  }

  pub fn get_entries_by_normalized_weights(&self, weights: Vec<f32>) -> Vec<&BenchmarkEntry> {
    let normalized_weights = normalize(weights);
    let distance = 1.0 / self.num_samples_per_weight as f32;

    let mut result = Vec::new();

    for entry in &self.entries {
      let mut pairs = entry.normalized_weights.iter().zip(normalized_weights.iter());
      if pairs.all(|p| (p.0 - p.1).abs() < distance) {
        result.push(entry.clone());
      }
    }

    result
  }

  fn construct_grid<F>(context: &F) -> Result<VertexBuffer<Vertex>, BufferCreationError> where F: Facade {
    let mut grid_vec = Vec::new();

    for x in 0 .. 11 {
      grid_vec.push(Vertex { position: (-1.0 + x as f32 * 0.2, 0.0, -1.0) });
      grid_vec.push(Vertex { position: (-1.0 + x as f32 * 0.2, 0.0, 1.0) });
    }

    for z in 0 .. 11 {
      grid_vec.push(Vertex { position: (-1.0, 0.0, -1.0 + z as f32 * 0.2) });
      grid_vec.push(Vertex { position: (1.0, 0.0, -1.0 + z as f32 * 0.2) });
    }

    grid_vec.push(Vertex { position: (0.0, -1.0, 0.0) });
    grid_vec.push(Vertex { position: (0.0, 1.0, 0.0) });

    VertexBuffer::new(context, &grid_vec)
  }
}

impl Drawable for Benchmark {
  fn draw(&mut self, target: &mut SimpleFrameBuffer,
      projection: [[f32; 4]; 4], view: [[f32; 4]; 4], model_transform: Matrix4<f32>,
      render_params: &DrawParameters, _: i32, _: &[Light; 32]) {
    let uniforms = uniform! {
      projection: projection,
      view: view,
      model: math::matrix_to_uniform(model_transform),
    };

    let mut grid_render_params = render_params.clone();
    grid_render_params.polygon_mode = PolygonMode::Line;

    target.draw(
        &self.grid,
        NoIndices(PrimitiveType::LinesList),
        &self.grid_program,
        &uniforms,
        &grid_render_params).unwrap();

    let mut point_render_params = render_params.clone();
    point_render_params.point_size = Some(20.0);
    point_render_params.polygon_mode = PolygonMode::Point;

    target.draw(
        (&self.points, &self.colors),
        NoIndices(PrimitiveType::Points),
        &self.program,
        &uniforms,
        &point_render_params).unwrap();
  }

  fn update(&mut self, _: &Facade, _: Matrix4<f32>, _: &Vec<Action>) {}
}

fn construct_grid_program<F>(context: &F) -> Program
    where F: Facade {
  Program::from_source(
    context,
    &r#"
      #version 140

      uniform mat4 projection;
      uniform mat4 view;
      uniform mat4 model;

      in vec3 position;

      void main() {
        vec4 position_global = model * vec4(position, 1.0);
        vec4 position_eye = view * position_global;

        gl_Position = projection * position_eye;
      }
    "#,
    &r#"
      #version 330

      const float SCREEN_GAMMA = 2.2;
      const float INTENSITY = 20.0;

      out vec4 color;

      void main() {
        color = vec4(0.0, 0.0, 0.0, 1.0);
      }
    "#,
    None).unwrap()
}

fn construct_program<F>(context: &F) -> Program
    where F: Facade {
  Program::from_source(
    context,
    &r#"
      #version 140

      uniform mat4 projection;
      uniform mat4 view;
      uniform mat4 model;

      in vec3 position;
      in vec3 color;

      out vec3 v_color;

      void main() {
        vec4 position_global = model * vec4(position, 1.0);
        vec4 position_eye = view * position_global;

        v_color = color;

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
    None).unwrap()
}
