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

use std::cmp;
use std::collections::HashMap;
use std::fmt::Write;
use std::time::Instant;

use quality::Quality;

const TARGET_FRAME_TIMES: [u32; 5] = [
    11_111_111u32,  // target for 90fps
    16_666_667u32,  // target for 60fps
    22_222_222u32,  // target for 45fps
    33_333_333u32,  // target for 30fps
    66_666_667u32,  // target for 15fps
];

pub struct LogEntry {
  pub event_instants: HashMap<String, Instant>,
  pub target_resolution: f32,
  pub target_msaa: f32,
  pub target_lod: f32,
}

pub struct FramePerformance {
  log: Vec<LogEntry>,
  event_instants: HashMap<String, Instant>,
  current_fps_target: usize,
  target_resolution: f32,
  target_msaa: f32,
  target_lod: f32,
}

impl FramePerformance {
  pub fn new(vr_mode: bool) -> FramePerformance {
    FramePerformance {
      log: Vec::new(),
      event_instants: HashMap::new(),
      current_fps_target: if vr_mode { 0 } else { 1 },
      target_resolution: 0.0,
      target_msaa: 0.0,
      target_lod: 0.0,
    }
  }

  pub fn process_event(&mut self, event: &str) {
    self.event_instants.insert(event.to_owned(), Instant::now());
  }

  pub fn start_frame(&mut self, quality: &Quality) {
    let targets = quality.get_target_levels();
    self.target_resolution = targets.0;
    self.target_msaa = targets.1;
    self.target_lod = targets.2;
  }

  pub fn record_frame_log(&mut self) {
    self.log.push(LogEntry {
      event_instants: self.event_instants.clone(),
      target_resolution: self.target_resolution,
      target_msaa: self.target_msaa,
      target_lod: self.target_lod,
    });
  }

  pub fn get_frame_number(&self) -> usize {
    self.log.len()
  }

  pub fn get_remaining_time(&self) -> u32 {
    let mut log_rev_iter = self.log.iter().rev();

    let frame_duration = if self.log.len() >= 1 {
      // we have a previous frame, so we can calculate based on events from last frame
      let last_frame = log_rev_iter.next().unwrap();
      let measure_start = last_frame.event_instants.get("post_sync_poses").unwrap();
      let measure_end = last_frame.event_instants.get("post_draw").unwrap();
      measure_end.duration_since(*measure_start).subsec_nanos()
    } else {
      // we have no previous frames, so we assume no frame duration
      0u32
    };

    let remaining = cmp::max(0, self.get_target_frame_time() as i32 - frame_duration as i32) as u32;

    // println!("target: {}, actual: {}, remaining: {}", self.get_target_frame_time(), frame_duration,
    //     remaining);

    remaining
  }

  pub fn get_predicted_remaining_time(&self) -> u32 {
    let remaining_time = self.get_remaining_time();

    // predict next remaining time
    remaining_time
  }

  pub fn get_target_frame_time(&self) -> u32 {
    TARGET_FRAME_TIMES[self.current_fps_target]
  }

  pub fn get_actual_frame_time(&self, i: usize) -> u32 {
    let this_frame = self.log.get(i);
    let next_frame = self.log.get(i + 1);

    if let (Some(this_frame), Some(next_frame)) = (this_frame, next_frame) {
      let next_frame_start = next_frame.event_instants.get("frame_start").unwrap();
      let this_frame_start = this_frame.event_instants.get("frame_start").unwrap();
      next_frame_start.duration_since(*this_frame_start).subsec_nanos()
    } else {
      0
    }
  }

  pub fn to_csv(&self) -> String {
    let keys = vec![
      "frame_start",
      "pre_input",
      "post_input",
      "pre_update_camera",
      "post_update_camera",
      "pre_update_world",
      "post_update_world",
      "pre_sync_poses",
      "post_sync_poses",
      "pre_sync_frame_data",
      "post_sync_frame_data",
      "pre_draw",
      "post_draw",
      "frame_end",
    ];

    let mut log_csv = String::new();
    log_csv.push_str("Frame,FPS,");
    log_csv.push_str(&keys.join(","));
    log_csv.push_str(",Resolution,MSAA,LOD\n");

    for (i, frame) in self.log.iter().enumerate() {
      let frame_time = self.get_actual_frame_time(i);
      let fps = 1_000_000_000f64 / (frame_time as f64);
      write!(&mut log_csv, "{},{},", i, fps).unwrap();
      for key in &keys {
        let frame_start_instant = frame.event_instants.get("frame_start").unwrap();
        let event_instant = frame.event_instants.get(*key).unwrap();
        let duration = event_instant.duration_since(*frame_start_instant).subsec_nanos();
        write!(&mut log_csv, "{},", duration).unwrap();
      }
      write!(&mut log_csv, ",{},{},{}\n",
          frame.target_resolution,
          frame.target_msaa,
          frame.target_lod).unwrap();
    }
    log_csv
  }
}
