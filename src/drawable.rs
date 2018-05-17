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

use cgmath::Matrix4;
use glium::DrawParameters;
use glium::backend::Facade;
use glium::framebuffer::SimpleFrameBuffer;
use std::f32;

use gui::Action;
use light::Light;

pub trait Drawable {
  fn draw(&mut self, target: &mut SimpleFrameBuffer, context: &Facade, projection: [[f32; 4]; 4],
      view: [[f32; 4]; 4], model_transform: Matrix4<f32>, render_params: &DrawParameters,
      num_lights: i32, lights: &[Light; 32], eye_i: usize, is_anaglyph: bool,
      show_bbox: bool);

  fn update(&mut self, context: &Facade, model_transform: Matrix4<f32>, actions: &Vec<Action>);
}
