#version 450 core

layout(location = 0) in vec2 vertex_position;

out vec2 texture_coords;

void main() {
    texture_coords = vertex_position;
    gl_Position = vec4(vertex_position, 0.0, 1.0);
}
