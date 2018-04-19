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
use std::collections::HashMap;
use std::mem;
use std::hash::Hash;
use std::hash::Hasher;

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

pub enum VisualizeMode {
  OneD,
  TwoD,
  ThreeD,
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

  points_1d: VertexBuffer<Vertex>,
  colors_1d: VertexBuffer<Color>,
  points_2d: VertexBuffer<Vertex>,
  colors_2d: VertexBuffer<Color>,
  points_3d: VertexBuffer<Vertex>,
  colors_3d: VertexBuffer<Color>,
  program: Program,

  mode: VisualizeMode,
}

#[derive(Debug, Copy, Clone)]
struct HashableF32(f32);

impl HashableF32 {
  fn key(&self) -> u32 {
    unsafe { mem::transmute(self.0) }
  }
}

impl Hash for HashableF32 {
  fn hash<H>(&self, state: &mut H) where H: Hasher
  {
    self.key().hash(state)
  }
}

impl PartialEq for HashableF32 {
    fn eq(&self, other: &HashableF32) -> bool {
        self.key() == other.key()
    }
}

impl Eq for HashableF32 {}

#[derive(Hash, Eq, PartialEq)]
struct Key1d(HashableF32);

#[derive(Hash, Eq, PartialEq)]
struct Key2d(HashableF32, HashableF32);

#[derive(Hash, Eq, PartialEq)]
struct Key3d(HashableF32, HashableF32, HashableF32);

impl Benchmark {
  pub fn from_file<F>(context: &F, filename: &str) -> Benchmark
      where F: Facade {
    let (vertices_1d, colors_1d) = Benchmark::construct_1d_data(filename);
    let (vertices_2d, colors_2d) = Benchmark::construct_2d_data(filename);
    let (vertices_3d, colors_3d) = Benchmark::construct_3d_data(filename);

    Benchmark {
      entries: Vec::new(),
      num_samples_per_weight: 0,
      grid: Benchmark::construct_grid(context).unwrap(),
      grid_program: construct_grid_program(context),
      points_1d: VertexBuffer::new(context, &vertices_1d).unwrap(),
      colors_1d: VertexBuffer::new(context, &colors_1d).unwrap(),
      points_2d: VertexBuffer::new(context, &vertices_2d).unwrap(),
      colors_2d: VertexBuffer::new(context, &colors_2d).unwrap(),
      points_3d: VertexBuffer::new(context, &vertices_3d).unwrap(),
      colors_3d: VertexBuffer::new(context, &colors_3d).unwrap(),
      program: construct_program(context),
      mode: VisualizeMode::OneD,
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

  fn construct_1d_data(filename: &str) -> (Vec<Vertex>, Vec<Color>) {
    let scale_factor = 1.0 / 64_000_000f32;

    let mut data = HashMap::new();
    let mut vertices = Vec::new();
    let mut colors = Vec::new();

    let mut reader = Reader::from_path(filename).unwrap();

    for result in reader.records() {
      match result {
        Ok(record) => {
          let level = record[7].parse::<f32>().unwrap();
          let time = record[4].parse::<f32>().unwrap();

          let d = data.entry(HashableF32(level)).or_insert((0.0, 0));

          // calculate mean time for each level
          *d = (d.0 + time, d.1 + 1)
        },
        Err(e) => println!("Could not parse line from benchmark: {}", e),
      }
    }

    for (&HashableF32(level), &(time_sum, time_count)) in &data {
      let time = time_sum / time_count as f32;

      vertices.push(Vertex {
        position: (level, time * scale_factor, 0.0),
      });

      colors.push(Color {
        color: (time * scale_factor, 0.0, 1.0 - time * scale_factor,),
      });
    }

    (vertices, colors)
  }

  fn construct_2d_data(filename: &str) -> (Vec<Vertex>, Vec<Color>) {
    let scale_factor = 1.0 / 64_000_000f32;

    let mut data = HashMap::new();
    let mut vertices = Vec::new();
    let mut colors = Vec::new();

    let mut reader = Reader::from_path(filename).unwrap();

    for result in reader.records() {
      match result {
        Ok(record) => {
          let level0 = record[7].parse::<f32>().unwrap();
          let level1 = record[8].parse::<f32>().unwrap();
          let time = record[4].parse::<f32>().unwrap();

          let d = data.entry(Key2d(
            HashableF32(level0),
            HashableF32(level1),
          )).or_insert((0.0, 0));

          // calculate mean time for each level
          *d = (d.0 + time, d.1 + 1)
        },
        Err(e) => println!("Could not parse line from benchmark: {}", e),
      }
    }

    for (&Key2d(HashableF32(level0), HashableF32(level1)), &(time_sum, time_count)) in &data {
      let time = time_sum / time_count as f32;

      vertices.push(Vertex {
        position: (level0, time * scale_factor, level1),
      });

      colors.push(Color {
        color: (time * scale_factor, 0.0, 1.0 - time * scale_factor,),
      });
    }

    (vertices, colors)
  }

  fn construct_3d_data(filename: &str) -> (Vec<Vertex>, Vec<Color>) {
    let scale_factor = 1.0 / 64_000_000f32;

    let mut data = HashMap::new();
    let mut vertices = Vec::new();
    let mut colors = Vec::new();

    let mut reader = Reader::from_path(filename).unwrap();

    for result in reader.records() {
      match result {
        Ok(record) => {
          let level0 = record[7].parse::<f32>().unwrap();
          let level1 = record[8].parse::<f32>().unwrap();
          let level2 = record[9].parse::<f32>().unwrap();
          let time = record[4].parse::<f32>().unwrap();

          let d = data.entry(Key3d(
            HashableF32(level0),
            HashableF32(level1),
            HashableF32(level2),
          )).or_insert((0.0, 0));

          // calculate mean time for each level
          *d = (d.0 + time, d.1 + 1)
        },
        Err(e) => println!("Could not parse line from benchmark: {}", e),
      }
    }

    for (&Key3d(HashableF32(level0), HashableF32(level1), HashableF32(level2)), &(time_sum, time_count)) in &data {
      let time = time_sum / time_count as f32;

      vertices.push(Vertex {
        position: (level0, time * scale_factor, level2),
      });

      colors.push(Color {
        color: (level1, 0.0, 1.0 - level1,),
      });
    }

    (vertices, colors)
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

  pub fn set_visualize_mode(&mut self, mode: VisualizeMode) {
    self.mode = mode;
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

    let vertex_buffers = match self.mode {
      VisualizeMode::OneD   => (&self.points_1d, &self.colors_1d),
      VisualizeMode::TwoD   => (&self.points_2d, &self.colors_2d),
      VisualizeMode::ThreeD => (&self.points_3d, &self.colors_3d),
    };

    target.draw(
        vertex_buffers,
        NoIndices(PrimitiveType::Points),
        &self.program,
        &uniforms,
        &point_render_params).unwrap();
  }

  fn update(&mut self, _: &Facade, _: Matrix4<f32>, input_actions: &Vec<Action>) {
    for action in input_actions {
      match action {
        &Action::VisualizeOneD   => self.set_visualize_mode(VisualizeMode::OneD),
        &Action::VisualizeTwoD   => self.set_visualize_mode(VisualizeMode::TwoD),
        &Action::VisualizeThreeD => self.set_visualize_mode(VisualizeMode::ThreeD),
        _ => (),
      }
    }
  }
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
