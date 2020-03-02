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
use webvr::VRDisplayPtr;

use performance::FramePerformance;

pub struct Quality {
  pub adaptive_quality: bool,
  pub level: Rc<RefCell<f32>>,
  pub weight_resolution: Rc<RefCell<f32>>,
  pub weight_msaa: Rc<RefCell<f32>>,
  pub weight_lod: Rc<RefCell<f32>>,
  pub quality_stats: (u32, u32, f32),
}

impl Quality {
  pub fn new(weights: (f32, f32, f32)) -> Quality {
    let (weight_resolution, weight_msaa, weight_lod) = weights;

    Quality {
      adaptive_quality: true,
      level: Rc::new(RefCell::new(0.5)),
      weight_resolution: Rc::new(RefCell::new(weight_resolution)),
      weight_msaa: Rc::new(RefCell::new(weight_msaa)),
      weight_lod: Rc::new(RefCell::new(weight_lod)),
      quality_stats: (0, 0, 0.0)
    }
  }

  pub fn set_level(&mut self, frame_performance: &FramePerformance, vr_display: Option<&VRDisplayPtr>) {
    let predicted_remaining_time = frame_performance.get_predicted_remaining_time(vr_display);
    let target_frame_time = frame_performance.get_target_frame_time();
    let ratio_remaining = f32::max(0.0, predicted_remaining_time as f32 / target_frame_time as f32);

    // println!("target: {}, remaining: {}, ratio: {}", target_frame_time, predicted_remaining_time, ratio_remaining);

    const EMERGENCY_ZONE: f32 = 0.05;   // 0.00 - 0.05
    const DANGER_ZONE: f32 = 0.1;       // 0.05 - 0.10
    const SAFE_ZONE: f32 = 0.3;         // 0.10 - 0.30
    const EASY_ZONE: f32 = 0.8;         // 0.30 - 0.80
    // IDLE_ZONE                           0.80 - 1.00

    let original_level = *self.level.borrow();

    if ratio_remaining < EMERGENCY_ZONE {
      *self.level.borrow_mut() = f32::max(original_level * 0.5, 0.0001);
    } else if ratio_remaining < DANGER_ZONE {
       *self.level.borrow_mut() = f32::max(original_level * 0.99, 0.0001);
    } else if ratio_remaining < SAFE_ZONE {
      // in safe zone, do nothing
    } else if ratio_remaining < EASY_ZONE {
      *self.level.borrow_mut() = f32::min(original_level * 1.01, 1.0);
    } else {
      *self.level.borrow_mut() = f32::min(original_level * 2.0, 1.0);
    }

    self.quality_stats = (target_frame_time, predicted_remaining_time, ratio_remaining);
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
