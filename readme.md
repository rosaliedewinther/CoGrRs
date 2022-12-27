# CoGrRs (Compute Graphics in Rust)

CoRrRs is an easy-to-use but performant library for writing renderers using compute shaders in rust. The library makes use of [WGPU](https://github.com/gfx-rs/wgpu) with the vulkan backend. The examples can be ran using the below code:

```console
cargo run --example hello_world
cargo run --example hello_sine
cargo run --example ray_tracer --release
```