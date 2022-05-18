use std::time::{Duration, Instant};

use bevy_ecs::change_detection::ResMut;
use log::info;

#[derive(Debug)]
pub struct FrameCounter {
    last_print: Instant,
    frames: usize,
}

impl Default for FrameCounter {
    fn default() -> Self {
        Self {
            last_print: Instant::now(),
            frames: 0,
        }
    }
}

pub(crate) fn frame_counter(mut frame_counter: ResMut<FrameCounter>) {
    frame_counter.frames += 1;

    let now = Instant::now();
    let delta = now.duration_since(frame_counter.last_print);
    if delta >= Duration::from_secs(1) {
        let fps = frame_counter.frames as f32 / delta.as_secs_f32();
        info!("FPS: {}", fps);
        frame_counter.last_print = now;
        frame_counter.frames = 0;
    }
}
