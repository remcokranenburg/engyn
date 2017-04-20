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

pub struct Stats {
  time_last_frame: Instant,
  time_last_update: Instant,
  frame_count: u64,
}

impl Stats {
  pub fn new() -> Stats {
    Stats {
      time_last_frame: Instant::now(),
      time_last_update: Instant::now(),
      frame_count: 0,
    }
  }

  pub fn process_frame_end(&mut self) {
    self.frame_count += 1;
    let time_frame_end = Instant::now();
    let sum_frame_time = time_frame_end.duration_since(self.time_last_update);

    let current_frame_time = time_frame_end.duration_since(self.time_last_frame);
    if current_frame_time > Duration::from_millis(20) {
      let current_frame_time_as_millis = (current_frame_time.as_secs() * 1000) as f32 +
          (current_frame_time.subsec_nanos() as f32 / 1_000_000f32);
      println!("Frame drop: {}ms", current_frame_time_as_millis);
    }
    self.time_last_frame = time_frame_end;

    if sum_frame_time >= Duration::new(1, 0) {
      let sum_frame_time_as_millis = (sum_frame_time.as_secs() * 1000) as f32 +
          (sum_frame_time.subsec_nanos() as f32 / 1_000_000f32);
      let fps = self.frame_count as f32 / (sum_frame_time_as_millis / 1000f32);
      let frame_time = sum_frame_time_as_millis / self.frame_count as f32;
      println!("Avg FPS: {} (Avg frame time: {}ms)", fps, frame_time);
      self.frame_count = 0;
      self.time_last_update = time_frame_end;
    }
  }
}
