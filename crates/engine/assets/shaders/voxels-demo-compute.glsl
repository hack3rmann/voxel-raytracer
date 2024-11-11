#version 450 core

layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

layout(std430, binding = 0) readonly buffer Voxels {
    uint colors[];
};

layout(rgba8, binding = 1) uniform image2D screen;

layout(push_constant) uniform struct Config {
    uvec2 viewport_size;
    uvec2 render_texture_size;
} config;

const float PI = 3.1415926535;

vec4 get_voxel_color(ivec3 pos) {
    if (pos.x < -8 || 8 <= pos.x || pos.y < -8 || 8 <= pos.y || pos.z < -8 || 8 <= pos.z) {
        return vec4(0.0);
    }

    pos += ivec3(8, 8, 8);

    uint index = 16 * (16 * pos.z + pos.y) + pos.x;
    uint color_pack = colors[index];

    return vec4(
        float((color_pack >> 0) & 255) / 255.0,
        float((color_pack >> 8) & 255) / 255.0,
        float((color_pack >> 16) & 255) / 255.0,
        float((color_pack >> 24) & 255) / 255.0
    );
}

struct RaytraceResult {
    vec4 color;
    vec3 position;
    vec3 normal;
    bool has_hit;
};

RaytraceResult raytrace(vec3 origin, vec3 direction, float max_distance) {
    float t = 0.0;
    ivec3 int_pos = ivec3(origin);
    ivec3 step = ivec3(sign(direction));

    vec3 tdelta = abs(1.0 / direction);
    vec3 dist = ((step + 1) / 2) * (int_pos - origin + 1)
            + (1 - (step + 1) / 2) * (origin - int_pos);

    vec3 tmax = dist * tdelta;
    vec3 pos = origin;
    int stepped_index = -1;

    while (t <= max_distance) {
        vec4 color = get_voxel_color(int_pos);
        pos = origin + t * direction;

        if (vec4(0.0) != color) {
            vec3 mask = vec3(stepped_index == 0, stepped_index == 1, stepped_index == 2);
            vec3 normal = -mask * step;

            return RaytraceResult(color, pos, normal, true);
        }

        if (tmax.x < tmax.y) {
            if (tmax.x < tmax.z) {
                int_pos.x += step.x;
                t = tmax.x;
                tmax.x += tdelta.x;
                stepped_index = 0;
            } else {
                int_pos.z += step.z;
                t = tmax.z;
                tmax.z += tdelta.z;
                stepped_index = 2;
            }
        } else {
            if (tmax.y < tmax.z) {
                int_pos.y += step.y;
                t = tmax.y;
                tmax.y += tdelta.y;
                stepped_index = 1;
            } else {
                int_pos.z += step.z;
                t = tmax.z;
                tmax.z += tdelta.z;
                stepped_index = 2;
            }
        }
    }

    return RaytraceResult(vec4(0.0), pos, vec3(0.0), false);
}

vec3 spherical_to_cartesian(vec3 coords) {
    return coords.x * vec3(
            sin(coords.z) * sin(coords.y),
            cos(coords.z),
            sin(coords.z) * cos(coords.y)
        );
}

void main() {
    ivec2 index = ivec2(gl_GlobalInvocationID.xy);

    float aspect_ratio = float(config.viewport_size.y) / float(config.viewport_size.x);
    vec2 screen_coord = 2.0 * vec2(index) / vec2(config.render_texture_size - 1) - 1.0;

    float camera_vfov = PI / 3.0;
    vec3 camera_target_pos = vec3(0.0);
    // distance theta phi
    vec3 camera_spherical_coords = vec3(16.0, 0.3, 0.8);
    vec3 camera_pos = camera_target_pos + spherical_to_cartesian(camera_spherical_coords);

    vec3 camera_direction = normalize(camera_target_pos - camera_pos);
    vec3 camera_tangent = vec3(cos(camera_spherical_coords.y), 0.0, -sin(camera_spherical_coords.y));
    vec3 camera_bitangent = cross(camera_direction, camera_tangent);

    float fov_tan = tan(0.5 * camera_vfov);
    vec3 ray_direction = normalize(camera_direction
                + (screen_coord.x / aspect_ratio) * fov_tan * camera_tangent
                + screen_coord.y * fov_tan * camera_bitangent
        );
    vec3 ray_origin = camera_pos;

    RaytraceResult result = raytrace(ray_origin, ray_direction, 100.0);

    vec3 light_position = vec3(10.0, 12.0, 16.0);
    vec4 color;

    if (result.has_hit) {
        vec3 to_light_direction = normalize(light_position - result.position);
        float brightness = max(0.0, dot(to_light_direction, result.normal));

        color = vec4(brightness * result.color, 1.0);
    } else {
        color = vec4(0.0);
    }

    imageStore(screen, index, color);
}
