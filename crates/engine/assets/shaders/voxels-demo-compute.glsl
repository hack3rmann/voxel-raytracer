#version 450 core

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(std430, binding = 0) readonly buffer Voxels {
    uint colors[];
};

layout(rgba8, binding = 1) uniform image2D screen;

void main() {}
