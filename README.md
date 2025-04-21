# GPU Ray Tracing in Bevy

A real-time ray tracing implementation using Bevy's compute shaders, rendering directly on the GPU.

## Overview

This project demonstrates how to implement a basic ray tracer that runs entirely on the GPU using Bevy's compute shader system. It renders a 3D scene with spheres and sky background using the classic ray tracing algorithm, with all computation happening on the graphics card for maximum performance.

## Features

- Real-time ray tracing on the GPU
- Sphere intersection and shading
- Sky gradient background
- Camera movement with proper view transformation
- Fully shader-based rendering pipeline
- Integration with Bevy engine

## Screenshots

*[Screenshots would be placed here]*

## Requirements

- Rust (stable channel)
- A GPU that supports compute shaders
- Cargo package manager

## Getting Started

### Installation

1. Clone the repository:
   ```
   git clone https://github.com/yourusername/gpu_ray_tracing.git
   cd gpu_ray_tracing
   ```

2. Build and run:
   ```
   cargo run --release
   ```

## How It Works

The ray tracer works by:

1. Setting up a compute shader pipeline that renders to a texture
2. For each pixel in the output image:
   - Calculating a ray from the camera through that pixel
   - Testing for intersections with spheres in the scene
   - Determining the color based on hit normals or sky gradient
   - Writing the resulting color to the output texture

The main components include:

- `compute_shader.rs`: Sets up the compute shader pipeline and handles GPU resource binding
- `camera.rs`: Defines the camera structure that's passed to the shader
- `scene.rs`: Manages scene objects and camera extraction
- `compute_shader_example.wgsl`: The shader that performs the actual ray tracing

## Controls

*[If there are any controls for moving the camera or interacting with the scene, list them here]*

## Implementation Details

### Ray Tracing Algorithm

The implementation follows the ray tracing approach described in "Ray Tracing in One Weekend" by Peter Shirley:
- Rays are cast from the camera through each pixel
- Sphere intersection tests determine what objects are visible
- Surface normals provide simple shading
- A sky gradient appears when rays miss all objects

### GPU Acceleration

Rather than tracing rays on the CPU, this project:
- Uses WGSL compute shaders to trace rays in parallel
- Exploits GPU cores for massive parallelization
- Renders directly to a texture that's displayed on screen

## Future Improvements

- Materials and textures
- Reflections and refractions
- Soft shadows
- Multiple light sources
- Anti-aliasing
- More primitive shapes

## License

*[Add your license information here]*

## Acknowledgments

- Inspired by "Ray Tracing in One Weekend" by Peter Shirley
- Built with the [Bevy](https://bevyengine.org/) game engine

---

*This project is for educational purposes to demonstrate GPU ray tracing techniques.*


