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
extern crate cgmath;
extern crate rand;
extern crate rust_webvr as webvr;

mod geometry;
mod material;
mod mesh;
mod object;
mod teapot;

use cgmath::Matrix4;
use cgmath::SquareMatrix;
use cgmath::Transform;
use cgmath::Vector3;
use glium::Depth;
use glium::DepthTest;
use glium::DisplayBuild;
use glium::DrawParameters;
use glium::GlObject;
use glium::Program;
use glium::Rect;
use glium::Surface;
use glium::backend::glutin_backend::GlutinFacade;
use glium::framebuffer::DepthRenderBuffer;
use glium::framebuffer::SimpleFrameBuffer;
use glium::framebuffer::ToColorAttachment;
use glium::framebuffer::ToDepthAttachment;
use glium::glutin::Event;
use glium::glutin::VirtualKeyCode;
use glium::index::IndexBuffer;
use glium::index::NoIndices;
use glium::index::PrimitiveType;
use glium::texture::DepthFormat;
use glium::texture::RawImage2d;
use glium::texture::SrgbTexture2d;
use glium::texture::Texture2d;
use glium::vertex::VertexBuffer;
use std::path::Path;
use webvr::VRDisplayEvent;
use webvr::VRLayer;
use webvr::VRServiceManager;

use geometry::Geometry;
use geometry::Normal;
use geometry::Texcoord;
use geometry::Vertex;
use material::Material;
use mesh::Mesh;
use object::Object;

#[derive(PartialEq)]
enum Action {
  Exit,
  Nothing,
}

fn load_texture(window: &GlutinFacade, name: &str) -> SrgbTexture2d {
  let image = image::open(&Path::new(&name)).unwrap().to_rgba();
  let image_dimensions = image.dimensions();
  let image = RawImage2d::from_raw_rgba_reversed(image.into_raw(), image_dimensions);
  SrgbTexture2d::new(window, image).unwrap()
}

fn vec_to_matrix(m: &[f32; 16]) -> Matrix4<f32> {
  Matrix4::new(
      m[0], m[1], m[2], m[3],
      m[4], m[5], m[6], m[7],
      m[8], m[9], m[10], m[11],
      m[12], m[13], m[14], m[15])
}

fn matrix_to_uniform<'a>(m: &'a Matrix4<f32>) -> &'a [[f32; 4]; 4] {
  m.as_ref()
}

fn vec_to_translation(t: &[f32; 3]) -> Matrix4<f32> {
    Matrix4::from_translation(Vector3::new(t[0], t[1], t[2]))
}

fn handle_key(key_code: VirtualKeyCode) -> Action {
  match key_code {
    VirtualKeyCode::Escape => Action::Exit,
    _ => Action::Nothing,
  }
}

fn main() {
  let mut vr = VRServiceManager::new();
  vr.register_defaults();
  vr.initialize_services();

  let displays = vr.get_displays();

  let display = match displays.get(0) {
    Some(d) => {
      println!("VR display 0: {}", d.borrow().data().display_name);
      d
    },
    None => {
      println!("Could not select VR device! Can't continue.");
      return
    }
  };

  let display_data = display.borrow().data();

  let render_width = display_data.left_eye_parameters.render_width;
  let render_height = display_data.left_eye_parameters.render_height;
  let window_width = render_width;
  let window_height = (render_height as f32 * 0.5) as u32;

  let window = glium::glutin::WindowBuilder::new()
    .with_title(format!("Engyn"))
    .with_depth_buffer(24)
    .with_vsync()
    .with_dimensions(window_width, window_height)
    .build_glium()
    .unwrap();


  println!("Loading textures...");
  let empty_tex = load_texture(&window, "data/empty.bmp");
  let marble_tex = load_texture(&window, "data/marble.jpg");
  println!("Textures loaded!");

  let target_texture = Texture2d::empty(&window, render_width * 2, render_height).unwrap();
  let color_attachment = target_texture.to_color_attachment();
  let depth_buffer = DepthRenderBuffer::new(&window, DepthFormat::I24, render_width * 2,
      render_height).unwrap();
  let depth_attachment = depth_buffer.to_depth_attachment();
  let mut framebuffer = SimpleFrameBuffer::with_depth_buffer(&window, color_attachment,
      depth_attachment).unwrap();

  let left_viewport = Rect {
      left: 0,
      bottom: 0,
      width: render_width,
      height: render_height,
  };

  let right_viewport = Rect {
      left: render_width,
      bottom: 0,
      width: render_width,
      height: render_height,
  };

  let render_program = Program::from_source(
      &window,
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

        void main() {
          v_texcoord = texcoord;
          v_normal = normal;
          gl_Position = projection * view * model * vec4(position, 1.0);
        }
      "#,
      &r#"
        #version 140

        uniform sampler2D albedo_map;
        in vec3 v_normal;
        in vec2 v_texcoord;
        out vec4 color;

        void main() {
          color = texture(albedo_map, v_texcoord);
        }
      "#,
      None).unwrap();

  let compositor_program = Program::from_source(
      &window,
      &r#"
        #version 140
        uniform mat4 matrix;
        in vec3 position;
        in vec2 texcoord;
        out vec2 v_texcoord;
        void main() {
          v_texcoord = texcoord;
          gl_Position = matrix * vec4(position, 1.0);
        }
      "#,
      &r#"
        #version 140

        uniform sampler2D sampler;

        in vec2 v_texcoord;
        out vec4 color;

        void main() {
          color = texture(sampler, v_texcoord);
        }
      "#,
      None).unwrap();

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
      material: Material { albedo_map: &marble_tex, metalness: 0.0, reflectivity: 0.0 },
    }),
    transform: Matrix4::<f32>::identity(),
  };

  world.push(my_triangle);

  let my_floor = Object {
    mesh: Some(Mesh {
      geometry: Geometry::new_quad(&window, [2.0, 2.0]),
      material: Material { albedo_map: &marble_tex, metalness: 0.0, reflectivity: 0.0 },
    }),
    transform: Matrix4::new(0.1, 0.0, 0.0, 0.0,
                            0.0, 0.1, 0.0, 0.0,
                            0.0, 0.0, 0.1, 0.0,
                            0.0, 0.0, 1.0, 1.0),
  };

  world.push(my_floor);

  // teapot

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
        indices: Some(IndexBuffer::new(
            &window,
            PrimitiveType::TrianglesList,
            &teapot::INDICES).unwrap()),
        normals: VertexBuffer::new(&window, &teapot::NORMALS).unwrap(),
        vertices: VertexBuffer::new(&window, &teapot::VERTICES).unwrap(),
        texcoords: VertexBuffer::new(&window, &my_teapot_texcoords).unwrap(),
      },
      material: Material { albedo_map: &marble_tex, metalness: 0.0, reflectivity: 0.0 },
    }),
    transform: Matrix4::new(
        0.005, 0.0, 0.0, 0.0,
        0.0, 0.005, 0.0, 0.0,
        0.0, 0.0, 0.005, 0.0,
        0.0, 1.0, 0.0, 1.0),
  };

  world.push(my_teapot);

  // empty texture to force glutin clean
  world.push(Object::new_plane(&window, &empty_tex, [0.0001,0.0001], [-0.1, 0.1, 0.0],
      [0.0, 0.0, 0.0], [-1.0,1.0,1.0]));

  let fbo_to_screen = Geometry::new_quad(&window, [2.0, 2.0]);

  let mut render_params = DrawParameters {
    depth: Depth { test: DepthTest::IfLess, write: true, .. Default::default() },
    .. Default::default()
  };

  let mut event_counter = 0u64;

  loop {
    let mut action = Action::Nothing;

    display.borrow_mut().sync_poses();

    let display_data = display.borrow().data();

    let standing_transform = if let Some(ref stage) = display_data.stage_parameters {
        vec_to_matrix(&stage.sitting_to_standing_transform).inverse_transform().unwrap()
    } else {
        // Stage parameters not avaialbe yet or unsupported
        // Assume 0.75m transform height
        vec_to_translation(&[0.0, 0.75, 0.0]).inverse_transform().unwrap()
    };

    framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

    let data = display.borrow().synced_frame_data(0.1, 1000.0);

    let left_view_matrix = vec_to_matrix(&data.left_view_matrix);
    let right_view_matrix = vec_to_matrix(&data.right_view_matrix);

    let eyes = [
      (&left_viewport, &data.left_projection_matrix, &left_view_matrix),
      (&right_viewport, &data.right_projection_matrix, &right_view_matrix),
    ];

    for eye in &eyes {
      render_params.viewport = Some(*eye.0);
      let projection = vec_to_matrix(eye.1);
      let eye_view = eye.2 * standing_transform;

      for object in &*world {
        match object.mesh {
          Some(ref m) => {
            let uniforms = uniform! {
              projection: *matrix_to_uniform(&projection),
              view: *matrix_to_uniform(&eye_view),
              model: *matrix_to_uniform(&object.transform),
              albedo_map: m.material.albedo_map,
              metalness: m.material.metalness,
              reflectivity: m.material.reflectivity,
            };

            match m.geometry.indices {
              Some(ref indices) => framebuffer.draw(
                  (&m.geometry.vertices, &m.geometry.normals, &m.geometry.texcoords),
                  indices,
                  &render_program,
                  &uniforms,
                  &render_params).unwrap(),
              None => framebuffer.draw(
                  (&m.geometry.vertices, &m.geometry.normals, &m.geometry.texcoords),
                  NoIndices(PrimitiveType::TrianglesList),
                  &render_program,
                  &uniforms,
                  &render_params).unwrap(),
            }
          },
          None => (),
        }
      }
    }

    let layer = VRLayer {
      texture_id: target_texture.get_id(),
      ..Default::default()
    };

    display.borrow_mut().submit_frame(&layer);

    // now render to desktop display

    let mut target = window.draw();
    target.clear_color_and_depth((1.0, 0.0, 0.0, 1.0), 1.0);

    let uniforms = uniform! {
        matrix: *matrix_to_uniform(&Matrix4::<f32>::identity()),
        sampler: &target_texture
    };

    target.draw(
        (
            &fbo_to_screen.vertices,
            &fbo_to_screen.texcoords
        ),
        fbo_to_screen.borrow_indices().unwrap(),
        &compositor_program,
        &uniforms,
        &Default::default()).unwrap();

    target.finish().unwrap();

    assert_no_gl_error!(window);

    // once every 100 frames, check for VR events
    event_counter += 1;
    if event_counter % 100 == 0 {
      for event in vr.poll_events() {
        match event {
          VRDisplayEvent::Connect(data) => {
            println!("VR display {}: Connected (name: {})", data.display_id, data.display_name);
          },
          VRDisplayEvent::Disconnect(display_id) => {
            println!("VR display {}: Disconnected.", display_id);
          },
          VRDisplayEvent::Activate(data, _) => {
            println!("VR display {}: Activated.", data.display_id);
          },
          VRDisplayEvent::Deactivate(data, _) => {
            println!("VR display {}: Deactivated.", data.display_id);
          },
          _ => println!("VR event: {:?}", event),
        }
      }
    }

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
  }
}
