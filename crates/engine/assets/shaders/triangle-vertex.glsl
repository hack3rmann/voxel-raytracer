#version 450 core

layout(location = 0) in vec2 vertex_position;
layout(location = 1) in vec3 vertex_color;

layout(push_constant) uniform PushConstant {
    uvec2 viewport_size;
} push_constant;

out vec3 v_vertex_color;

void main() {
    float aspect_ratio = float(push_constant.viewport_size.y) / float(push_constant.viewport_size.x);
    vec2 position = vec2(vertex_position.x * aspect_ratio, vertex_position.y);

    v_vertex_color = vertex_color;
    gl_Position = vec4(position, 0.0, 1.0);
}
