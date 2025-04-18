struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: mat4x4<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera * vec4<f32>(input.pos, 1.0);
    out.uv = input.uv;
    out.color = input.color;
    return out;
}

@group(0) @binding(1)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(2)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(t_diffuse, s_diffuse, in.uv);
    let d = tex.r;

    let aaf = fwidthFine(d);

    let alpha = smoothstep(0.5 - aaf, 0.5 + aaf, d);

    return vec4<f32>(in.color.xyz, alpha);
    //return vec4<f32>(0.0, tex.r, 0.0, 1.0);
}
