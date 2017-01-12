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

#[macro_use] extern crate glium;
extern crate nalgebra;

mod teapot;

use glium::DisplayBuild;
use glium::Program;
use glium::Surface;
use glium::backend::glutin_backend::GlutinFacade;
use glium::glutin::Event;
use glium::glutin::VirtualKeyCode;
use glium::index::IndexBuffer;
use glium::index::NoIndices;
use glium::index::PrimitiveType;
use glium::vertex::VertexBuffer;

use nalgebra::Matrix4;

use teapot::Vertex;

#[derive(PartialEq)]
enum Action {
  Exit,
  Nothing,
}

struct Mesh {
  pub indices: Option<IndexBuffer<u16>>,
  pub vertices: VertexBuffer<Vertex>,
}

impl Mesh {
  fn new_quad(window: &GlutinFacade) -> Mesh {
    Mesh {
      indices: Some(IndexBuffer::new(window, PrimitiveType::TriangleStrip, &[1, 2, 0, 3u16]).unwrap()),
      vertices: VertexBuffer::new(window, &[
          Vertex { position: (-1.0, -1.0, -0.0) },
          Vertex { position: (-1.0,  1.0, -0.0) },
          Vertex { position: ( 1.0,  1.0, -0.0) },
          Vertex { position: ( 1.0, -1.0, -0.0) }]).unwrap(),
    }
  }
}

struct Object3d {
  pub mesh: Option<Mesh>,
  pub transform: Matrix4<f32>,
}

fn draw(window: &GlutinFacade, program: &Program, world: &mut Vec<Object3d>, frame_number: u32) {
  let i = frame_number as f32;
  let r = 0.5 + 0.5 * (i / 17.0).sin();
  let g = 0.5 + 0.5 * (i / 19.0).sin();
  let b = 0.5 + 0.5 * (i / 23.0).sin();

  let mut target = window.draw();
  target.clear_color(r, g, b, 1.0);

  for object in world {
    let x =  if (frame_number / 100) % 2 == 0 { 0.01 } else { -0.01 };

    let translation = Matrix4::new(
        1.0, 0.0, 0.0, x,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0);

    object.transform *= translation;

    match object.mesh {
      Some(ref m) => {
        let uniforms = uniform! {
          matrix: *object.transform.as_ref(),
        };

        match m.indices {
          Some(ref indices) => target.draw(&m.vertices, indices, program, &uniforms, &Default::default()).unwrap(),
          None => target.draw(&m.vertices, NoIndices(PrimitiveType::TrianglesList), program, &uniforms, &Default::default()).unwrap(),
        }
      },
      None => ()
    }
  }

  target.finish().unwrap();
}

fn handle_key(key_code: VirtualKeyCode) -> Action {
  match key_code {
    VirtualKeyCode::Escape => Action::Exit,
    _ => Action::Nothing,
  }
}

fn main() {
  let window = glium::glutin::WindowBuilder::new()
    .with_title(format!("Engine"))
    .with_vsync()
    .build_glium()
    .unwrap();

  let vshader_src = r#"
    #version 140

    in vec3 position;
    uniform mat4 matrix;

    void main() {
      gl_Position = matrix * vec4(position, 1.0);
    }
  "#;

  let fshader_src = r#"
    #version 140

    out vec4 color;

    void main() {
      color = vec4(1.0, 0.0, 0.0, 1.0);
    }
  "#;

  let program = Program::from_source(&window, vshader_src, fshader_src, None).unwrap();

  let mut world = Vec::new();

  let my_triangle = Object3d {
    mesh: Some(Mesh {
      indices: None,
      vertices: VertexBuffer::new(&window, &[
          Vertex { position: (-0.50, -0.50, 0.00) },
          Vertex { position: ( 0.00,  0.50, 0.00) },
          Vertex { position: ( 0.50, -0.25, 0.00) } ]).unwrap(),
    }),
    transform: Matrix4::new(1.0, 0.0, 0.0, -0.5,
                            0.0, 1.0, 0.0, -0.5,
                            0.0, 0.0, 1.0,  0.0,
                            0.0, 0.0, 0.0,  1.0),
  };

  world.push(my_triangle);

  let my_floor = Object3d {
    mesh: Some(Mesh::new_quad(&window)),
    transform: Matrix4::new(0.1, 0.0, 0.0, -0.5,
                            0.0, 0.1, 0.0,  0.5,
                            0.0, 0.0, 0.1,  0.0,
                            0.0, 0.0, 0.0,  1.0),
  };

  world.push(my_floor);

  let my_teapot = Object3d {
    mesh: Some(Mesh {
      indices: Some(IndexBuffer::new(&window, PrimitiveType::TrianglesList, &teapot::INDICES).unwrap()),
      vertices: VertexBuffer::new(&window, &teapot::VERTICES).unwrap(),
    }),
    transform: Matrix4::new(0.003, 0.00, 0.00, 0.0,
                            0.00, 0.004, 0.00, 0.0,
                            0.00, 0.00, 0.01, 0.0,
                            0.00, 0.00, 0.00, 1.0),
  };

  world.push(my_teapot);

  let mut frame_number = 0;

  loop {
    let mut action = Action::Nothing;

    draw(&window, &program, &mut world, frame_number);

    for event in window.poll_events() {
      action = match event {
        Event::Closed => Action::Exit,
        Event::KeyboardInput(_, _, key_code) => handle_key(key_code.unwrap()),
        Event::Resized(width, height) => {
          println!("resized to {}x{}", width, height);
          Action::Nothing
        }
        _ => Action::Nothing
      };

      if action != Action::Nothing { break }
    }

    match action {
      Action::Exit => {
        println!("Exiting...");
        break;
      },
      Action::Nothing => ()
    };

    frame_number += 1;
  }
}
