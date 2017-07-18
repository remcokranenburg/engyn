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
#[macro_use] extern crate conrod;
extern crate rand;
extern crate rust_webvr as webvr;
extern crate tobj;

mod adaptive_canvas;
mod camera;
mod geometry;
mod gui;
mod light;
mod material;
mod math;
mod mesh;
mod object;
mod performance;
mod teapot;
mod uniforms;

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
use glium::Program;
use glium::Surface;
use glium::backend::glutin_backend::GlutinFacade;
use glium::glutin::Event;
use glium::glutin::MouseCursor;
use glium::glutin::CursorState;
use glium::glutin::ElementState;
use glium::glutin::VirtualKeyCode;
use glium::glutin::WindowBuilder;
use glium::index::IndexBuffer;
use glium::index::PrimitiveType;
use glium::texture::RawImage2d;
use glium::texture::SrgbTexture2d;
use glium::uniforms::MagnifySamplerFilter;
use glium::vertex::VertexBuffer;
use std::path::Path;
use std::f32;
use webvr::VREvent;
use webvr::VRDisplayEvent;
use webvr::VRServiceManager;

use adaptive_canvas::AdaptiveCanvas;
use camera::FpsCamera;
use light::Light;
use geometry::Geometry;
use geometry::Texcoord;
use gui::Gui;
use material::Material;
use mesh::Mesh;
use object::Object;
use performance::FramePerformance;

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
        .with_title("Engyn")
        .with_dimensions(window_width, window_height)
        .build_glium()
        .unwrap();

      (true, context, (render_width, render_height))
    },
    None => {
      println!("No VR device detected. Drawing on normal monitor.");
      let context = WindowBuilder::new()
        .with_title("Engyn")
        .with_vsync()
        .with_fullscreen(glium::glutin::get_primary_monitor())
        .build_glium()
        .unwrap();

      let (width, height) = {
        let window = context.get_window().unwrap();
        let (window_width, window_height) = window.get_inner_size_pixels().unwrap();
        let render_width = (window_width as f32 * 0.5) as u32;
        let render_height = window_height;
        let origin_x = window_width as i32 / 2;
        let origin_y = window_height as i32 / 2;
        window.set_cursor_position(origin_x, origin_y).unwrap();
        window.set_cursor(MouseCursor::NoneCursor);
        window.set_cursor_state(CursorState::Grab).ok().expect("Could not grab mouse cursor");
        (render_width, render_height)
      };

      (false, context, (width, height))
    },
  };

  println!("Loading textures...");
  let marble_tex = load_texture(&context, "data/marble.jpg");
  let terrain_tex = load_texture(&context, "data/terrain.png");
  println!("Textures loaded!");

  let mut canvas = AdaptiveCanvas::new(
      &context,
      render_dimensions.0 * 4,
      render_dimensions.1 * 2);

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
        out vec3 v_vertex_position;

        void main() {
          mat4 normal_matrix = transpose(inverse(model)); // TODO: put this in host code
          vec4 position_global = model * vec4(position, 1.0);
          vec4 position_eye = view * position_global;

          v_texcoord = texcoord;
          v_normal = vec3(normal_matrix * vec4(normal, 1.0));
          v_vertex_position = vec3(position_global);
          gl_Position = projection * position_eye;
        }
      "#,
      &str::replace(r#"
        #version 330
        layout(std140) uniform;

        const float SCREEN_GAMMA = 2.2;
        const float INTENSITY = 20.0;

        struct Light {
          vec3 color;
          vec3 position;
        };

        uniform sampler2D albedo_map;
        uniform int num_lights;
        uniform Light lights[MAX_NUM_LIGHTS];

        in vec3 v_normal;
        in vec2 v_texcoord;
        in vec3 v_vertex_position;

        out vec4 color;

        vec3 attenuate(vec3 color, vec3 light_position, float radius, vec3 surface_position) {
          float dist = distance(light_position, surface_position);
          float attenuation_factor = clamp(1.0 - dist * dist / (radius * radius), 0.0, 1.0);
          attenuation_factor *= attenuation_factor;
          return color * attenuation_factor;
        }

        vec3 calculate_lighting(
            vec3 light_position,
            vec3 normal,
            vec3 combined_color) {
          vec3 light_direction = normalize(light_position - v_vertex_position);
          float lambertian = max(dot(light_direction, normal), 0.0);
          return lambertian * combined_color;
        }

        void main() {
          vec3 normal = normalize(v_normal);
          vec3 material_color = vec3(texture(albedo_map, v_texcoord));
          vec3 color_linear = vec3(0.0);

          for(int i = 0; i < num_lights; i++) {
            vec3 color_one_light = calculate_lighting(lights[i].position, normal, lights[i].color * material_color);
            color_one_light = attenuate(color_one_light, lights[i].position, INTENSITY, v_vertex_position);
            color_linear += color_one_light;
          }

          vec3 color_gamma_corrected = pow(color_linear, vec3(1.0 / SCREEN_GAMMA)); // assumes textures are linearized (i.e. not sRGB))
          color = vec4(color_gamma_corrected, 1.0);
        }
      "#, "MAX_NUM_LIGHTS", &format!("{}", uniforms::MAX_NUM_LIGHTS)),
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

  // add a light

  let num_lights = 4;
  let mut lights: [Light; uniforms::MAX_NUM_LIGHTS] = Default::default();
  lights[0] = Light { color: [1.0, 0.0, 0.0], position: [10.0, 10.0, 10.0] };
  lights[1] = Light { color: [0.0, 1.0, 0.0], position: [10.0, 10.0, -10.0] };
  lights[2] = Light { color: [0.0, 0.0, 1.0], position: [-10.0, 10.0, -10.0] };
  lights[2] = Light { color: [1.0, 1.0, 1.0], position: [-10.0, 10.0, 10.0] };

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

  for _ in &gamepads {
    println!("We've found a gamepad!");
    gamepad_models.push(Object {
      mesh: Some(Mesh {
        geometry: Geometry::from_obj(&context, "data/vive-controller.obj"),
        material: Material { albedo_map: &marble_tex, metalness: 0.0, reflectivity: 0.0 },
      }),
      transform: Matrix4::<f32>::identity(),
    });
  }

  let mut gui = Gui::new(&context, render_dimensions.0 as f64, render_dimensions.1 as f64);

  let mut frame_performance = FramePerformance::new();

  loop {
    frame_performance.process_frame_start();

    let aspect_ratio = render_dimensions.0 as f32 / render_dimensions.1 as f32;
    let mono_projection = cgmath::perspective(Deg(45.0), aspect_ratio, 0.01f32, 1000.0);

    let (
        standing_transform,
        left_projection_matrix,
        right_projection_matrix,
        left_view_matrix,
        right_view_matrix) = if vr_mode {
      let display = displays.get(0).unwrap();
      display.borrow_mut().sync_poses();
      let display_data = display.borrow().data();

      let standing_transform = if let Some(ref stage) = display_data.stage_parameters {
        math::vec_to_matrix(&stage.sitting_to_standing_transform).inverse_transform().unwrap()
      } else {
        // Stage parameters not available yet or unsupported
        // Assume 0.75m transform height
        math::vec_to_translation(&[0.0, 0.75, 0.0]).inverse_transform().unwrap()
      };

      let frame_data = display.borrow().synced_frame_data(0.1, 1000.0);

      let left_projection_matrix = math::vec_to_matrix(&frame_data.left_projection_matrix);
      let right_projection_matrix = math::vec_to_matrix(&frame_data.right_projection_matrix);
      let left_view_matrix = math::vec_to_matrix(&frame_data.left_view_matrix);
      let right_view_matrix = math::vec_to_matrix(&frame_data.right_view_matrix);

      (standing_transform, left_projection_matrix, right_projection_matrix, left_view_matrix,
          right_view_matrix)
    } else {
      let standing_transform = Matrix4::<f32>::identity();
      let view = fps_camera.get_view(0.016); // TODO: get actual timedelta

      (standing_transform, mono_projection, mono_projection, view, view)
    };

    let inverse_standing_transform = standing_transform.inverse_transform().unwrap();

    {
      let eyes = [
        (&canvas.viewports[0], &left_projection_matrix, &left_view_matrix),
        (&canvas.viewports[1], &right_projection_matrix, &right_view_matrix),
      ];

      let mut framebuffer = canvas.get_framebuffer(&context).unwrap();
      framebuffer.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

      for eye in &eyes {
        let projection = math::matrix_to_uniform(*eye.1);
        let view = math::matrix_to_uniform(eye.2 * standing_transform);
        let viewport = *eye.0;

        render_params.viewport = Some(viewport);

        for object in &mut world {
          object.draw(&mut framebuffer, projection, view, &render_program, &render_params, num_lights, lights);
        }

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
          gamepad_models[i].draw(&mut framebuffer, projection, view, &render_program, &render_params, num_lights, lights);
        }

        gui.draw(&mut framebuffer, viewport);
      }

      if vr_mode {
        let display = displays.get(0).unwrap();
        display.borrow_mut().submit_frame(&canvas.layer);
      }

      // now draw the canvas as a texture to the window

      let mut target = context.draw();

      let src_rect = glium::Rect {
        left: 0,
        bottom: 0,
        width: canvas.viewports[0].width * 2,
        height: canvas.viewports[0].height,
      };

      let (width, height) = window.get_inner_size_pixels().unwrap();

      let blit_target = glium::BlitTarget {
        left: 0,
        bottom: 0,
        width: width as i32,
        height: height as i32,
      };

      framebuffer.blit_color(&src_rect, &target, &blit_target, MagnifySamplerFilter::Linear);

      target.finish().unwrap();
    }

    // once every 100 frames, check for VR events
    event_counter += 1;
    if event_counter % 100 == 0 {
      for event in vr.poll_events() {
        match event {
          VREvent::Display(VRDisplayEvent::Connect(data)) => {
            println!("VR display {}: Connected (name: {})", data.display_id, data.display_name);
          },
          VREvent::Display(VRDisplayEvent::Disconnect(display_id)) => {
            println!("VR display {}: Disconnected.", display_id);
          },
          VREvent::Display(VRDisplayEvent::Activate(data, _)) => {
            println!("VR display {}: Activated.", data.display_id);
          },
          VREvent::Display(VRDisplayEvent::Deactivate(data, _)) => {
            println!("VR display {}: Deactivated.", data.display_id);
          },
          _ => println!("VR event: {:?}", event),
        }
      }
    }

    assert_no_gl_error!(context);

    for event in context.poll_events() {
      if let Some(event) = conrod::backend::winit::convert(event.clone(), &context) {
        gui.handle_event(event);
      }

      match event {
        Event::Closed | Event::KeyboardInput(_, _, Some(VirtualKeyCode::Q)) => {
          println!("Exiting...");
          return;
        },
        Event::KeyboardInput(element_state, _, Some(key_code)) => {
          match key_code {
            VirtualKeyCode::Escape => {
              if element_state == ElementState::Pressed {
                gui.is_visible = !gui.is_visible;
              }
            },
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
            VirtualKeyCode::Equals => {
              if element_state == ElementState::Pressed {
                frame_performance.reduce_fps();
              };
            },
            VirtualKeyCode::Minus => {
              if element_state == ElementState::Pressed {
                frame_performance.increase_fps();
              }
            },
            VirtualKeyCode::PageDown => {
              if element_state == ElementState::Pressed {
                canvas.decrease_resolution();
              }
            },
            VirtualKeyCode::PageUp => {
              if element_state == ElementState::Pressed {
                canvas.increase_resolution();
              }
            }
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
          render_dimensions = ((width as f64 * 0.5) as u32, height);
          println!("resized to {}x{}", width, height);
        }
        _ => {}
      };
    }

    frame_performance.process_frame_end();
  }
}
