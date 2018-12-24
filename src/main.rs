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

              extern crate argparse;
              extern crate bincode;
              extern crate cgmath;
              extern crate chrono;
#[macro_use]  extern crate conrod;
              extern crate csv;
#[macro_use]  extern crate glium;
              extern crate image;
              extern crate itertools;
              extern crate rand;
              extern crate rust_webvr as webvr;
#[macro_use]  extern crate serde_derive;
              extern crate serde_yaml;
              extern crate tobj;

mod adaptive_canvas;
mod benchmark;
mod camera;
mod conic;
mod demo;
mod drawable;
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
mod scene;
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
use glium::glutin::EventsLoop;
use glium::glutin::MouseCursor;
use glium::glutin::ContextBuilder;
use glium::glutin::CursorState;
use glium::glutin::Window;
use glium::glutin::WindowBuilder;
use glium::index::IndexBuffer;
use glium::index::PrimitiveType;
use glium::uniforms::MagnifySamplerFilter;
use glium::vertex::VertexBuffer;
use itertools::Itertools;
use rand::prng::hc128::Hc128Rng;
use rand::SeedableRng;
use rand::Rng;
use std::cell::RefCell;
use std::f32;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::rc::Rc;
use webvr::VRDisplayPtr;
use webvr::VRFramebufferAttributes;
use webvr::VRGamepadPtr;
use webvr::VRServiceManager;

use adaptive_canvas::AdaptiveCanvas;
use benchmark::Benchmark;
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
use scene::Scene;

fn calculate_num_objects(objects: &Vec<Object>) -> u32 {
  objects.iter().fold(0, |acc, o| acc + 1 + calculate_num_objects(&o.children))
}

fn update_camera(fps_camera: &mut FpsCamera, actions: &Vec<Action>) {
  fps_camera.process_actions(actions);
}

fn update_world(display: &Display, world: &mut Vec<Object>, gui: &mut Gui, actions: &Vec<Action>) {
  for object in world {
    if let Some(ref mut drawable) = object.drawable {
      drawable.update(display, object.transform, actions);
    }

    update_world(display, &mut object.children, gui, actions);
  }
}

enum StereoMode {
  StereoNone,
  StereoCross,
  StereoAnaglyph,
}

fn draw_frame(
    quality: &Quality,
    vr_mode: bool,
    stereo_mode: &StereoMode,
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
    canvas: &mut AdaptiveCanvas,
    frame_performance: &mut FramePerformance,
    render_dimensions: &mut (u32, u32),
    fps_camera: &mut FpsCamera,
    gui: &mut Gui,
    demo: &mut Option<Demo>,
    demo_record: bool,
    show_bbox: bool) {

  let aspect_ratio = render_dimensions.0 as f32 / render_dimensions.1 as f32;
  let mono_projection = cgmath::perspective(Deg(45.0), aspect_ratio * 2.0, 0.01f32, 1000.0);
  let stereo_projection = cgmath::perspective(Deg(45.0), aspect_ratio, 0.01f32, 1000.0);

  let (
      standing_transform,
      left_projection_matrix,
      right_projection_matrix,
      mut left_view_matrix,
      mut right_view_matrix) = if vr_mode {
    frame_performance.process_event("pre_sync_poses");
    vr_display.unwrap().borrow_mut().sync_poses();
    frame_performance.process_event("post_sync_poses");

    let display_data = vr_display.unwrap().borrow().data();

    let standing_transform = if let Some(ref stage) = display_data.stage_parameters {
      math::vec_to_matrix(&stage.sitting_to_standing_transform).inverse_transform().unwrap()
    } else {
      // Stage parameters not available yet or unsupported
      // Assume 0.75m transform height
      math::vec_to_translation(&[0.0, 0.75, 0.0]).inverse_transform().unwrap()
    };

    frame_performance.process_event("pre_sync_frame_data");
    let frame_data = vr_display.unwrap().borrow().synced_frame_data(0.1, 1000.0);
    frame_performance.process_event("post_sync_frame_data");

    let left_projection_matrix = math::vec_to_matrix(&frame_data.left_projection_matrix);
    let right_projection_matrix = math::vec_to_matrix(&frame_data.right_projection_matrix);
    let left_view_matrix = math::vec_to_matrix(&frame_data.left_view_matrix);
    let right_view_matrix = math::vec_to_matrix(&frame_data.right_view_matrix);

    (standing_transform, left_projection_matrix, right_projection_matrix, left_view_matrix,
        right_view_matrix)
  } else {
    frame_performance.process_event("pre_sync_poses");
    frame_performance.process_event("post_sync_poses");

    frame_performance.process_event("pre_sync_frame_data");
    let standing_transform = Matrix4::<f32>::identity();
    let view = fps_camera.get_view(0.016); // TODO: get actual timedelta
    frame_performance.process_event("post_sync_frame_data");

    let left_translation = Matrix4::from_translation(Vector3::new(-0.01, 0.0, 0.0));
    let left_view = left_translation * view;
    let right_translation = Matrix4::from_translation(Vector3::new(0.01, 0.0, 0.0));
    let right_view = right_translation * view;
    (standing_transform, stereo_projection, stereo_projection, left_view, right_view)
  };

  let inverse_standing_transform = standing_transform.inverse_transform().unwrap();

  frame_performance.process_event("pre_draw");

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
    let eyes = match stereo_mode {
      &StereoMode::StereoNone => vec![
        (&canvas.viewport, &mono_projection, &left_view_matrix, (true, true, true, true)),
      ],
      &StereoMode::StereoCross => vec![
        (&canvas.viewports[0], &left_projection_matrix, &left_view_matrix, (true, true, true, true)),
        (&canvas.viewports[1], &right_projection_matrix, &right_view_matrix, (true, true, true, true)),
      ],
      &StereoMode::StereoAnaglyph => vec![
        (&canvas.viewport, &mono_projection, &left_view_matrix, (true, false, false, true)),
        (&canvas.viewport, &mono_projection, &right_view_matrix, (false, true, true, true)),
      ],
    };

    let is_anaglyph = if let &StereoMode::StereoAnaglyph = stereo_mode { true } else { false };

    let mut framebuffer = canvas.get_framebuffer(display).unwrap();
    framebuffer.clear_color(0.4, 0.4, 0.4, 1.0);

    for (eye_i, eye) in eyes.iter().enumerate() {
      framebuffer.clear_depth(1.0);

      let projection = math::matrix_to_uniform(*eye.1);
      let view = math::matrix_to_uniform(eye.2 * standing_transform);
      let viewport = *eye.0;

      render_params.color_mask = eye.3;
      render_params.viewport = Some(viewport);

      let mut i = 0;
      let target_lod = quality.get_target_levels().2;
      for object in world.iter_mut() {
        if target_lod > (i as f32 / num_objects as f32) {
          i = object.draw(target_lod, i, num_objects, &mut framebuffer, display, projection, view, &render_params, num_lights, lights, eye_i, is_anaglyph, show_bbox);
        }
      }

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
        gamepad_models[i].draw(1.0, 0, 1, &mut framebuffer, display, projection, view, &render_params, num_lights, lights, eye_i, is_anaglyph, show_bbox);
      }

      empty.draw(1.0, 0, 1, &mut framebuffer, display, projection, view, &render_params, num_lights, lights, eye_i, is_anaglyph, show_bbox);

      canvas.resolve(display);

      gui.draw(&mut canvas.get_resolved_framebuffer(display).unwrap(), *eye.0);
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
      width: canvas.viewport.width,
      height: canvas.viewport.height,
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

    frame_performance.process_event("post_draw");

    target.finish().unwrap();
  }

  //assert_no_gl_error!(*display);

  // if !vr_mode {
  //   display.finish();
  // }

}

fn main() {
  let mut open_filename = "".to_string();
  let mut save_filename = "".to_string();
  let mut perf_filename = "".to_string();
  let mut demo_filename = "".to_string();
  let mut demo_record = false;
  let mut demo_length = -1i32;
  let mut weights = Vec::<f32>::new();
  let mut enable_supersampling = true;
  let mut visualize_perf = false;

  {
    let mut ap = ArgumentParser::new();
    ap.set_description("Engyn: a configurable adaptive quality graphics engine.");
    ap.add_option(&["-V", "--version"],
        Print(env!("CARGO_PKG_VERSION").to_string()), "show version");
    ap.refer(&mut open_filename)
      .add_option(&["-o", "--open"], Store, "open scene from .yml file");
    ap.refer(&mut save_filename)
      .add_option(&["-s", "--save"], Store, "save scene to .yml file");
    ap.refer(&mut perf_filename)
      .add_option(&["-p", "--perf"], Store, "performance measurements");
    ap.refer(&mut visualize_perf)
      .add_option(&["--visualize", "--vis"], StoreTrue, "visualize performance measurements");
    ap.refer(&mut demo_filename)
      .add_option(&["-d", "--demo-filename"], Store, "file to use for playing demos (or record)");
    ap.refer(&mut demo_length)
      .add_option(&["-t", "--trim"], Store, "trim the demo to length (in frames)");
    ap.refer(&mut demo_record)
      .add_option(&["-r", "--record"], StoreTrue, "set this to record demo instead of playback");
    ap.refer(&mut weights)
      .add_option(&["--weights"], List, "quality weights");
    ap.refer(&mut enable_supersampling)
      .add_option(&["--no-supersampling"], StoreFalse, "limit maximum resolution to monitor \
          resolution");

    ap.parse_args_or_exit();
  }

  if save_filename != "" {
    let scene = Scene::new();
    scene.to_yaml(&save_filename).unwrap();
  }

  let mut demo = if demo_record {
    println!("Recording demo {}", demo_filename);
    Some(Demo::new())
  } else if demo_filename != "" {
    let demo = Demo::from_bincode(&demo_filename).unwrap();
    println!("Playing back demo {} ({} frames)", demo_filename, demo.entries.len());
    Some(demo)
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

  let marble_material = Rc::new(RefCell::new(Material {
    albedo_map: resource_manager.get_texture(&Path::new("data/marble.jpg")).unwrap(),
    ambient_color: [0.0, 0.0, 0.0],
    diffuse_color: [0.0, 0.0, 0.0],
    specular_color: [1.0, 1.0, 1.0],
    shininess: 1.0,
    metalness: 0.0,
    reflectivity: 0.0,
  }));

  let canvas_dimensions = if enable_supersampling {
    (render_dimensions.0 * 4, render_dimensions.1 * 2)
  } else {
    (render_dimensions.0 * 2, render_dimensions.1)
  };

  let mut canvas = AdaptiveCanvas::new(&display, canvas_dimensions.0, canvas_dimensions.1, 3);

  let mut world = Vec::new();
  let mut num_lights = 0;
  let mut lights: [Light; uniforms::MAX_NUM_LIGHTS] = Default::default();

  if visualize_perf && perf_filename != "" {
    world.push(Benchmark::from_file(&display, &Path::new(&perf_filename)).as_object());
  } else if open_filename != "" {
    let scene = Scene::from_yaml(&open_filename).unwrap();
    world.push(scene.as_object(&display, &resource_manager));

    for (i, light) in scene.lights.iter().enumerate() {
      if i < uniforms::MAX_NUM_LIGHTS {
        lights[i] = *light;
      }
    }

    num_lights = usize::min(scene.lights.len(), uniforms::MAX_NUM_LIGHTS) as i32;
  } else {
    // a triangle
    world.push(Object::new_triangle(&display, &resource_manager, Rc::clone(&marble_material),
        [1.0, 1.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]));

    // a terrain mesh
    let mut terrain = Object::from_file(&display, &resource_manager,
        &Path::new("data/terrain.obj"));
    terrain.transform = Matrix4::identity();
    world.push(terrain);

    // a teapot

    let my_teapot_texcoords = {
      let mut texcoords = [Texcoord { texcoord: (0.0, 0.0) }; 531];

      for i in 0..texcoords.len() {
        texcoords[i].texcoord = rand::random::<(f32, f32)>();
      }

      texcoords
    };

    let my_teapot_bounding_box = {
      let mut bounding_box = (
        [f32::INFINITY; 3],
        [f32::NEG_INFINITY; 3],
      );

      for vertex in teapot::VERTICES.iter() {
        bounding_box.0[0] = bounding_box.0[0].min(vertex.position.0);
        bounding_box.0[1] = bounding_box.0[1].min(vertex.position.1);
        bounding_box.0[2] = bounding_box.0[2].min(vertex.position.2);
        bounding_box.1[0] = bounding_box.1[0].max(vertex.position.0);
        bounding_box.1[1] = bounding_box.1[1].max(vertex.position.1);
        bounding_box.1[2] = bounding_box.1[2].max(vertex.position.2);
      }

      bounding_box
    };

    let my_teapot = Object {
      children: Vec::new(),
      drawable: Some(Box::new(Mesh::new(
          &display,
          Rc::new(RefCell::new(Geometry {
            bounding_box: my_teapot_bounding_box,
            indices: Some(IndexBuffer::new(
                &display,
                PrimitiveType::TrianglesList,
                &teapot::INDICES).unwrap()),
            normals: VertexBuffer::new(&display, &teapot::NORMALS).unwrap(),
            vertices: VertexBuffer::new(&display, &teapot::VERTICES).unwrap(),
            texcoords: VertexBuffer::new(&display, &my_teapot_texcoords).unwrap(),
          })),
          Rc::clone(&marble_material),
          &resource_manager))),
      transform: Matrix4::new(
          0.005, 0.0, 0.0, 0.0,
          0.0, 0.005, 0.0, 0.0,
          0.0, 0.0, 0.005, 0.0,
          0.0, 1.0, 0.0, 1.0),
      size: (0..3).map(|i| (my_teapot_bounding_box.1[i] - my_teapot_bounding_box.0[i]).powi(2)).sum(),
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
        size: 0.0,
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
        size: 0.0,
    };

    world.push(my_network);

    // add a light

    num_lights = 1;
    lights[0] = Light { color: [1.0, 0.9, 0.9], position: [10.0, 10.0, 10.0] };
    // lights[1] = Light { color: [0.9, 1.0, 0.9], position: [10.0, 10.0, -10.0] };
    // lights[2] = Light { color: [0.9, 0.9, 1.0], position: [-10.0, 10.0, -10.0] };
    // lights[3] = Light { color: [1.0, 1.0, 1.0], position: [-10.0, 10.0, 10.0] };
  }

  let num_objects = calculate_num_objects(&world);

  // empty texture to force glutin clean
  let mut empty = Object::new_plane(&display, &resource_manager, Rc::new(RefCell::new(Material {
        albedo_map: resource_manager.get_texture(&Path::new("data/empty.bmp")).unwrap(),
        ambient_color: [0.0, 0.0, 0.0],
        diffuse_color: [0.0, 0.0, 0.0],
        specular_color: [0.0, 0.0, 0.0],
        shininess: 0.0,
        metalness: 0.0,
        reflectivity: 0.0,
      })),
      [0.0001,0.0001], [-0.1, 0.1, 0.0], [0.0, 0.0, 0.0], [-1.0,1.0,1.0]);

  let mut render_params = DrawParameters {
    depth: Depth { test: DepthTest::IfLess, write: true, .. Default::default() },
    //backface_culling: BackfaceCullingMode::CullClockwise,
    .. Default::default()
  };

  let mut fps_camera = FpsCamera::new(Vector3::new(0.0, 1.8, 3.0));
  fps_camera.pitch = Rad(-f32::consts::PI / 8.0);

  // create a model for each gamepad
  let gamepads = vr.get_gamepads();
  let mut gamepad_models = Vec::new();

  println!("Found {} controller{}!", gamepads.len(), match gamepads.len() { 1 => "", _ => "s" });

  for _ in &gamepads {
    println!("We've found a gamepad!");
    let gamepad_model_path = Path::new("data/vive-controller.obj");
    let gamepad_model = Object::from_file(&display, &resource_manager, &gamepad_model_path);
    gamepad_models.push(gamepad_model);
  }

  let mut input_handler = InputHandler::new(gamepads.len());
  let mut quality = Quality::new(if weights.len() >= 3 {
    (weights[0], weights[1], weights[2])
  } else {
    (0.5, 0.1, 1.0)
  });
  let mut gui = Gui::new(&display, Rc::clone(&quality.weight_resolution),
      Rc::clone(&quality.weight_msaa), Rc::clone(&quality.weight_lod));
  let mut frame_performance = FramePerformance::new(vr_mode);

  let num_iterations = 50;
  let target_steps = 1.0 / (num_iterations - 1) as f32;

  let mut stereo_mode = StereoMode::StereoCross;
  let mut show_bbox = false;

  if let Some(d) = vr_display {
    d.borrow_mut().start_present(Some(VRFramebufferAttributes {
      depth: false,
      multisampling: false,
      multiview: false,
    }));
  }

  let configurations = if benchmarking {
    let mut c = Vec::new();
    let seed = [
        4, 8, 15, 16, 23, 42,
        4, 8, 15, 16, 23, 42,
        4, 8, 15, 16, 23, 42,
        4, 8, 15, 16, 23, 42,
        4, 8, 15, 16, 23, 42,
        4, 8,
    ];
    let mut rng = Hc128Rng::from_seed(seed);

    for _ in 0..num_iterations {
      let mut configuration = rng.gen::<(f32, f32, f32)>();
      if weights.len() >= 3 {
        if weights[0] >= 0.0 && weights[0] <= 1.0 {
          configuration.0 = weights[0];
        }

        if weights[1] >= 0.0 && weights[1] <= 1.0 {
          configuration.1 = weights[1];
        }

        if weights[2] >= 0.0 && weights[2] <= 1.0 {
          configuration.2 = weights[2];
        }
      }
      c.push(configuration);
    }

    c
  } else {
    vec![(0.0, 0.0, 0.0)]
  };

  println!("Configurations:");
  for c in &configurations {
    println!("{} {} {}", c.0, c.1, c.2);
  }

  for c in &configurations {
    frame_performance.reset_frame_count();

    if benchmarking {
      quality = Quality::new(*c);
    }

    'main: loop {
      quality.set_level(&frame_performance);
      let targets = quality.get_target_levels();
      canvas.set_resolution_scale(targets.0);
      canvas.set_msaa_scale(targets.1);

      frame_performance.start_frame(&quality);
      frame_performance.process_event("frame_start");
      frame_performance.process_event("pre_input");

      // prepare GUI and handle its actions
      let gui_action = gui.prepare(*quality.level.borrow());

      // get input and handle its actions
      let input_actions = input_handler.process(&gui_action, &gamepads, &mut vr, &display, &window,
          vr_mode, &mut events_loop, &mut gui);

      for action in &input_actions {
        match action {
          &Action::Quit => break 'main,
          &Action::StereoNone => stereo_mode = StereoMode::StereoNone,
          &Action::StereoCross => stereo_mode = StereoMode::StereoCross,
          &Action::StereoAnaglyph => stereo_mode = StereoMode::StereoAnaglyph,
          &Action::ToggleBoundingBox => show_bbox = !show_bbox,
          _ => (),
        }

        if let &Action::Quit = action {
          break 'main
        }
      }

      frame_performance.process_event("post_input");

      frame_performance.process_event("pre_update_camera");
      update_camera(&mut fps_camera, &input_actions);
      frame_performance.process_event("post_update_camera");

      frame_performance.process_event("pre_update_world");
      update_world(&display, &mut world, &mut gui, &input_actions);
      frame_performance.process_event("post_update_world");

      draw_frame(&quality, vr_mode, &stereo_mode, vr_display, &display, &window,
          &mut render_params, &mut world, num_objects, &lights, num_lights, &mut empty,
          &gamepads, &mut gamepad_models, &mut canvas, &mut frame_performance,
          &mut render_dimensions, &mut fps_camera, &mut gui, &mut demo, demo_record, show_bbox);

      frame_performance.process_event("frame_end");
      frame_performance.record_frame_log();

      // quit when demo is done
      if let Some(d) = demo.as_mut() {
        if !demo_record && frame_performance.get_frame_number() as usize >= d.entries.len() {
          break 'main;
        }
      }
    }
  }

  let now = Utc::now().format("%Y-%m-%d-%H-%M-%S");

  if !visualize_perf && (benchmarking || perf_filename != "") {
    // write benchmark csv
    let csv = frame_performance.to_csv();
    let mut file = File::create(format!("{}-{}.csv", perf_filename, now)).unwrap();
    file.write_all(csv.as_bytes()).unwrap();
  }

  if demo_record || demo_length > 0 {
    if let Some(d) = demo.as_mut() {
      let filename = if demo_filename != "" {
        demo_filename.to_string()
      } else {
        format!("performance/{}.demo", now)
      };

      if demo_length <= 0 {
        d.to_bincode(&filename).unwrap();
      } else {
        let mut new_demo = Demo::new();
        let step_size = d.entries.len() / demo_length as usize;

        for entry in d.entries.iter().step(step_size) {
          if new_demo.entries.len() < 180 {
            new_demo.entries.push(entry.clone());
          } else {
            break;
          }
        }

        new_demo.to_bincode(&filename).unwrap();
      }
    }
  }
}
