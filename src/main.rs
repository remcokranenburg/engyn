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
extern crate image;
extern crate nalgebra;
extern crate rand;

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
use glium::texture::Texture2d;
use glium::texture::RawImage2d;
use glium::vertex::VertexBuffer;

use nalgebra::Matrix4;

use std::fs::File;
use std::io::Cursor;
use std::io::prelude::*;

use teapot::Vertex;
use teapot::Normal;

#[derive(Copy, Clone)]
pub struct Texcoord {
    pub texcoord: (f32, f32)
}

implement_vertex!(Texcoord, texcoord);

#[derive(PartialEq)]
enum Action {
  Exit,
  Nothing,
}

struct Geometry {
  pub indices: Option<IndexBuffer<u16>>,
  pub normals: VertexBuffer<Normal>,
  pub vertices: VertexBuffer<Vertex>,
  pub texcoords: VertexBuffer<Texcoord>,
}

impl Geometry {
  fn new_plane(window: &GlutinFacade, width: f32, height: f32) -> Geometry {
    let width_half = width / 2.0;
    let height_half = height / 2.0;

    Geometry {
      indices: Some(IndexBuffer::new(window, PrimitiveType::TriangleStrip, &[1, 2, 0, 3u16]).unwrap()),
      normals: VertexBuffer::new(window, &[
          Normal { normal: (0.0, 0.0, 1.0) },
          Normal { normal: (0.0, 0.0, 1.0) },
          Normal { normal: (0.0, 0.0, 1.0) },
          Normal { normal: (0.0, 0.0, 1.0) }]).unwrap(),
      vertices: VertexBuffer::new(window, &[
          Vertex { position: (-width_half, -height_half, 0.0) },
          Vertex { position: (-width_half,  height_half, 0.0) },
          Vertex { position: ( width_half,  height_half, 0.0) },
          Vertex { position: ( width_half, -height_half, 0.0) }]).unwrap(),
      texcoords: VertexBuffer::new(window, &[
          Texcoord { texcoord: (0.0, 0.0) },
          Texcoord { texcoord: (0.0, 1.0) },
          Texcoord { texcoord: (1.0, 1.0) },
          Texcoord { texcoord: (1.0, 0.0) }]).unwrap(),
    }
  }
}

struct Material {
  pub albedo_map: Texture2d,
  pub metalness: f32,
  pub reflectivity: f32,
}

impl Material {
  fn new(window: &GlutinFacade, albedo_map: &str, metalness: f32, reflectivity: f32) -> Material {
    fn load_texture(window: &GlutinFacade, filename: &str) -> Texture2d {
      let mut f = File::open(filename).expect("no such file");
      let mut buf = Vec::new();
      f.read_to_end(&mut buf).expect("could not read from file");
      let image = image::load(Cursor::new(&buf), image::JPEG).unwrap();
      let image = image.to_rgba();
      let image_dimensions = image.dimensions();
      let image = RawImage2d::from_raw_rgba_reversed(image.into_raw(), image_dimensions);

      Texture2d::new(window, image).unwrap()
    }

    Material {
      albedo_map: load_texture(window, albedo_map),
      metalness: metalness,
      reflectivity: reflectivity,
    }
  }
}

struct Mesh {
  pub geometry: Geometry,
  pub material: Material,
}

struct Object {
  pub mesh: Option<Mesh>,
  pub transform: Matrix4<f32>,
}

fn draw(window: &GlutinFacade, program: &Program, world: &mut Vec<Object>, frame_number: u32) {
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
          albedo_map: &m.material.albedo_map,
        };

        match m.geometry.indices {
          Some(ref indices) => target.draw(
              (&m.geometry.vertices, &m.geometry.normals, &m.geometry.texcoords),
              indices,
              program,
              &uniforms,
              &Default::default()).unwrap(),
          None => target.draw(
              (&m.geometry.vertices, &m.geometry.normals, &m.geometry.texcoords),
              NoIndices(PrimitiveType::TrianglesList),
              program,
              &uniforms,
              &Default::default()).unwrap(),
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
    in vec3 normal;
    in vec2 texcoord;
    out vec3 v_normal;
    out vec2 v_texcoord;
    uniform mat4 matrix;

    void main() {
      v_texcoord = texcoord;
      v_normal = normal;
      gl_Position = matrix * vec4(position, 1.0);
    }
  "#;

  let fshader_src = r#"
    #version 140

    in vec3 v_normal;
    in vec2 v_texcoord;
    out vec4 color;
    uniform sampler2D albedo_map;

    void main() {
      color = texture(albedo_map, v_texcoord);
    }
  "#;

  let program = Program::from_source(&window, vshader_src, fshader_src, None).unwrap();

  let mut world = Vec::new();

  let my_triangle = Object {
    mesh: Some(Mesh {
      geometry: Geometry {
        indices: None,
        normals: VertexBuffer::new(&window, &[
            Normal { normal: (0.0, 0.0, 1.0) },
            Normal { normal: (0.0, 0.0, 1.0) },
            Normal { normal: (0.0, 0.0, 1.0) }]).unwrap(),
        vertices: VertexBuffer::new(&window, &[
            Vertex { position: (-0.50, -0.50, 0.00) },
            Vertex { position: ( 0.50, -0.50, 0.00) },
            Vertex { position: ( 0.00,  0.50, 0.00) } ]).unwrap(),
        texcoords: VertexBuffer::new(&window, &[
            Texcoord { texcoord: (0.0, 0.0) },
            Texcoord { texcoord: (1.0, 0.0) },
            Texcoord { texcoord: (0.5, 1.0) },
          ]).unwrap(),
      },
      material: Material::new(&window, "data/marble.jpg", 0.0, 0.2),
    }),
    transform: Matrix4::new(1.0, 0.0, 0.0, -0.5,
                            0.0, 1.0, 0.0, -0.5,
                            0.0, 0.0, 1.0,  0.0,
                            0.0, 0.0, 0.0,  1.0),
  };

  world.push(my_triangle);

  let my_floor = Object {
    mesh: Some(Mesh {
      geometry: Geometry::new_plane(&window, 2.0, 2.0),
      material: Material::new(&window, "data/marble.jpg", 0.0, 0.2),
    }),
    transform: Matrix4::new(0.1, 0.0, 0.0, -0.5,
                            0.0, 0.1, 0.0,  0.5,
                            0.0, 0.0, 0.1,  0.0,
                            0.0, 0.0, 0.0,  1.0),
  };

  world.push(my_floor);

  let my_teapot_texcoords = {
    let mut texcoords = [Texcoord { texcoord: (0.0, 0.0) }; 531];

    for i in 0..texcoords.len() {
      texcoords[i].texcoord = rand::random::<(f32, f32)>();
    }

    texcoords
  };

  let my_teapot = Object {
    mesh: Some(Mesh {
      geometry: Geometry {
        indices: Some(IndexBuffer::new(&window, PrimitiveType::TrianglesList, &teapot::INDICES).unwrap()),
        normals: VertexBuffer::new(&window, &teapot::NORMALS).unwrap(),
        vertices: VertexBuffer::new(&window, &teapot::VERTICES).unwrap(),
        texcoords: VertexBuffer::new(&window, &my_teapot_texcoords).unwrap(),
      },
      material: Material::new(&window, "data/marble.jpg", 0.0, 0.2),
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
