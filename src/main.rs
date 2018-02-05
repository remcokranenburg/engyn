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

extern crate argparse;
extern crate bincode;
extern crate cgmath;
extern crate chrono;
#[macro_use] extern crate conrod;
#[macro_use] extern crate glium;
extern crate image;
extern crate rand;
extern crate rust_webvr as webvr;
#[macro_use] extern crate serde_derive;
extern crate tobj;

mod adaptive_canvas;
mod camera;
mod conic;
mod demo;
mod geometry;
mod gui;
mod input;
mod light;
mod material;
mod math;
mod mesh;
mod network_graph;
mod object;
mod performance;
mod quality;
mod resources;
mod teapot;
mod uniforms;

use argparse::ArgumentParser;
use argparse::List;
use argparse::Print;
use argparse::Store;
use argparse::StoreFalse;
use argparse::StoreTrue;
use cgmath::Deg;
use cgmath::Matrix4;
use cgmath::Quaternion;
use cgmath::Rad;
use cgmath::SquareMatrix;
use cgmath::Transform;
use cgmath::Vector3;
use chrono::Utc;
use glium::BlitTarget;
use glium::Depth;
use glium::DepthTest;
use glium::Display;
use glium::DrawParameters;
use glium::Rect;
use glium::Surface;
use glium::glutin::Event;
use glium::glutin::EventsLoop;
use glium::glutin::WindowEvent;
use glium::glutin::KeyboardInput;
use glium::glutin::MouseCursor;
use glium::glutin::ContextBuilder;
use glium::glutin::CursorState;
use glium::glutin::ElementState;
use glium::glutin::VirtualKeyCode;
use glium::glutin::Window;
use glium::glutin::WindowBuilder;
use glium::index::IndexBuffer;
use glium::index::PrimitiveType;
use glium::uniforms::MagnifySamplerFilter;
use glium::vertex::VertexBuffer;
use std::cell::RefCell;
use std::env;
use std::path::Path;
use std::f32;
use std::fs::File;
use std::io::prelude::*;
use std::rc::Rc;
use webvr::VREvent;
use webvr::VRDisplayPtr;
use webvr::VRDisplayEvent;
use webvr::VRGamepadPtr;
use webvr::VRServiceManager;

use adaptive_canvas::AdaptiveCanvas;
use camera::FpsCamera;
use conic::Conic;
use demo::Demo;
use demo::DemoEntry;
use light::Light;
use geometry::Geometry;
use geometry::Texcoord;
use gui::Action;
use gui::Gui;
use input::InputHandler;
use material::Material;
use mesh::Mesh;
use network_graph::Network;
use object::Object;
use performance::FramePerformance;
use quality::Quality;
use resources::ResourceManager;

fn calculate_num_objects(objects: &Vec<Object>) -> u32 {
  objects.iter().fold(0, |acc, o| acc + 1 + calculate_num_objects(&o.children))
}

fn update(display: &Display, world: &mut Vec<Object>, gui: &mut Gui, action: &Action) {
  let action = gui.process_action(action);

  for object in world {
    match object.drawable {
      Some(ref mut drawable) => drawable.update(display, object.transform, &action),
      None => (),
    };

    update(display, &mut object.children, gui, &action);
  }
}

fn draw_frame(
    benchmarking: bool,
    feature: &str,
    quality: &Quality,
    perf_filename: &str,
    demo_filename: &str,
    vr: &mut VRServiceManager,
    vr_mode: bool,
    vr_display: Option<&VRDisplayPtr>,
    display: &Display,
    window: &Window,
    render_params: &mut DrawParameters,
    world: &mut Vec<Object>,
    num_objects: u32,
    lights: &[Light; uniforms::MAX_NUM_LIGHTS],
    num_lights: i32,
    empty: &mut Object,
    gamepads: &Vec<VRGamepadPtr>,
    gamepad_models: &mut Vec<Object>,
    event_counter: &mut u64,
    events_loop: &mut EventsLoop,
    canvas: &mut AdaptiveCanvas,
    frame_performance: &mut FramePerformance,
    render_dimensions: &mut (u32, u32),
    fps_camera: &mut FpsCamera,
    gui: &mut Gui,
    demo: &mut Option<Demo>,
    demo_record: bool) -> bool {
  frame_performance.process_frame_start(feature, *quality.level.borrow());

  let aspect_ratio = render_dimensions.0 as f32 / render_dimensions.1 as f32;
  let mono_projection = cgmath::perspective(Deg(45.0), aspect_ratio, 0.01f32, 1000.0);
  let mut action;

  let (
      standing_transform,
      left_projection_matrix,
      right_projection_matrix,
      mut left_view_matrix,
      mut right_view_matrix) = if vr_mode {
    vr_display.unwrap().borrow_mut().sync_poses();
    frame_performance.process_sync_poses();

    let display_data = vr_display.unwrap().borrow().data();

    let standing_transform = if let Some(ref stage) = display_data.stage_parameters {
      math::vec_to_matrix(&stage.sitting_to_standing_transform).inverse_transform().unwrap()
    } else {
      // Stage parameters not available yet or unsupported
      // Assume 0.75m transform height
      math::vec_to_translation(&[0.0, 0.75, 0.0]).inverse_transform().unwrap()
    };

    let frame_data = vr_display.unwrap().borrow().synced_frame_data(0.1, 1000.0);

    frame_performance.process_sync_frame_data();

    let left_projection_matrix = math::vec_to_matrix(&frame_data.left_projection_matrix);
    let right_projection_matrix = math::vec_to_matrix(&frame_data.right_projection_matrix);
    let left_view_matrix = math::vec_to_matrix(&frame_data.left_view_matrix);
    let right_view_matrix = math::vec_to_matrix(&frame_data.right_view_matrix);

    (standing_transform, left_projection_matrix, right_projection_matrix, left_view_matrix,
        right_view_matrix)
  } else {
    frame_performance.process_sync_poses();

    let standing_transform = Matrix4::<f32>::identity();
    let view = fps_camera.get_view(0.016); // TODO: get actual timedelta

    frame_performance.process_sync_frame_data();

    let left_translation = Matrix4::from_translation(Vector3::new(-0.05, 0.0, 0.0));
    let left_view = left_translation * view;
    let right_translation = Matrix4::from_translation(Vector3::new(0.05, 0.0, 0.0));
    let right_view = right_translation * view;
    (standing_transform, mono_projection, mono_projection, left_view, right_view)
  };

  let inverse_standing_transform = standing_transform.inverse_transform().unwrap();

  action = gui.prepare(*quality.level.borrow());

  frame_performance.process_draw_start();

  // record demo entry
  if let Some(ref mut d) = *demo {
    let frame_number = frame_performance.get_frame_number() as usize;

    if demo_record {
      d.entries.push(DemoEntry {
        head_left: left_view_matrix.clone().into(),
        head_right: right_view_matrix.clone().into(),
      });
    } else if frame_number < d.entries.len() {
      left_view_matrix = d.entries[frame_number].head_left.into();
      right_view_matrix = d.entries[frame_number].head_right.into();
    }
  }

  {
    let eyes = [
      (&canvas.viewports[0], &left_projection_matrix, &left_view_matrix),
      (&canvas.viewports[1], &right_projection_matrix, &right_view_matrix),
    ];

    let mut framebuffer = canvas.get_framebuffer(display).unwrap();
    framebuffer.clear_color_and_depth((0.4, 0.4, 0.4, 1.0), 1.0);

    for eye in &eyes {
      let projection = math::matrix_to_uniform(*eye.1);
      let view = math::matrix_to_uniform(eye.2 * standing_transform);
      let viewport = *eye.0;

      render_params.viewport = Some(viewport);

      let mut i = 0;
      let quality_level = quality.get_target_lod();
      for object in world.iter_mut() {
        if quality_level > (i as f32 / num_objects as f32) {
          i = object.draw(quality_level, i, num_objects, &mut framebuffer, projection, view, &render_params, num_lights, lights);
        }
      }

      empty.draw(1.0, 0, 1, &mut framebuffer, projection, view, &render_params, num_lights, lights);

      for (i, ref gamepad) in gamepads.iter().enumerate() {
        let state = gamepad.borrow().state();
        let rotation = match state.pose.orientation {
          Some(o) => Matrix4::from(Quaternion::new(o[3], o[0], o[1], o[2])), // WebVR presents quaternions as (x, y, z, w)
          None => Matrix4::<f32>::identity(),
        };
        let position = match state.pose.position {
          Some(position) => Matrix4::from_translation(Vector3::from(position)),
          None => Matrix4::<f32>::identity(),
        };

        gamepad_models[i].transform = inverse_standing_transform * position * rotation;
        gamepad_models[i].draw(1.0, 0, 1, &mut framebuffer, projection, view, &render_params, num_lights, lights);
      }

      canvas.resolve(display);

      for eye in &eyes {
        gui.draw(&mut canvas.get_resolved_framebuffer(display).unwrap(), *eye.0);
      }
    }

    if vr_mode {
      vr_display.unwrap().borrow_mut().render_layer(canvas.get_resolved_layer());
      vr_display.unwrap().borrow_mut().submit_frame();
    }

    // now draw the canvas as a texture to the window

    let target = display.draw();

    let src_rect = Rect {
      left: 0,
      bottom: 0,
      width: canvas.viewports[0].width * 2,
      height: canvas.viewports[0].height,
    };

    let (width, height) = window.get_inner_size().unwrap();

    let blit_target = BlitTarget {
      left: 0,
      bottom: 0,
      width: width as i32,
      height: height as i32,
    };

    canvas.get_resolved_framebuffer(display).unwrap()
      .blit_color(&src_rect, &target, &blit_target, MagnifySamplerFilter::Linear);

    target.finish().unwrap();
  }

  let predicted_remaining_time = frame_performance.process_draw_end();

  // once every 100 frames, check for VR events
  *event_counter += 1;
  if *event_counter % 100 == 0 {
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
        _ => (),
      }
    }
  }

  assert_no_gl_error!(*display);

  let mut is_done = false;

  events_loop.poll_events(|event| {
    if let Some(event) = conrod::backend::winit::convert_event(event.clone(), display) {
      gui.handle_event(event);
    }

    match event {
      Event::WindowEvent { event, .. } => match event {
        WindowEvent::Closed => is_done = true,
        WindowEvent::Resized(width, height) => {
          *render_dimensions = (width / 2, height);
          println!("resized to {}x{}", width, height);
        },
        WindowEvent::KeyboardInput { input, .. } => {
          let key_is_pressed = input.state == ElementState::Pressed;

          match input {
            KeyboardInput { virtual_keycode, .. } => match virtual_keycode {
              Some(VirtualKeyCode::Q)         => if gui.is_visible { is_done = true },
              Some(VirtualKeyCode::Escape)    => if key_is_pressed {
                gui.is_visible = !gui.is_visible;

                if gui.is_visible {
                  window.set_cursor(MouseCursor::Default);
                  window.set_cursor_state(CursorState::Normal)
                      .ok()
                      .expect("Could not ungrab mouse cursor");
                } else {
                  window.set_cursor(MouseCursor::NoneCursor);
                  window.set_cursor_state(CursorState::Grab)
                      .ok()
                      .expect("Could not grab mouse cursor");
                }
              },
              // Some(VirtualKeyCode::Up)        => if key_is_pressed { gui.select_previous() },
              // Some(VirtualKeyCode::Down)      => if key_is_pressed { gui.select_next() },
              // Some(VirtualKeyCode::Left)      => if key_is_pressed { gui.decrease_slider() },
              // Some(VirtualKeyCode::Right)     => if key_is_pressed { gui.increase_slider() },
              // Some(VirtualKeyCode::H)         => if key_is_pressed { conic.decrease_eccentricity() },
              // Some(VirtualKeyCode::J)         => if key_is_pressed { conic.increase_eccentricity() },
              // Some(VirtualKeyCode::K)         => if key_is_pressed { conic.decrease_slr() },
              // Some(VirtualKeyCode::L)         => if key_is_pressed { conic.increase_slr() },
              // Some(VirtualKeyCode::Return)    => if key_is_pressed {
              //   let tmp_action = gui.activate();
              //
              //   match tmp_action {
              //     Action::None => (),
              //     _ => action = tmp_action,
              //   }
              // },

              // activate while key is pressed
              Some(VirtualKeyCode::W) => fps_camera.forward = key_is_pressed,
              Some(VirtualKeyCode::S) => fps_camera.backward = key_is_pressed,
              Some(VirtualKeyCode::A) => fps_camera.left = key_is_pressed,
              Some(VirtualKeyCode::D) => fps_camera.right = key_is_pressed,
              _ => {},
            },
          }
        },
        WindowEvent::CursorMoved { position, .. } => {
          if !vr_mode && !gui.is_visible {
            let (width, height) = window.get_inner_size().unwrap();
            let origin_x = width as f64 / 4.0;
            let origin_y = height as f64 / 4.0;
            let rel_x = position.0 - origin_x;
            let rel_y = position.1 - origin_y;

            if frame_performance.get_frame_number() > 1 {
              fps_camera.pitch = Rad((fps_camera.pitch - Rad(rel_y as f32 / 1000.0)).0
                .max(-f32::consts::PI / 2.0)
                .min(f32::consts::PI / 2.0));
              fps_camera.yaw -= Rad(rel_x as f32 / 1000.0);
            }

            window.set_cursor_position(origin_x as i32, origin_y as i32).unwrap();
          }
        },
        _ => (),
      },
      _ => (),
    };
  });

  match action {
    Action::Quit => return false,
    Action::Resume => {
      gui.is_visible = false;

      if !vr_mode {
        let (width, height) = window.get_inner_size().unwrap();
        let origin_x = (width / 2) as i32;
        let origin_y = (height / 2) as i32;
        window.set_cursor_position(origin_x, origin_y).unwrap();
        window.set_cursor(MouseCursor::NoneCursor);
        window.set_cursor_state(CursorState::Grab).ok().expect("Could not grab mouse cursor");
      }
    },
    _ => (),
  }

  if !benchmarking {
    quality.set_level(predicted_remaining_time, frame_performance.get_target_frame_time());
    canvas.set_resolution_scale(quality.get_target_resolution());
    canvas.set_msaa_scale(quality.get_target_msaa());
  }

  frame_performance.process_frame_end();

  // quit when demo is done
  if let Some(d) = demo.as_mut() {
    if !demo_record && frame_performance.get_frame_number() as usize >= d.entries.len() {
      is_done = true;
    }
  }

  if is_done {
    let now = Utc::now().format("%Y-%m-%d-%H-%M-%S");

    if perf_filename != "" && !benchmarking {
      let csv = frame_performance.to_csv();
      let mut file = File::create(format!("{}-{}.csv", perf_filename, now)).unwrap();
      file.write_all(csv.as_bytes()).unwrap();
    }

    if demo_record {
      if let Some(d) = demo.as_mut() {
        let filename = if demo_filename != "" {
          demo_filename.to_string()
        } else {
          format!("performance/{}.demo", now)
        };
        d.to_bincode(&filename).unwrap();
      }
    }

    return false;
  }

  true
}

fn main() {
  let mut obj_filename = "".to_string();
  let mut perf_filename = "".to_string();
  let mut demo_filename = "".to_string();
  let mut demo_record = false;
  let mut weights = Vec::<f32>::new();
  let mut enable_supersampling = true;

  {
    let mut ap = ArgumentParser::new();
    ap.set_description("Engyn: a configurable adaptive quality graphics engine.");
    ap.add_option(&["-V", "--version"],
        Print(env!("CARGO_PKG_VERSION").to_string()), "show version");
    ap.refer(&mut obj_filename)
      .add_option(&["-o", "--open"], Store, "open .obj file");
    ap.refer(&mut perf_filename)
      .add_option(&["-p", "--perf"], Store, "performance measurements");
    ap.refer(&mut demo_filename)
      .add_option(&["-d", "--demo-filename"], Store, "file to use for playing demos (or record)");
    ap.refer(&mut demo_record)
      .add_option(&["-r", "--record"], StoreTrue, "set this to record demo instead of playback");
    ap.refer(&mut weights)
      .add_option(&["--weights"], List, "quality weights");
    ap.refer(&mut enable_supersampling)
      .add_option(&["-s", "--no-supersampling"], StoreFalse, "limit maximum resolution to monitor \
          resolution");

    ap.parse_args_or_exit();
  }

  let mut demo = if demo_record {
    Some(Demo::new())
  } else if demo_filename != "" {
    Some(Demo::from_bincode(&demo_filename).unwrap())
  } else {
    None
  };

  let benchmarking = perf_filename != "" && demo.is_some();

  let mut vr = VRServiceManager::new();
  vr.register_defaults();
  vr.initialize_services();

  let vr_displays = vr.get_displays();
  let vr_display = vr_displays.get(0);
  let vr_mode = vr_display.is_some();

  let mut events_loop = EventsLoop::new();
  let window_builder = WindowBuilder::new()
    .with_title("Engyn")
    .with_fullscreen(Some(events_loop.get_primary_monitor()));

  let context_builder = ContextBuilder::new()
    .with_vsync(!vr_mode);

  let display = Display::new(window_builder, context_builder, &events_loop).unwrap();

  let window = display.gl_window();

  let mut render_dimensions = match vr_display {
    Some(d) => {
      let params = d.borrow().data().left_eye_parameters;
      (params.render_width, params.render_height)
    },
    None => {
      let dimensions = window.get_inner_size().unwrap();
      (dimensions.0 / 2, dimensions.1)
    },
  };

  let resource_manager = ResourceManager::new(&display);

  if !vr_mode {
    let (width, height) = window.get_inner_size().unwrap();
    let origin_x = width / 4;
    let origin_y = height / 4;
    window.set_cursor_position(origin_x as i32, origin_y as i32).unwrap();
    window.set_cursor(MouseCursor::NoneCursor);
    window.set_cursor_state(CursorState::Grab).ok().expect("Could not grab mouse cursor");
  }

  let executable_string = env::args().nth(0).unwrap();
  let executable_path = Path::new(&executable_string).parent().unwrap();
  let project_path = executable_path.parent().unwrap().parent().unwrap();

  println!("Executable path: {}", executable_path.to_str().unwrap());
  println!("Executable path: {}", project_path.to_str().unwrap());

  println!("Loading materials...");
  let empty_path = project_path.join("data").join("empty.bmp");
  let empty_path = resource_manager.get_texture(empty_path.to_str().unwrap()).unwrap();
  let empty_material = Rc::new(RefCell::new(Material {
    albedo_map: Rc::clone(&empty_path),
    metalness: 0.0,
    reflectivity: 0.0,
  }));
  let marble_path = project_path.join("data").join("marble.jpg");
  let marble_texture = resource_manager.get_texture(marble_path.to_str().unwrap()).unwrap();
  let marble_material = Rc::new(RefCell::new(Material {
    albedo_map: Rc::clone(&marble_texture),
    metalness: 0.0,
    reflectivity: 0.0,
  }));
  let terrain_path = project_path.join("data").join("terrain.png");
  let terrain_texture = resource_manager.get_texture(terrain_path.to_str().unwrap()).unwrap();
  let terrain_material = Rc::new(RefCell::new(Material {
    albedo_map: Rc::clone(&terrain_texture),
    metalness: 0.0,
    reflectivity: 0.0,
  }));
  println!("Materials loaded!");

  let canvas_dimensions = if enable_supersampling {
    (render_dimensions.0 * 4, render_dimensions.1 * 2)
  } else {
    (render_dimensions.0 * 2, render_dimensions.1)
  };

  let mut canvas = AdaptiveCanvas::new(&display, canvas_dimensions.0, canvas_dimensions.1, 4);

  let mut world = Vec::new();

  if obj_filename != "" {
    world.push(Object::from_file(&display, &resource_manager, &obj_filename));
  } else {
    // a triangle
    world.push(Object::new_triangle(&display, &resource_manager, Rc::clone(&marble_material),
        [1.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]));

    // a terrain mesh
    world.push(Object {
      children: Vec::new(),
      drawable: Some(Box::new(Mesh::new(
          &display,
          Geometry::from_obj(&display, "data/terrain.obj"),
          Rc::clone(&terrain_material),
          &resource_manager))),
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
      children: Vec::new(),
      drawable: Some(Box::new(Mesh::new(
          &display,
          Geometry {
            indices: Some(IndexBuffer::new(
                &display,
                PrimitiveType::TrianglesList,
                &teapot::INDICES).unwrap()),
            normals: VertexBuffer::new(&display, &teapot::NORMALS).unwrap(),
            vertices: VertexBuffer::new(&display, &teapot::VERTICES).unwrap(),
            texcoords: VertexBuffer::new(&display, &my_teapot_texcoords).unwrap(),
          },
          Rc::clone(&marble_material),
          &resource_manager))),
      transform: Matrix4::new(
          0.005, 0.0, 0.0, 0.0,
          0.0, 0.005, 0.0, 0.0,
          0.0, 0.0, 0.005, 0.0,
          0.0, 1.0, 0.0, 1.0),
    };

    world.push(my_teapot);

    let my_conic = Object {
        children: Vec::new(),
        drawable: Some(Box::new(Conic::new(&display))),
        transform: Matrix4::new(
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 1.0, -1.0, 1.0),
    };

    world.push(my_conic);

    let my_network = Object {
        children: Vec::new(),
        drawable: Some(Box::new(Network::new(&display, 200, 10))),
        transform: Matrix4::new(
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 1.0, 1.0, 1.0),
    };

    world.push(my_network);
  }

  let num_objects = calculate_num_objects(&world);

  // empty texture to force glutin clean
  let mut empty = Object::new_plane(&display, &resource_manager, Rc::clone(&empty_material),
      [0.0001,0.0001], [-0.1, 0.1, 0.0], [0.0, 0.0, 0.0], [-1.0,1.0,1.0]);

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
  fps_camera.pitch = Rad(-f32::consts::PI / 8.0);

  // create a model for each gamepad
  let gamepads = vr.get_gamepads();
  let mut gamepad_models = Vec::new();

  println!("Found {} controller{}!", gamepads.len(), match gamepads.len() { 1 => "", _ => "s" });

  for _ in &gamepads {
    println!("We've found a gamepad!");
    gamepad_models.push(Object {
      children: Vec::new(),
      drawable: Some(Box::new(Mesh::new(
          &display,
          Geometry::from_obj(&display, "data/vive-controller.obj"),
          Rc::clone(&marble_material),
          &resource_manager))),
      transform: Matrix4::<f32>::identity(),
    });
  }

  let mut input_handler = InputHandler::new(gamepads.len());
  let mut quality = Quality::new(weights, enable_supersampling);
  let mut gui = Gui::new(&display, Rc::clone(&quality.weight_resolution),
      Rc::clone(&quality.weight_msaa), Rc::clone(&quality.weight_lod));
  let mut frame_performance = FramePerformance::new(vr_mode);

  if benchmarking {
    for feature in &["resolution", "msaa", "lod"] {
      for quality_integer in 0u32 .. 50u32 {
        quality.set_benchmark_mode(*feature, quality_integer as f32 * 0.02);
        frame_performance.reset_frame_count();

        loop {
          canvas.set_resolution_scale(quality.get_target_resolution());
          canvas.set_msaa_scale(quality.get_target_msaa());

          let action = input_handler.process(&gamepads);
          // TODO: update_camera()
          // TODO: maybe merge benchmark and non-benchmark loops again

          update(&display, &mut world, &mut gui, &action);

          if !draw_frame(benchmarking, feature, &quality, &perf_filename, &demo_filename, &mut vr,
              vr_mode, vr_display, &display, &window, &mut render_params, &mut world, num_objects,
              &lights, num_lights, &mut empty, &gamepads, &mut gamepad_models,&mut event_counter,
              &mut events_loop, &mut canvas, &mut frame_performance, &mut render_dimensions,
              &mut fps_camera, &mut gui, &mut demo, demo_record) {
            break;
          }
        }
      }
    }

    // write benchmark csv
    let now = Utc::now().format("%Y-%m-%d-%H-%M-%S");
    let csv = frame_performance.to_csv();
    let mut file = File::create(format!("{}-{}.csv", perf_filename, now)).unwrap();
    file.write_all(csv.as_bytes()).unwrap();

  } else {
    loop {
      let action = input_handler.process(&gamepads);
      // TODO: update_camera()
      // TODO: maybe merge benchmark and non-benchmark loops again

      update(&display, &mut world, &mut gui, &action);

      if !draw_frame(benchmarking, "", &quality, &perf_filename, &demo_filename, &mut vr,
          vr_mode, vr_display, &display, &window, &mut render_params, &mut world, num_objects,
          &lights, num_lights, &mut empty, &gamepads, &mut gamepad_models, &mut event_counter,
          &mut events_loop, &mut canvas, &mut frame_performance, &mut render_dimensions,
          &mut fps_camera, &mut gui, &mut demo, demo_record) {
        break;
      }
    }
  }
}
