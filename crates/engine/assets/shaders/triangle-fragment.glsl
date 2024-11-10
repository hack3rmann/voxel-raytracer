#version 450 core

in vec3 v_vertex_color;
out vec4 result_color;

void main() {
    result_color = vec4(v_vertex_color, 1.0);
}
