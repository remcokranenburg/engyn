             extern crate csv;
#[macro_use] extern crate serde_derive;

use csv::Reader;
use csv::Writer;
use std::collections::HashMap;
use std::env;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct LogEntry {
    AnalysisTarget: String,
    Frame: u32,
    Sample: u32,
    Dropped: u32,
    TimeStart: u32,
    TimeEnd: u32,
    frame_start: u32,
    pre_input: u32,
    post_input: u32,
    pre_update_camera: u32,
    post_update_camera: u32,
    pre_update_world: u32,
    post_update_world: u32,
    pre_sync_poses: u32,
    post_sync_poses: u32,
    pre_sync_frame_data: u32,
    post_sync_frame_data: u32,
    pre_draw: u32,
    post_draw: u32,
    frame_end: u32,
    Level: String,
    WeightResolution: String,
    WeightMSAA: String,
    WeightLOD: String,
    TargetResolution: String,
    TargetMSAA: String,
    TargetLOD: String,
    TargetFrameTime: u64,
    PredictedRemainingTime: u64,
    RatioRemaining: String,
}

fn main() {
    let filename = env::args().nth(1).expect("Could not parse command line arguments");
    let mut rdr = Reader::from_path(filename).unwrap();
    let mut logs = HashMap::new();

    for result in rdr.deserialize() {
        let r: LogEntry = result.expect("Could not read record");
        match logs.get_mut((r.AnalysisTarget, r.Frame, r.WeightResolution)) {
            Some(entry) => {
                *entry.Dropped += r.Dropped;
                entry.Level += r.Level;
                entry.TargetFrameTime += r.TargetFrameTime;
                entry.PredictedRemainingTime += r.PredictedRemainingTime;
            },
            None => logs.insert(
                (r.AnalysisTarget, r.Frame, r.WeightResolution),
                r
            ),
        }
    }

    for (_, value) in logs.iter_mut() {
        (*value).Level /= 10.0;
        (*value).RatioRemaining = (*value).TargetFrameTime as f32 / (*value).PredictedRemainingTime as f32;
        (*value).TargetFrameTime /= 10;
        (*value).PredictedRemainingTime /= 10;
    }

    let mut wtr = Writer::from_path(filename + ".mergesamples.csv").unwrap();

    for(_, value) in logs.iter() {
        wtr.serialize(value).expect("Could not serialize record");
    }
 }
