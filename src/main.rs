extern crate nalgebra;
#[macro_use] extern crate glium;

use glium::DisplayBuild;
use glium::Program;
use glium::Surface;
use glium::backend::glutin_backend::GlutinFacade;
use glium::glutin::VirtualKeyCode;
use glium::index::IndexBuffer;
use glium::index::NoIndices;
use glium::index::PrimitiveType::TrianglesList;
use glium::vertex::VertexBuffer;

use nalgebra::Matrix4;

#[derive(Copy, Clone)]
struct Vertex {
  position: [f32; 3],
}

implement_vertex!(Vertex, position);

struct Mesh {
  pub indices: Option<IndexBuffer<u16>>,
  pub program: Program,
  pub vertices: VertexBuffer<Vertex>,
}

struct Object3d {
  pub children: Vec<Object3d>,
  pub mesh: Option<Mesh>,
  pub transform_local: Matrix4<f32>,
  pub transform_global: Matrix4<f32>,
}

fn draw(
    window: &GlutinFacade,
    object: &mut Object3d,
    frame_number: u32) {
  let i = frame_number as f32;
  let r = 0.5 + 0.5 * (i / 17.0).sin();
  let g = 0.5 + 0.5 * (i / 19.0).sin();
  let b = 0.5 + 0.5 * (i / 23.0).sin();

  let mut target = window.draw();
  target.clear_color(r, g, b, 1.0);

  let x =  if (frame_number / 100) % 2 == 0 { 0.01 } else { -0.01 };

  let translation = Matrix4::new(
      1.0, 0.0, 0.0, x,
      0.0, 1.0, 0.0, 0.0,
      0.0, 0.0, 1.0, 0.0,
      0.0, 0.0, 0.0, 1.0);

  object.transform_global *= translation;

  match object.mesh {
    Some(ref m) => {
      let uniforms = uniform! {
        matrix: *object.transform_global.as_ref(),
      };

      match m.indices {
        Some(ref i) => target.draw(&m.vertices, i, &m.program, &uniforms, &Default::default()).unwrap(),
        None => target.draw(&m.vertices, &NoIndices(TrianglesList), &m.program, &uniforms, &Default::default()).unwrap(),
      }
    },
    None => ()
  }

  target.finish().unwrap();
}

fn handle_key(key_code: VirtualKeyCode, done: &mut bool) {
  match key_code {
    VirtualKeyCode::Escape => *done = true,
    _ => (),
  }
}

fn main() {
  let monitors = glium::glutin::get_available_monitors();

  println!("Available monitors:");

  for m in monitors {
    let (w, h) = m.get_dimensions();
    let n = m.get_name().unwrap();
    println!("Monitor: name={}, dimensions={}x{}", n, w, h);
  }

  let monitor = glium::glutin::get_primary_monitor();
  let (width, height) = monitor.get_dimensions();

  let window = glium::glutin::WindowBuilder::new()
    .with_fullscreen(monitor)
    .with_dimensions(width, height)
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

  let mut my_triangle = Object3d {
    children: Vec::new(),
    mesh: Some(Mesh {
      indices: None,
      program: Program::from_source(&window, vshader_src, fshader_src, None).unwrap(),
      vertices: VertexBuffer::new(&window, &[
          Vertex { position: [-0.50, -0.50, 0.00] },
          Vertex { position: [ 0.00,  0.50, 0.00] },
          Vertex { position: [ 0.50, -0.25, 0.00] } ]).unwrap(),
    }),
    transform_local: nalgebra::new_identity(4),
    transform_global: Matrix4::new(
        1.0, 0.0, 0.0, -0.5,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0),
  };

  let mut frame_number = 0;
  let mut done = false;

  while !done {
    draw(&window, &mut my_triangle, frame_number);

    // listing the events produced by the window and waiting to be received
    for ev in window.poll_events() {
      match ev {
        glium::glutin::Event::Closed => {
          done = true;
        },
        glium::glutin::Event::KeyboardInput(_, _, key_code) => {
          handle_key(key_code.unwrap(), &mut done);
        },
        glium::glutin::Event::Resized(width, height) => {
          println!("resized to {}x{}", width, height);
        }
        _ => ()
      }
    }

    frame_number += 1;
  }

  println!("Exiting...");
}
