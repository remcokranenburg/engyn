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

use cgmath::Matrix4;
use glium::backend::Facade;
use glium::DrawParameters;
use glium::framebuffer::SimpleFrameBuffer;
use glium::index::NoIndices;
use glium::index::PrimitiveType;
use glium::PolygonMode;
use glium::Program;
use glium::Surface;
use glium::VertexBuffer;
use rand;
use rand::distributions::IndependentSample;
use rand::distributions::Range;
use rand::Rng;
use std::f32;

use geometry::Vertex;
use gui::Action;
use light::Light;
use math;
use object::Drawable;

pub struct Node {
  pub vertex: Vertex,
  pub velocity: (f32, f32, f32),
  pub fixed: bool,
}

pub struct Network {
  pub nodes: Vec<Node>,
  pub links: Vec<(usize, usize)>,

  program: Program,
  nodes_buffer: VertexBuffer<Vertex>,

  // simulation
  alpha: f32,
  alpha_decay: f32,
  alpha_min: f32,
  alpha_target: f32,
  velocity_decay: f32,

  // gravity
  gravity_strength: f32,

  // many bodies
  many_bodies_strength: f32,
  many_bodies_distance_min2: f32,
  many_bodies_distance_max2: f32,
}

impl Network {
  pub fn new(context: &Facade, num_nodes: usize, num_links: usize) -> Network {
    let mut nodes = Vec::new();
    let mut links = Vec::new();

    Network::initialize_nodes(&mut nodes, num_nodes);
    Network::initialize_links(&mut links, num_nodes, num_links);

    let program = Program::from_source(
      context,
      &r#"
        #version 140

        uniform mat4 projection;
        uniform mat4 view;
        uniform mat4 model;

        in vec3 position;
        //in vec3 color;

        out vec3 v_color;

        void main() {
          vec4 position_global = model * vec4(position, 1.0);
          vec4 position_eye = view * position_global;

          v_color = vec3(1.0, 0.6, 0.0);

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

    Network {
      nodes: nodes,
      links: links,

      program: program,
      nodes_buffer: VertexBuffer::empty_dynamic(context, 0).unwrap(),

      alpha: 0.1,
      alpha_decay: 1.0 - f32::powf(0.001, 1.0 / 600.0),
      alpha_min: 0.001,
      alpha_target: 0.0,
      velocity_decay: 0.1,

      gravity_strength: 2.0,

      many_bodies_strength: -0.005,
      many_bodies_distance_min2: 0.01,
      many_bodies_distance_max2: 1.0,
    }
  }

  fn gravity_force(&mut self) {
    for node in &mut self.nodes {
      node.velocity.0 += -node.vertex.position.0 * self.gravity_strength * self.alpha;
      node.velocity.1 += -node.vertex.position.1 * self.gravity_strength * self.alpha;
      node.velocity.2 += -node.vertex.position.2 * self.gravity_strength * self.alpha;
    }
  }

  fn many_bodies_force(&mut self) {
    for i in 0 .. self.nodes.len() {
      for j in 0 .. self.nodes.len() {
        if i != j {
          let mut diff = (
              self.nodes[j].vertex.position.0 - self.nodes[i].vertex.position.0,
              self.nodes[j].vertex.position.1 - self.nodes[i].vertex.position.1,
              self.nodes[j].vertex.position.2 - self.nodes[i].vertex.position.2,
          );

          if diff.0 == 0.0 { diff.0 = Network::jiggle(); }
          if diff.1 == 0.0 { diff.1 = Network::jiggle(); }
          if diff.2 == 0.0 { diff.2 = Network::jiggle(); }

          let mut distance2 = diff.0 * diff.0 + diff.1 * diff.1 + diff.2 * diff.2;

          if distance2 >= self.many_bodies_distance_max2 {
            continue;
          }

          if distance2 < self.many_bodies_distance_min2 {
            distance2 = f32::sqrt(self.many_bodies_distance_min2 * distance2)
          }

          let change = self.many_bodies_strength * self.alpha / distance2;
          self.nodes[i].velocity.0 += diff.0 * change;
          self.nodes[i].velocity.1 += diff.1 * change;
          self.nodes[i].velocity.2 += diff.2 * change;
        }
      }
    }
  }

  fn jiggle() -> f32 {
    let mut rng = rand::thread_rng();
    (rng.next_f32() - 0.5) * 1e-6
  }

  fn initialize_nodes(nodes: &mut Vec<Node>, num_nodes: usize) {
    while nodes.len() < num_nodes {
      nodes.push(Node {
        vertex: Vertex { position: (Network::jiggle(), Network::jiggle(), Network::jiggle()) },
        velocity: (0.0, 0.0, 0.0),
        fixed: false,
      });
    }
  }

  fn initialize_links(links: &mut Vec<(usize, usize)>, num_nodes: usize, num_links: usize) {
    let mut rng = rand::thread_rng();
    let link_range = Range::new(0, num_nodes);

    while links.len() < num_links {
      let src_index = link_range.ind_sample(&mut rng);
      let dst_index = link_range.ind_sample(&mut rng);
      links.push((src_index, dst_index));
    }
  }
}

impl Drawable for Network {
  fn draw(&mut self, target: &mut SimpleFrameBuffer, projection: [[f32; 4]; 4], view: [[f32; 4]; 4],
      model_transform: Matrix4<f32>, render_params: &DrawParameters, _: i32, _: &[Light; 32],
      eye_i: usize, is_anaglyph: bool) {
    let uniforms = uniform! {
      projection: projection,
      view: view,
      model: math::matrix_to_uniform(model_transform),
    };

    let mut point_render_params = render_params.clone();
    point_render_params.point_size = Some(20.0);
    point_render_params.polygon_mode = PolygonMode::Point;

    target.draw(
        &self.nodes_buffer,
        NoIndices(PrimitiveType::Points),
        &self.program,
        &uniforms,
        &point_render_params).unwrap();
  }

  fn update(&mut self, context: &Facade, _: Matrix4<f32>, _: &Vec<Action>) {
    if self.alpha < self.alpha_min {
      let num_nodes = self.nodes.len();
      self.alpha = 0.1;
      self.nodes = Vec::new();
      Network::initialize_nodes(&mut self.nodes, num_nodes);
    }

    self.alpha += (self.alpha_target - self.alpha) * self.alpha_decay;

    self.gravity_force();
    self.many_bodies_force();

    for ref mut node in &mut self.nodes {
      if !node.fixed {
        node.velocity.0 *= self.velocity_decay;
        node.vertex.position.0 += node.velocity.0;
        node.velocity.1 *= self.velocity_decay;
        node.vertex.position.1 += node.velocity.1;
        node.velocity.2 *= self.velocity_decay;
        node.vertex.position.2 += node.velocity.2;
      }
    }

    self.nodes_buffer = VertexBuffer::empty_dynamic(context, self.nodes.len()).unwrap();

    let mut mapped = self.nodes_buffer.map();
    for (i, node) in self.nodes.iter().enumerate() {
      mapped[i] = node.vertex;
    }
  }
}
