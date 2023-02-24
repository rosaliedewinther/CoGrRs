pub struct SliderData<T> {
    pub min: T,
    pub max: T,
    pub current: T,
}
pub struct MetricData {
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
