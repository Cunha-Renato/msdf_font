mod app;
mod font_data;

fn main() {
    let el = winit::event_loop::EventLoop::new().unwrap();
    el.run_app(&mut crate::app::App::default()).unwrap();
}
