#version 450 core

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(std430, binding = 0) readonly buffer Voxels {
    uint colors[];
};

layout(rgba8, binding = 1) uniform image2D screen;

layout(push_constant) uniform uvec2 viewport_size;

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
    float px = origin.x;
    float py = origin.y;
    float pz = origin.z;

    float dx = direction.x;
    float dy = direction.y;
    float dz = direction.z;

    float t = 0.0;
    int ix = int(px);
    int iy = int(py);
    int iz = int(pz);

    int stepx = (dx > 0.0) ? 1 : -1;
    int stepy = (dy > 0.0) ? 1 : -1;
    int stepz = (dz > 0.0) ? 1 : -1;

    float infinity = uintBitsToFloat(0x7F800000);

    float tx_delta = (dx == 0.0) ? infinity : abs(1.0 / dx);
    float ty_delta = (dy == 0.0) ? infinity : abs(1.0 / dy);
    float tz_delta = (dz == 0.0) ? infinity : abs(1.0 / dz);

    float xdist = (stepx > 0) ? (ix + 1 - px) : (px - ix);
    float ydist = (stepy > 0) ? (iy + 1 - py) : (py - iy);
    float zdist = (stepz > 0) ? (iz + 1 - pz) : (pz - iz);

    float tx_max = (tx_delta < infinity) ? tx_delta * xdist : infinity;
    float ty_max = (ty_delta < infinity) ? ty_delta * ydist : infinity;
    float tz_max = (tz_delta < infinity) ? tz_delta * zdist : infinity;

    int stepped_index = -1;

    vec3 pos = origin;

    while (t <= max_distance) {
        vec4 color = get_voxel_color(ivec3(ix, iy, iz));
        pos = vec3(px + t * dx, py + t * dy, pz + t * dz);

        if (vec4(0.0) != color) {
            vec3 normal = vec3(0.0f);

            switch (stepped_index) {
                case 0:
                {
                    normal.x = -stepx;
                    break;
                }
                case 1:
                {
                    normal.y = -stepy;
                    break;
                }
                case 2:
                {
                    normal.z = -stepz;
                    break;
                }
                default:
                {
                    break;
                }
            }

            return RaytraceResult(
                color,
                pos,
                normal,
                true
            );
        }

        if (tx_max < ty_max) {
            if (tx_max < tz_max) {
                ix += stepx;
                t = tx_max;
                tx_max += tx_delta;
                stepped_index = 0;
            } else {
                iz += stepz;
                t = tz_max;
                tz_max += tz_delta;
                stepped_index = 2;
            }
        } else {
            if (ty_max < tz_max) {
                iy += stepy;
                t = ty_max;
                ty_max += ty_delta;
                stepped_index = 1;
            } else {
                iz += stepz;
                t = tz_max;
                tz_max += tz_delta;
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
    vec4 color = vec4(vec2(index) / 1024.0, 0.0, 1.0);

    float aspect_ratio = float(viewport_size.y) / float(viewport_size.x);
    vec2 screen_coord = 2.0 * vec2(index) / vec2(1024.0) - 1.0;

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

    if (result.has_hit) {
        vec3 to_light_direction = normalize(light_position - result.position);
        float brightness = max(0.0, dot(to_light_direction, result.normal));

        color = vec4(brightness * result.color, 1.0);
    } else {
        color = vec4(0.0);
    }

    imageStore(screen, index, color);
}
