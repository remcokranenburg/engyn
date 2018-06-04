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

use std::cell::RefCell;
use std::rc::Rc;
use std::f32;
use performance::FramePerformance;

pub struct Quality {
  pub adaptive_quality: bool,
  pub level: Rc<RefCell<f32>>,
  pub weight_resolution: Rc<RefCell<f32>>,
  pub weight_msaa: Rc<RefCell<f32>>,
  pub weight_lod: Rc<RefCell<f32>>,
}

impl Quality {
  pub fn new(weights: Vec<f32>) -> Quality {
    let (
      weight_resolution,
      weight_msaa,
      weight_lod,
    ) = if weights.len() == 3 { (
      weights[0],
      weights[1],
      weights[2],
    ) } else { (
      0.50,
      0.01,
      1.00,
    ) };

    Quality {
      adaptive_quality: true,
      level: Rc::new(RefCell::new(0.5)),
      weight_resolution: Rc::new(RefCell::new(weight_resolution)),
      weight_msaa: Rc::new(RefCell::new(weight_msaa)),
      weight_lod: Rc::new(RefCell::new(weight_lod)),
    }
  }

  pub fn set_level(&self, frame_performance: &FramePerformance) {
    let predicted_remaining_time = frame_performance.get_predicted_remaining_time();
    let target_frame_time = frame_performance.get_target_frame_time();
    let ratio_remaining = predicted_remaining_time as f32 / target_frame_time as f32;

    // println!("target: {}, remaining: {}, ratio: {}", target_frame_time, predicted_remaining_time, ratio_remaining);

    let original_level = *self.level.borrow();

    if ratio_remaining < 0.1 {
      *self.level.borrow_mut() = f32::max(original_level * 0.5, 0.0001);
    } else if ratio_remaining < 0.2 {
       *self.level.borrow_mut() = f32::max(original_level * 0.99, 0.0001);
    } else if ratio_remaining < 0.3 {
      // between 0.2 and 0.3, do nothing
    } else if ratio_remaining > 0.9 {
      *self.level.borrow_mut() = f32::min(original_level * 2.0, 1.0);
    } else {
      *self.level.borrow_mut() = f32::min(original_level * 1.01, 1.0);
    }
  }

  pub fn set_target_levels(&mut self, values: (f32, f32, f32)) {
    self.adaptive_quality = false;
    *self.weight_resolution.borrow_mut() = values.0;
    *self.weight_msaa.borrow_mut() = values.1;
    *self.weight_lod.borrow_mut() = values.2;
  }

  pub fn get_target_levels(&self) -> (f32, f32, f32) {
    let weight_resolution = *self.weight_resolution.borrow();
    let weight_msaa = *self.weight_msaa.borrow();
    let weight_lod = *self.weight_lod.borrow();

    if self.adaptive_quality {
      let lowest_weight = f32::max(0.01, f32::min(weight_resolution, f32::min(weight_msaa, weight_lod)));
      let level = *self.level.borrow();
      let denormalized_level = level / lowest_weight;
      (
        f32::min(1.0, weight_resolution * denormalized_level),
        f32::min(1.0, weight_msaa * denormalized_level),
        f32::min(1.0, weight_lod * denormalized_level),
      )
    } else {
      (weight_resolution, weight_msaa, weight_lod)
    }
  }
}
