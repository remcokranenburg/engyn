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
use std::cmp::Ordering;

use benchmark::Benchmark;
use benchmark::BenchmarkEntry;
use performance::FramePerformance;

pub struct Quality {
  pub adaptive_resolution: bool,
  pub adaptive_msaa: bool,
  pub adaptive_lod: bool,
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
      adaptive_resolution: true,
      adaptive_msaa: true,
      adaptive_lod: true,
      level: Rc::new(RefCell::new(1.0)),
      weight_resolution: Rc::new(RefCell::new(weight_resolution)),
      weight_msaa: Rc::new(RefCell::new(weight_msaa)),
      weight_lod: Rc::new(RefCell::new(weight_lod)),

      enable_supersampling: enable_supersampling,
    }
  }

  pub fn set_level(&self, frame_performance: &FramePerformance) {
    let predicted_remaining_time = frame_performance.get_predicted_remaining_time();
    let target_frame_time = frame_performance.get_target_frame_time();
    let ratio_remaining = predicted_remaining_time as f32 / target_frame_time as f32;

    // println!("target: {}, remaining: {}, ratio: {}", target_frame_time, predicted_remaining_time, ratio_remaining);

    let original_level = *self.level.borrow();

    if ratio_remaining < 0.05 {
      *self.level.borrow_mut() = f32::max(original_level * 0.5, 0.0001);
    } else if ratio_remaining < 0.2 {
      *self.level.borrow_mut() = f32::max(original_level * 0.99, 0.0001);
    } else if ratio_remaining > 0.8 {
      *self.level.borrow_mut() = f32::min(original_level * 2.0, 1.0);
    } else {
      *self.level.borrow_mut() = f32::min(original_level * 1.01, 1.0);
    }
  }

  fn mix(x: f32, y: f32, a: f32) -> f32 {
    a * x + (1.0 - a) * y
  }

  pub fn set_target_resolution(&mut self, value: f32) {
    self.adaptive_resolution = false;
    *self.weight_resolution.borrow_mut() = value;
  }

  pub fn get_target_resolution(&self) -> f32 {
    if self.adaptive_resolution {
      let default_target = if self.enable_supersampling { 0.5 } else { 1.0 };
      Quality::mix(*self.level.borrow(), default_target, *self.weight_resolution.borrow())
    } else {
      *self.weight_resolution.borrow()
    }
  }

  pub fn set_target_msaa(&mut self, value: f32) {
    self.adaptive_msaa = false;
    *self.weight_msaa.borrow_mut() = value;
  }

  pub fn get_target_msaa(&self) -> f32 {
    if self.adaptive_msaa {
      Quality::mix(*self.level.borrow(), 0.0, *self.weight_msaa.borrow())
    } else {
      *self.weight_msaa.borrow()
    }
  }

  pub fn set_target_lod(&mut self, value: f32) {
    self.adaptive_lod = false;
    *self.weight_lod.borrow_mut() = value;
  }

  pub fn get_target_lod(&self) -> f32 {
    if self.adaptive_lod {
      Quality::mix(*self.level.borrow(), 1.0, *self.weight_lod.borrow())
    } else {
      *self.weight_lod.borrow()
    }
  }
}
