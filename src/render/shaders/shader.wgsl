struct RenderSettings {
    view_proj: mat4x4<f32>,

    height_scale: f32,
    tex_size: u32
};

@group(0) @binding(0)
var<uniform> settings: RenderSettings;

@group(1) @binding(0)
var t_noise: texture_2d<f32>;
@group(1) @binding(1)
var s_noise: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn uv_to_i(uv: f32) -> u32 {
    var res = u32(uv * f32(settings.tex_size));

    if (res < 0u) {
        return 0u;
    } else if (res > settings.tex_size) {
        return settings.tex_size - 1u;
    }

    return res;
}

@vertex
fn vs_main(
    model: VertexInput
) -> VertexOutput {
    var out: VertexOutput;

    var tex_x = uv_to_i(model.uv.x);
    var tex_y = uv_to_i(model.uv.y);

    var raw_height = textureLoad(t_noise, vec2<u32>(tex_x, tex_y), 0).x;
    var height = raw_height * settings.height_scale;

    out.clip_position = settings.view_proj * vec4<f32>(model.position.x, height, model.position.y, 1.0);
    out.uv = model.uv;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(t_noise, s_noise, in.uv).x, 0.0, 0.0, 1.0);
}