#[macro_use]

extern crate glium;

use glium::DisplayBuild;
use glium::Surface;
use glium::backend::glutin_backend::GlutinFacade;
use glium::glutin::VirtualKeyCode;
use glium::Program;
use glium::index::NoIndices;
use glium::vertex::VertexBuffer;

#[derive(Copy, Clone)]
struct Vertex {
  position: [f32; 2],
}

implement_vertex!(Vertex, position);

fn draw(
    window: &GlutinFacade,
    vertex_buffer: &VertexBuffer<Vertex>,
    indices: &NoIndices,
    program: &Program,
    frame_number: &mut u32) {
  let i = *frame_number as f32;
  let r = 0.5 + 0.5 * (i / 17.0).sin();
  let g = 0.5 + 0.5 * (i / 19.0).sin();
  let b = 0.5 + 0.5 * (i / 23.0).sin();

  let mut target = window.draw();
  target.clear_color(r, g, b, 1.0);

  let t = i / 100.0 % 1.0 - 0.5;

  target.draw(vertex_buffer, indices, program, &uniform! { t: t }, &Default::default()).unwrap();
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

    in vec2 position;

    uniform float t;

    void main() {
      vec2 pos = position;
      pos.x += t;
      gl_Position = vec4(pos, 0.0, 1.0);
    }
  "#;

  let fshader_src = r#"
    #version 140

    out vec4 color;

    void main() {
      color = vec4(1.0, 0.0, 0.0, 1.0);
    }
  "#;

  let program = glium::Program::from_source(&window, vshader_src, fshader_src, None).unwrap();

  let v1 = Vertex { position: [-0.5, -0.5] };
  let v2 = Vertex { position: [ 0.0,  0.5] };
  let v3 = Vertex { position: [ 0.5, -0.25] };
  let shape = vec![v1, v2, v3];

  let vertex_buffer = glium::VertexBuffer::new(&window, &shape).unwrap();
  let indices = glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList);

  let mut frame_number = 0;
  let mut done = false;

  while !done {
    draw(&window, &vertex_buffer, &indices, &program, &mut frame_number);

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
