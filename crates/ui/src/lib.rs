use gpu::auto_encoder::AutoEncoder;
use gpu::Context;
use winit::event::Event;
use winit::window::Window;

pub mod imgui;

struct SliderData<T> {
    min: T,
    max: T,
    current: T,
}
struct MetricData {
    pub values: std::vec::Vec<f32>,
    pub current_index: usize,
    pub max_index: usize,
    pub min_index: usize,
    pub handled_indices: i32,
    pub rolling_average: f32,
}
impl MetricData {
    pub fn new(size: usize) -> Self {
        MetricData {
            values: vec![0f32; size],
            current_index: 0,
            max_index: 0,
            min_index: 0,
            handled_indices: 0,
            rolling_average: 0f32,
        }
    }
}

pub trait ComboBoxable: Copy {
    fn get_names() -> &'static [&'static str];
    fn get_variant(index: usize) -> Self;
}

pub trait UI {
    fn new(gpu_context: &Context, window: &Window) -> Self;
    fn draw(&mut self, gpu_context: &mut AutoEncoder, window: &Window);
    fn handle_event(event: Event<()>);
    fn slider(&mut self, name: &str, min_value: f32, max_val: f32, value: &mut f32);
    fn slideri(&mut self, name: &str, min_value: i32, max_val: i32, value: &mut i32);
    fn toggle(&mut self, name: &str, state: &mut bool);
    fn text(&mut self, name: &str, text: &str);
    fn combobox<Enum: ComboBoxable>(&mut self, name: &str, item: &mut Enum);
    fn metric(&mut self, name: &str, size: u32, value: f32);
}
