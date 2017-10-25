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
use std::fmt::Write;

// these target frame times will cause vsync misses so it hits the desired frame rate
const TARGET_FRAME_TIMES: [u32; 5] = [
    13_300_000u32,  // target for 90fps
    20_000_000u32,  // target for 60fps
    25_000_000u32,  // target for 45fps
    40_000_000u32,  // target for 30fps
    77_000_000u32,  // target for 15fps
];

pub struct FramePerformance {
  log: Vec<(u32, f32)>,
  time_frame_start: Instant,
  frame_count: i32,
  current_fps_target: usize,
}

impl FramePerformance {
  pub fn new() -> FramePerformance {
    FramePerformance {
      log: Vec::new(),
      time_frame_start: Instant::now(),
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

  pub fn process_frame_start(&mut self) {
      self.frame_count += 1;
      self.time_frame_start = Instant::now();
  }

  pub fn process_frame_end(&mut self, vr_mode: bool, quality: f32) -> bool {
    let current_frame_time = Instant::now().duration_since(self.time_frame_start);
    let target_frame_time = Duration::new(0, TARGET_FRAME_TIMES[self.current_fps_target]);

    if self.current_fps_target > 0 && current_frame_time < target_frame_time {
      thread::sleep(target_frame_time - current_frame_time);
    }

    self.log.push((
        current_frame_time.subsec_nanos(),
        quality));
    current_frame_time > Duration::new(0, TARGET_FRAME_TIMES[if vr_mode { 0 } else { 2 }])
  }

  pub fn to_csv(&self) -> String {
    let mut log_csv = String::from("Frame,FPS,Quality\n");
    for (i, frame) in self.log.iter().enumerate() {
      let fps = 1_000_000_000f64 / (frame.0 as f64);
      write!(&mut log_csv, "{},{},{}\n", i, fps, frame.1).unwrap();
    }
    log_csv
  }
}
