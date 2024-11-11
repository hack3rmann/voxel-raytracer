#version 450 core

layout(set = 0, binding = 0) uniform texture2D u_texture;
layout(set = 0, binding = 1) uniform sampler u_sampler;

in vec2 texture_coords;
out vec4 result_color;

void main() {
    result_color = texture(sampler2D(u_texture, u_sampler), texture_coords);
}
