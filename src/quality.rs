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

use std::cell::RefCell;
use std::rc::Rc;
use std::f32;

const BENCHMARK_BASE_LEVEL: f32 = 1.0;

pub struct Quality {
  pub benchmark_mode: String,
  pub level: Rc<RefCell<f32>>,
  pub weight_resolution: Rc<RefCell<f32>>,
  pub weight_msaa: Rc<RefCell<f32>>,
  pub weight_lod: Rc<RefCell<f32>>,

  enable_supersampling: bool,
}

impl Quality {
  pub fn new(weights: Vec<f32>, enable_supersampling: bool) -> Quality {
    let (
      weight_resolution,
      weight_msaa,
      weight_lod,
    ) = if weights.len() == 3 { (
      weights[0],
      weights[1],
      weights[2],
    ) } else { (
      1.0,
      1.0,
      1.0,
    ) };

    Quality {
      benchmark_mode: "".to_string(),
      level: Rc::new(RefCell::new(1.0)),
      weight_resolution: Rc::new(RefCell::new(weight_resolution)),
      weight_msaa: Rc::new(RefCell::new(weight_msaa)),
      weight_lod: Rc::new(RefCell::new(weight_lod)),

      enable_supersampling: enable_supersampling,
    }
  }

  pub fn set_benchmark_mode(&mut self, mode: &str, level: f32) {
    self.benchmark_mode = mode.to_string();
    *self.level.borrow_mut() = level;
  }

  pub fn set_level(&self, predicted_remaining_time: u32, target_frame_time: u32) {
    let ratio_remaining = predicted_remaining_time as f32 / target_frame_time as f32;

    let original_level = *self.level.borrow();

    if ratio_remaining < 0.05 {
      *self.level.borrow_mut() = f32::max(original_level * 0.5, 0.0);
    } else if ratio_remaining < 0.2 {
      *self.level.borrow_mut() = f32::max(original_level * 0.99, 0.0);
    } else {
      *self.level.borrow_mut() = f32::min(original_level * 1.01, 1.0);
    }
  }

  fn mix(x: f32, y: f32, a: f32) -> f32 {
    a * x + (1.0 - a) * y
  }

  pub fn get_target_resolution(&self) -> f32 {
    if self.benchmark_mode == "resolution" {
      *self.level.borrow()
    } else if self.benchmark_mode != "" {
      BENCHMARK_BASE_LEVEL
    } else {
      let default_target = if self.enable_supersampling { 0.5 } else { 1.0 };
      Quality::mix(*self.level.borrow(), default_target, *self.weight_resolution.borrow())
    }
  }

  pub fn get_target_msaa(&self) -> f32 {
    if self.benchmark_mode == "msaa" {
      *self.level.borrow()
    } else if self.benchmark_mode != "" {
      BENCHMARK_BASE_LEVEL
    } else {
      Quality::mix(*self.level.borrow(), 0.0, *self.weight_msaa.borrow())
    }
  }

  pub fn get_target_lod(&self) -> f32 {
    if self.benchmark_mode == "lod" {
      *self.level.borrow()
    } else if self.benchmark_mode != "" {
      BENCHMARK_BASE_LEVEL
    } else {
      Quality::mix(*self.level.borrow(), 1.0, *self.weight_lod.borrow())
    }
  }
}
