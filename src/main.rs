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
extern crate tobj;

mod camera;
mod geometry;
mod material;
mod math;
mod mesh;
mod object;
mod teapot;

use cgmath::Deg;
use cgmath::Matrix4;
use cgmath::Quaternion;
use cgmath::Rad;
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
use glium::glutin::MouseCursor;
use glium::glutin::CursorState;
use glium::glutin::ElementState;
use glium::glutin::VirtualKeyCode;
use glium::glutin::WindowBuilder;
use glium::index::IndexBuffer;
use glium::index::PrimitiveType;
use glium::texture::DepthFormat;
use glium::texture::RawImage2d;
use glium::texture::SrgbTexture2d;
use glium::texture::Texture2d;
use glium::vertex::VertexBuffer;
use std::path::Path;
use std::f32;
use webvr::VRDisplayEvent;
use webvr::VRLayer;
use webvr::VRServiceManager;

use camera::FpsCamera;
use geometry::Geometry;
use geometry::Normal;
use geometry::Texcoord;
use geometry::Vertex;
use material::Material;
use mesh::Mesh;
use object::Object;

fn load_texture(context: &GlutinFacade, name: &str) -> SrgbTexture2d {
  let image = image::open(&Path::new(&name)).unwrap().to_rgba();
  let image_dimensions = image.dimensions();
  let image = RawImage2d::from_raw_rgba_reversed(image.into_raw(), image_dimensions);
  SrgbTexture2d::new(context, image).unwrap()
}

fn main() {
  let mut vr = VRServiceManager::new();
  vr.register_defaults();
  vr.initialize_services();

  let displays = vr.get_displays();

  let (vr_mode, context, mut render_dimensions) = match displays.get(0) {
    Some(d) => {
      let data = d.borrow().data();
      println!("VR display 0: {}", data.display_name);

      let render_width = data.left_eye_parameters.render_width;
      let render_height = data.left_eye_parameters.render_height;
      let window_width = render_width;
      let window_height = (render_height as f32 * 0.5) as u32;

      let context = WindowBuilder::new()
        .with_title(format!("Engyn"))
        .with_depth_buffer(24)
        .with_vsync()
        .with_dimensions(window_width, window_height)
        .build_glium()
        .unwrap();

      (true, context, (render_width, render_height))
    },
    None => {
      println!("No VR device detected. Continuing in normal mode.");
      let context = WindowBuilder::new()
        .with_title(format!("Engyn"))
        .with_depth_buffer(24)
        .with_vsync()
        .with_dimensions(1280, 720)
        .build_glium()
        .unwrap();

      let (width, height) = {
        let window = context.get_window().unwrap();
        let (width, height) = window.get_inner_size_pixels().unwrap();
        let origin_x = width as i32 / 2;
        let origin_y = height as i32 / 2;
        window.set_cursor_position(origin_x, origin_y).unwrap();
        window.set_cursor(MouseCursor::NoneCursor);
        window.set_cursor_state(CursorState::Grab).ok().expect("Could not grab mouse cursor");
        (width, height)
      };

      (false, context, (width, height))
    },
  };

  println!("Loading textures...");
  let empty_tex = load_texture(&context, "data/empty.bmp");
  let marble_tex = load_texture(&context, "data/marble.jpg");
  let terrain_tex = load_texture(&context, "data/terrain.png");
  println!("Textures loaded!");

  let target_texture = Texture2d::empty(&context, render_dimensions.0 * 2,
      render_dimensions.1).unwrap();
  let color_attachment = target_texture.to_color_attachment();
  let depth_buffer = DepthRenderBuffer::new(&context, DepthFormat::I24, render_dimensions.0 * 2,
      render_dimensions.1).unwrap();
  let depth_attachment = depth_buffer.to_depth_attachment();
  let mut framebuffer = SimpleFrameBuffer::with_depth_buffer(&context, color_attachment,
      depth_attachment).unwrap();

  let left_viewport = Rect {
      left: 0,
      bottom: 0,
      width: render_dimensions.0,
      height: render_dimensions.1,
  };

  let right_viewport = Rect {
      left: render_dimensions.0,
      bottom: 0,
      width: render_dimensions.0,
      height: render_dimensions.1,
  };

  let render_program = Program::from_source(
      &context,
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
      &context,
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

  // a triangle
  world.push(Object::new_triangle(&context, &marble_tex, [1.0, 1.0], [0.0, 0.0, 0.0],
      [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]));

  // a terrain mesh
  world.push(Object {
    mesh: Some(Mesh {
      geometry: Geometry::from_obj(&context, "data/terrain.obj"),
      material: Material { albedo_map: &terrain_tex, metalness: 0.0, reflectivity: 0.0 },
    }),
    transform: Matrix4::<f32>::identity(),
  });

  // a teapot

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
            &context,
            PrimitiveType::TrianglesList,
            &teapot::INDICES).unwrap()),
        normals: VertexBuffer::new(&context, &teapot::NORMALS).unwrap(),
        vertices: VertexBuffer::new(&context, &teapot::VERTICES).unwrap(),
        texcoords: VertexBuffer::new(&context, &my_teapot_texcoords).unwrap(),
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
  world.push(Object::new_plane(&context, &empty_tex, [0.0001,0.0001], [-0.1, 0.1, 0.0],
      [0.0, 0.0, 0.0], [-1.0,1.0,1.0]));

  let fbo_to_screen = Geometry::new_quad(&context, [2.0, 2.0]);

  let mut render_params = DrawParameters {
    depth: Depth { test: DepthTest::IfLess, write: true, .. Default::default() },
    .. Default::default()
  };

  let mut event_counter = 0u64;

  let mut fps_camera = FpsCamera::new(Vector3::new(0.0, 1.8, 3.0));

  let window = context.get_window().unwrap();

  // create a model for each gamepad
  let gamepads = vr.get_gamepads();
  let mut gamepad_models = Vec::new();

  println!("Found {} controller{}!", gamepads.len(), match gamepads.len() { 1 => "", _ => "s" });

  for gamepad in &gamepads {
    println!("We've found a gamepad!");
    gamepad_models.push(Object {
      mesh: Some(Mesh {
        geometry: Geometry::from_obj(&context, "data/vive-controller.obj"),
        material: Material { albedo_map: &marble_tex, metalness: 0.0, reflectivity: 0.0 },
      }),
      transform: Matrix4::<f32>::identity(),
    });
  }

  loop {
    let aspect_ratio = render_dimensions.0 as f32 / render_dimensions.1 as f32;
    let mono_projection = cgmath::perspective(Deg(45.0), aspect_ratio, 0.01f32, 1000.0);

    match displays.get(0) {
      Some(d) => {
        d.borrow_mut().sync_poses();

        let data = d.borrow().data();

        let standing_transform = if let Some(ref stage) = data.stage_parameters {
            math::vec_to_matrix(&stage.sitting_to_standing_transform).inverse_transform().unwrap()
        } else {
            // Stage parameters not avaialbe yet or unsupported
            // Assume 0.75m transform height
            math::vec_to_translation(&[0.0, 0.75, 0.0]).inverse_transform().unwrap()
        };

        framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

        let data = d.borrow().synced_frame_data(0.1, 1000.0);

        let left_view_matrix = math::vec_to_matrix(&data.left_view_matrix);
        let right_view_matrix = math::vec_to_matrix(&data.right_view_matrix);

        let eyes = [
          (&left_viewport, &data.left_projection_matrix, &left_view_matrix),
          (&right_viewport, &data.right_projection_matrix, &right_view_matrix),
        ];

        for eye in &eyes {
          let projection = math::matrix_to_uniform(math::vec_to_matrix(eye.1));
          let view = math::matrix_to_uniform(eye.2 * standing_transform);
          let viewport = Some(*eye.0);

          render_params.viewport = viewport;

          for object in &mut world {
            object.draw(&mut framebuffer, projection, view, &render_program, &render_params);
          }

          let inverse_standing_transform = standing_transform.inverse_transform().unwrap();

          for (i, ref gamepad) in gamepads.iter().enumerate() {
            let pose = gamepad.borrow().state().pose;
            let rotation = match pose.orientation {
              Some(o) => Matrix4::from(Quaternion::new(o[3], o[0], o[1], o[2])), // WebVR presents quaternions as (x, y, z, w)
              None => Matrix4::<f32>::identity(),
            };
            let position = match pose.position {
              Some(position) => Matrix4::from_translation(Vector3::from(position)),
              None => Matrix4::<f32>::identity(),
            };

            gamepad_models[i].transform = inverse_standing_transform * position * rotation;
            gamepad_models[i].draw(&mut framebuffer, projection, view, &render_program, &render_params);
          }
        }

        let layer = VRLayer {
          texture_id: target_texture.get_id(),
          ..Default::default()
        };

        d.borrow_mut().submit_frame(&layer);

        // now draw the framebuffer as a texture to the window

        let mut target = context.draw();
        target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

        let uniforms = uniform! {
            matrix: math::matrix_to_uniform(Matrix4::<f32>::identity()),
            sampler: &target_texture
        };

        target.draw(
            (&fbo_to_screen.vertices, &fbo_to_screen.texcoords),
            fbo_to_screen.borrow_indices().unwrap(),
            &compositor_program,
            &uniforms,
            &Default::default()).unwrap();

        target.finish().unwrap();

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
      },
      None => {
        // draw the scene normally
        let mut target = context.draw();
        target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

        let projection = math::matrix_to_uniform(mono_projection);
        // TODO: get actual timedelta
        let view = math::matrix_to_uniform(fps_camera.get_view(0.016));
        let viewport = None;

        render_params.viewport = viewport;

        for object in &mut world {
          object.draw(&mut target, projection, view, &render_program, &render_params);
        }

        target.finish().unwrap();
      }
    }

    assert_no_gl_error!(context);

    for event in context.poll_events() {
      match event {
        Event::Closed | Event::KeyboardInput(_, _, Some(VirtualKeyCode::Escape)) => {
          println!("Exiting...");
          return;
        },
        Event::KeyboardInput(element_state, _, Some(key_code)) => {
          match key_code {
            VirtualKeyCode::Up | VirtualKeyCode::W => {
              match element_state {
                ElementState::Pressed => fps_camera.forward = true,
                ElementState::Released => fps_camera.forward = false,
              };
            },
            VirtualKeyCode::Down | VirtualKeyCode::S => {
              match element_state {
                ElementState::Pressed => fps_camera.backward = true,
                ElementState::Released => fps_camera.backward = false,
              };
            },
            VirtualKeyCode::Left | VirtualKeyCode::A => {
              match element_state {
                ElementState::Pressed => fps_camera.left = true,
                ElementState::Released => fps_camera.left = false,
              };
            },
            VirtualKeyCode::Right | VirtualKeyCode::D => {
              match element_state {
                ElementState::Pressed => fps_camera.right = true,
                ElementState::Released => fps_camera.right = false,
              };
            },
            _ => {},
          }
        },
        Event::MouseMoved(x, y) => {
          if !vr_mode {
            let (width, height) = window.get_inner_size_pixels().unwrap();
            let origin_x = width as i32 / 2;
            let origin_y = height as i32 / 2;
            let rel_x = x - origin_x;
            let rel_y = y - origin_y;
            fps_camera.pitch = Rad((fps_camera.pitch - Rad(rel_y as f32 / 1000.0)).0
              .max(-f32::consts::PI / 2.0)
              .min(f32::consts::PI / 2.0));
            fps_camera.yaw -= Rad(rel_x as f32 / 1000.0);
            window.set_cursor_position(origin_x, origin_y).unwrap();
          }
        },
        Event::Resized(width, height) => {
          render_dimensions = (width, height);
          println!("resized to {}x{}", width, height);
        }
        _ => {}
      };
    }
  }
}
