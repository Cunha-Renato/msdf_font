use std::time::Instant;

use crate::font_data::FontData;

mod app;
mod font_data;

fn run_app() {
    let el = winit::event_loop::EventLoop::new().unwrap();
    el.run_app(&mut crate::app::App::default()).unwrap();
}

fn test_time_generation() {
    const SAMPLES: usize = 10;

    let mut dur_sum = 0;

    for _ in 0..SAMPLES {
        let now = Instant::now();
        let (_, _) = FontData::new("CascadiaCode-Medium.ttf").unwrap();
        dur_sum += now.elapsed().as_millis() as usize;
    }

    println!("Avg duration: {}ms", dur_sum / SAMPLES);
}

fn main() {
    // test_time_generation();
    run_app();
}
