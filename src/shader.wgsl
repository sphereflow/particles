struct VertexOutput {
    @builtin(position) out_pos: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

struct Transform {
    transform: mat4x4<f32>,
};

@group(0)
@binding(0)
var<uniform> u_transform: Transform;

@vertex
fn vs_main(
        @location(0) in_pos: vec3<f32>,
        @location(1) tex_coord: vec2<f32>,
        @location(2) instance_pos: vec4<f32>,
        @location(3) particle_type: u32,
        ) -> VertexOutput {
    var out: VertexOutput;
    out.out_pos = u_transform.transform * vec4<f32>(in_pos + instance_pos.xyz, 1.0);
    out.tex_coord = vec2<f32>((tex_coord.x + f32(particle_type)) * 0.2, tex_coord.y);
    return out;
}

@group(1)@binding(0)
var texture: texture_2d<f32>;
@group(1)@binding(1)
var t_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(texture, t_sampler, in.tex_coord);
    return tex;
}
