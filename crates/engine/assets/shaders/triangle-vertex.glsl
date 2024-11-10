#version 450 core

layout(location = 0) in vec2 vertex_position;
layout(location = 1) in vec3 vertex_color;

out vec3 v_vertex_color;

void main() {
    v_vertex_color = vertex_color;
    gl_Position = vec4(vertex_position, 0.0, 1.0);
}
