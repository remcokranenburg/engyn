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

use webvr::VRGamepadPtr;

use gui::Action;

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

  pub fn process(&mut self, gamepads: &Vec<VRGamepadPtr>) -> Action {
    for (i, ref gamepad) in gamepads.iter().enumerate() {
      let state = gamepad.borrow().state();

      if state.buttons[0].pressed {
        self.grip_button_pressed[i] = true;
      } else if self.grip_button_pressed[i] {
        self.grip_button_pressed[i] = false;
        println!("grip button clicked");
        return Action::GuiSelectNext;
      }

      if state.buttons[1].pressed {
        self.menu_button_pressed[i] = true;
      } else if self.menu_button_pressed[i] {
        self.menu_button_pressed[i] = false;
        println!("menu button clicked");
        return Action::GuiToggleMenu;
      }

      if state.axes[2] == 1.0 {
        self.trigger_button_pressed[i] = true;
      } else if self.trigger_button_pressed[i] {
        self.trigger_button_pressed[i] = false;
        println!("trigger button clicked");
        return Action::GuiActivateMenuItem;
      }

      if state.axes[0] > 0.0 {
        return Action::ChangeWeight(state.axes[0] as f32);
      }
    }

    Action::None
  }
}
