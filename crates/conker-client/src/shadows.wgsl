// bevy_pbr 0.19.1's `render/shadows.wgsl` without shadows, for the browser ([`crate::render`]'s
// `no_shadows`): no light of the game casts any, so every fetch is 1.0 (unshadowed), as Bevy's own
// gives for a light without shadow maps. Kept: what other modules import (volumetric fog, the cascade
// debug view). WebGL2 on ANGLE/D3D11 took 6-17 s to compile each PBR pipeline with Bevy's shadow
// sampling, about 1.5 s without.
#define_import_path bevy_pbr::shadows

#import bevy_pbr::mesh_view_bindings as view_bindings

#import bevy_render::{
    color_operations::hsv_to_rgb,
    maths::PI_2
}

fn fetch_point_shadow(
    light_id: u32,
    frag_position: vec4<f32>,
    surface_normal: vec3<f32>,
    frag_coord_xy: vec2<f32>,
) -> f32 {
    return 1.0;
}

fn fetch_spot_shadow(
    light_id: u32,
    frag_position: vec4<f32>,
    surface_normal: vec3<f32>,
    near_z: f32,
    frag_coord_xy: vec2<f32>,
) -> f32 {
    return 1.0;
}

fn get_cascade_index(light_id: u32, view_z: f32) -> u32 {
    let light = &view_bindings::lights.directional_lights[light_id];

    for (var i: u32 = 0u; i < (*light).num_cascades; i = i + 1u) {
        if (-view_z < (*light).cascades[i].far_bound) {
            return i;
        }
    }
    return (*light).num_cascades;
}

// Converts from world space to the uv position in the light's shadow map.
//
// The depth is stored in the return value's z coordinate. If the return value's
// w coordinate is 0.0, then we landed outside the shadow map entirely.
fn world_to_directional_light_local(
    light_id: u32,
    cascade_index: u32,
    offset_position: vec4<f32>
) -> vec4<f32> {
    let light = &view_bindings::lights.directional_lights[light_id];
    let cascade = &(*light).cascades[cascade_index];

    let offset_position_clip = (*cascade).clip_from_world * offset_position;
    if (offset_position_clip.w <= 0.0) {
        return vec4(0.0);
    }
    let offset_position_ndc = offset_position_clip.xyz / offset_position_clip.w;
    // No shadow outside the orthographic projection volume
    if (any(offset_position_ndc.xy < vec2<f32>(-1.0)) || offset_position_ndc.z < 0.0
            || any(offset_position_ndc > vec3<f32>(1.0))) {
        return vec4(0.0);
    }

    // compute texture coordinates for shadow lookup, compensating for the Y-flip difference
    // between the NDC and texture coordinates
    let flip_correction = vec2<f32>(0.5, -0.5);
    let light_local = offset_position_ndc.xy * flip_correction + vec2<f32>(0.5, 0.5);

    let depth = offset_position_ndc.z;

    return vec4(light_local, depth, 1.0);
}

fn fetch_directional_shadow(
    light_id: u32,
    frag_position: vec4<f32>,
    surface_normal: vec3<f32>,
    view_z: f32,
    frag_coord_xy: vec2<f32>,
) -> f32 {
    return 1.0;
}

fn cascade_debug_visualization(
    output_color: vec3<f32>,
    light_id: u32,
    view_z: f32,
) -> vec3<f32> {
    let overlay_alpha = 0.95;
    let cascade_index = get_cascade_index(light_id, view_z);
    let cascade_color_hsv = vec3(
        f32(cascade_index) / f32(#{MAX_CASCADES_PER_LIGHT}u + 1u) * PI_2,
        1.0,
        0.5
    );
    let cascade_color = hsv_to_rgb(cascade_color_hsv);
    return vec3<f32>(
        (1.0 - overlay_alpha) * output_color.rgb + overlay_alpha * cascade_color
    );
}
