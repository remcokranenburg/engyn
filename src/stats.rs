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

use std::time::Duration;
use std::time::Instant;
use std::thread;

// these target frame times will cause vsync misses so it hits the desired frame rate
const TARGET_FRAME_TIMES: [u32; 4] = [
    11_000_000u32,  // 90fps target for 90fps
    22_000_000u32,  // 45fps target for 45fps
    33_000_000u32,  // 30fps target for 30fps
    66_000_000u32,  // 20fps target for 15fps
];

fn duration_as_millis(duration: Duration) -> f64 {
  ((duration.as_secs() * 1000) as f64) + (duration.subsec_nanos() as f64 / 1_000_000f64)
}

pub struct Stats {
  time_last_frame: Instant,
  time_last_update: Instant,
  frame_count: u32,
  current_fps_target: usize,
}

impl Stats {
  pub fn new() -> Stats {
    Stats {
      time_last_frame: Instant::now(),
      time_last_update: Instant::now(),
      frame_count: 0,
      current_fps_target: 0,
    }
  }

  pub fn reduce_fps(&mut self) {
    if self.current_fps_target < TARGET_FRAME_TIMES.len() - 1 {
      self.current_fps_target += 1;
      println!("Target frame time: {:?}", TARGET_FRAME_TIMES[self.current_fps_target] / 1_000_000u32);
    }
  }

  pub fn increase_fps(&mut self) {
    if self.current_fps_target > 0 {
      self.current_fps_target -= 1;
      println!("Target frame time: {:?}", TARGET_FRAME_TIMES[self.current_fps_target] / 1_000_000u32);
    }
  }

  pub fn process_frame_end(&mut self) {
    self.frame_count += 1;
    let time_frame_end = Instant::now();
    let sum_frame_time = time_frame_end.duration_since(self.time_last_update);
    let current_frame_time = time_frame_end.duration_since(self.time_last_frame);

    if sum_frame_time >= Duration::new(1, 0) {
      let sum_frame_time_as_millis = duration_as_millis(sum_frame_time);
      let fps = self.frame_count as f64 / (sum_frame_time_as_millis / 1000f64);
      let frame_time = sum_frame_time_as_millis / self.frame_count as f64;
      println!("Avg FPS: {} ({}ms), dropped {} frames",
          fps,
          frame_time,
          90 - self.frame_count);
      self.frame_count = 0;
      self.time_last_update = time_frame_end;
    }

    let target_frame_time = Duration::new(0, TARGET_FRAME_TIMES[self.current_fps_target]);

    if self.current_fps_target > 0 {
      thread::sleep(target_frame_time - current_frame_time);
    }

    self.time_last_frame = time_frame_end;
  }
}
