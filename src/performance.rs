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

use std::time::Instant;
use std::fmt::Write;

const TARGET_FRAME_TIMES: [u32; 5] = [
    11_111_111u32,  // target for 90fps
    16_666_667u32,  // target for 60fps
    22_222_222u32,  // target for 45fps
    33_333_333u32,  // target for 30fps
    66_666_667u32,  // target for 15fps
];

pub struct LogEntry {
  pub frame_time: u32,
  pub sync_poses_time: u32,
  pub sync_frame_data_time: u32,
  pub draw_time: u32,
  pub post_draw_time: u32,
  pub quality: f32,
}

pub struct FramePerformance {
  log: Vec<LogEntry>,
  time_frame_start: Instant,
  time_sync_poses: Instant,
  time_sync_frame_data: Instant,
  time_draw_start: Instant,
  time_draw_end: Instant,
  time_frame_end: Instant,
  frame_count: i32,
  current_fps_target: usize,
}

impl FramePerformance {
  pub fn new(vr_mode: bool) -> FramePerformance {
    let now = Instant::now();

    FramePerformance {
      log: Vec::new(),
      time_frame_start: now,
      time_sync_poses: now,
      time_sync_frame_data: now,
      time_draw_start: now,
      time_draw_end: now,
      time_frame_end: now,
      frame_count: 0,
      current_fps_target: if vr_mode { 0 } else { 1 },
    }
  }

  pub fn process_frame_start(&mut self, quality: f32) {
    let time_new_frame = Instant::now();

    // write log entry for previous frame
    self.log.push(LogEntry {
      frame_time: time_new_frame.duration_since(self.time_frame_start).subsec_nanos(),
      sync_poses_time: self.time_sync_poses.duration_since(self.time_frame_start).subsec_nanos(),
      sync_frame_data_time: self.time_sync_frame_data.duration_since(self.time_sync_poses).subsec_nanos(),
      draw_time: self.time_draw_end.duration_since(self.time_draw_start).subsec_nanos(),
      post_draw_time: self.time_frame_end.duration_since(self.time_draw_end).subsec_nanos(),
      quality: quality,
    });

    self.frame_count += 1;
    self.time_frame_start = time_new_frame;
  }

  pub fn process_sync_poses(&mut self) {
    self.time_sync_poses = Instant::now();
  }

  pub fn process_sync_frame_data(&mut self) {
    self.time_sync_frame_data = Instant::now();
  }

  pub fn process_draw_start(&mut self) {
    self.time_draw_start = Instant::now();
  }

  pub fn process_draw_end(&mut self) -> u32 {
    self.time_draw_end = Instant::now();

    let current_frame_time = self.time_draw_end.duration_since(self.time_sync_poses)
      .subsec_nanos();

    if self.get_target_frame_time() < current_frame_time {
      0
    } else {
      self.get_target_frame_time() - current_frame_time
    }
  }

  pub fn process_frame_end(&mut self) {
    self.time_frame_end = Instant::now();
  }

  pub fn get_frame_number(&self) -> i32 {
    self.frame_count
  }

  pub fn get_target_frame_time(&self) -> u32 {
    TARGET_FRAME_TIMES[self.current_fps_target]
  }

  pub fn to_csv(&self) -> String {
    let mut log_csv = String::from("Frame,FPS,SyncPoses,SyncFrameData,Draw,PostDraw,Idle,Quality\n");
    for (i, frame) in self.log.iter().enumerate() {
      let fps = 1_000_000_000f64 / (frame.frame_time as f64);
      write!(&mut log_csv, "{},{},{},{},{},{},{},{}\n",
          i,
          fps,
          frame.sync_poses_time,
          frame.sync_frame_data_time,
          frame.draw_time,
          frame.post_draw_time,
          frame.frame_time - frame.sync_poses_time - frame.sync_frame_data_time - frame.draw_time - frame.post_draw_time,
          frame.quality).unwrap();
    }
    log_csv
  }
}
