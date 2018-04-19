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

use cgmath::Rad;
use conrod;
use glium::Display;
use glium::glutin::Event;
use glium::glutin::EventsLoop;
use glium::glutin::Window;
use glium::glutin::WindowEvent;
use glium::glutin::KeyboardInput;
use glium::glutin::ElementState;
use glium::glutin::VirtualKeyCode;
use std::f32;
use webvr::VREvent;
use webvr::VRDisplayEvent;
use webvr::VRGamepadPtr;
use webvr::VRServiceManager;

use gui::Action;
use gui::Gui;

pub struct InputHandler {
  grip_button_pressed: Vec<bool>,
  menu_button_pressed: Vec<bool>,
  trigger_button_pressed: Vec<bool>,

}

impl InputHandler {
  pub fn new(num_gamepads: usize) -> InputHandler {
    let mut g = Vec::new();
    let mut m = Vec::new();
    let mut t = Vec::new();

    for _ in 0 .. num_gamepads {
      g.push(false);
      m.push(false);
      t.push(false);
    }

    InputHandler {
      grip_button_pressed: g,
      menu_button_pressed: m,
      trigger_button_pressed: t,
    }
  }

  pub fn process(&mut self, gui_action: &Action, gamepads: &Vec<VRGamepadPtr>,
      vr: &mut VRServiceManager, display: &Display, window: &Window, vr_mode: bool,
      events_loop: &mut EventsLoop, gui: &mut Gui) -> Vec<Action> {

    let actions = {
      let mut actions = Vec::new();
      actions.push(gui.process_gui_action(gui_action, window, vr_mode));
      actions.append(&mut self.process_gamepad_state(gamepads));
      actions.append(&mut self.process_vr_events(vr));
      actions.append(&mut self.process_glutin_events(display, window, vr_mode, events_loop, gui));
      actions
    };

    let mut result_actions = gui.process_actions(&actions, window, vr_mode);

    for action in actions { result_actions.push(action) }

    result_actions
  }

  fn process_gamepad_state(&mut self, gamepads: &Vec<VRGamepadPtr>) -> Vec<Action> {
    let mut actions = Vec::new();

    for (i, ref gamepad) in gamepads.iter().enumerate() {
      let state = gamepad.borrow().state();

      if state.buttons[0].pressed {
        self.grip_button_pressed[i] = true;
      } else if self.grip_button_pressed[i] {
        self.grip_button_pressed[i] = false;
        println!("grip button clicked");
        actions.push(Action::GuiSelectNext);
      }

      if state.buttons[1].pressed {
        self.menu_button_pressed[i] = true;
      } else if self.menu_button_pressed[i] {
        self.menu_button_pressed[i] = false;
        println!("menu button clicked");
        actions.push(Action::GuiToggleMenu);
      }

      if state.axes[2] == 1.0 {
        self.trigger_button_pressed[i] = true;
      } else if self.trigger_button_pressed[i] {
        self.trigger_button_pressed[i] = false;
        println!("trigger button clicked");
        actions.push(Action::GuiActivateMenuItem);
      }

      if state.axes[0] > 0.0 {
        actions.push(Action::ChangeWeight(state.axes[0] as f32));
      }
    }

    actions
  }

  fn process_vr_events(&self, vr: &mut VRServiceManager) -> Vec<Action> {
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

    vec![]
  }

  fn process_glutin_events(&self, display: &Display, window: &Window, vr_mode: bool,
      events_loop: &mut EventsLoop, gui: &mut Gui) -> Vec<Action> {
    let mut actions = Vec::new();

    events_loop.poll_events(|event| {
      if let Some(event) = conrod::backend::winit::convert_event(event.clone(), display) {
        gui.handle_event(event);
      }

      match event {
        Event::WindowEvent { event, .. } => match event {
          WindowEvent::Closed => actions.push(Action::Quit),
          WindowEvent::Resized(width, height) => {
            println!("resized to {}x{}", width, height);
            actions.push(Action::Resize(width / 2, height));
          },
          WindowEvent::KeyboardInput { input, .. } => {
            let key_is_pressed = input.state == ElementState::Pressed;

            match input {
              KeyboardInput { virtual_keycode, .. } => match virtual_keycode {

                // the following are instantaneous actions
                Some(VirtualKeyCode::Q)         => if gui.is_visible { actions.push(Action::Quit) },
                Some(VirtualKeyCode::Escape)    => if key_is_pressed { actions.push(Action::GuiToggleMenu) },
                Some(VirtualKeyCode::Up)        => if key_is_pressed { actions.push(Action::GuiSelectPrevious) },
                Some(VirtualKeyCode::Down)      => if key_is_pressed { actions.push(Action::GuiSelectNext) },
                Some(VirtualKeyCode::Left)      => if key_is_pressed { actions.push(Action::GuiDecreaseSlider) },
                Some(VirtualKeyCode::Right)     => if key_is_pressed { actions.push(Action::GuiIncreaseSlider) },
                Some(VirtualKeyCode::Return)    => if key_is_pressed { actions.push(Action::GuiActivateMenuItem) },
                Some(VirtualKeyCode::H)         => if key_is_pressed { actions.push(Action::ConicEccentricityDecrease) },
                Some(VirtualKeyCode::J)         => if key_is_pressed { actions.push(Action::ConicEccentricityIncrease) },
                Some(VirtualKeyCode::K)         => if key_is_pressed { actions.push(Action::ConicSlrDecrease) },
                Some(VirtualKeyCode::L)         => if key_is_pressed { actions.push(Action::ConicSlrIncrease) },
                Some(VirtualKeyCode::F1)        => if key_is_pressed { if !vr_mode { actions.push(Action::StereoNone) } },
                Some(VirtualKeyCode::F2)        => if key_is_pressed { actions.push(Action::StereoCross) },
                Some(VirtualKeyCode::F3)        => if key_is_pressed { actions.push(Action::StereoAnaglyph) },
                Some(VirtualKeyCode::Key1)      => if key_is_pressed { actions.push(Action::VisualizeOneD) },
                Some(VirtualKeyCode::Key2)      => if key_is_pressed { actions.push(Action::VisualizeTwoD) },
                Some(VirtualKeyCode::Key3)      => if key_is_pressed { actions.push(Action::VisualizeThreeD) },

                // the following are longer actions
                Some(VirtualKeyCode::W) => actions.push(Action::CameraMoveForward(key_is_pressed)),
                Some(VirtualKeyCode::S) => actions.push(Action::CameraMoveBackward(key_is_pressed)),
                Some(VirtualKeyCode::A) => actions.push(Action::CameraMoveLeft(key_is_pressed)),
                Some(VirtualKeyCode::D) => actions.push(Action::CameraMoveRight(key_is_pressed)),
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

              actions.push(Action::CameraRotate {
                pitch: -Rad(rel_y as f32 / 1000.0),
                yaw: -Rad(rel_x as f32 / 1000.0),
              });

              window.set_cursor_position(origin_x as i32, origin_y as i32).unwrap();
            }
          },
          _ => (),
        },
        _ => (),
      };
    });

    actions
  }
}
