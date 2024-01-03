struct VertexOutput {
    @builtin(position) out_pos: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct Transform {
    transform: mat4x4<f32>,
};

@group(0)
@binding(0)
var<uniform> u_transform: Transform;

fn rotation_between(a: vec3<f32>, b: vec3<f32>) -> mat3x3<f32> {
    let an = normalize(a);
    let bn = normalize(b);
    let v = cross(an, bn);
    // cosine
    let c = dot(an, bn);
    let omc = 1.0 - c;
    // sine
    let s = 1.0 - c * c;
    let rot = mat3x3<f32>(  vec3<f32>(c + v.x * v.x * omc, v.x * v.y * omc + v.z * s, v.z * v.x * omc - v.y * s),
                            vec3<f32>(v.x * v.y * omc - v.z * s, c + v.y * v.y * omc, v.z * v.y * omc + v.x * s),
                            vec3<f32>(v.x * v.z * omc + v.y * s, v.y * v.z * omc - v.x * s, c + v.z * v.z * omc));
    return rot;
}

fn rotation_from_010_to(up: vec3<f32>) -> mat3x3<f32> {
    let upn = normalize(up);
    let right = cross(vec3<f32>(0.0, 1.0, 0.0), up);
    let forward = cross(right, upn);
    return mat3x3<f32>(right, upn, forward);
}

@vertex
fn vs_main(
        @location(0) in_pos: vec3<f32>,
        @location(1) tex_coord: vec2<f32>,
        @location(2) arrow_pos: vec4<f32>,
        @location(3) arrow_dir: vec4<f32>,
        @location(4) color: vec4<f32>,
        ) -> VertexOutput {
    var out: VertexOutput;
    // var rot = rotation_between(vec3<f32>(0.0, 1.0, 0.0), arrow_dir.xyz);
    var rot = rotation_from_010_to(arrow_dir.xyz);
    // handle rotation case where a = 0, 1, 0 and b = 0, -1, 0
    if (arrow_dir.x == 0.0) && (arrow_dir.z == 0.0) && (arrow_dir.y < 0.0) {
        // this works despite matrices being column major because this matrix is symmetric
        rot = mat3x3<f32>(  1.0, 0.0, 0.0,
                            0.0, -1.0, 0.0,
                            0.0, 0.0, -1.0);
    }
    let arrow_len = length(arrow_dir) * 0.1;
    let in_pos_alen = vec3<f32>(in_pos.x * 0.3, in_pos.y * arrow_len, in_pos.z * 0.3);

    out.out_pos = u_transform.transform * vec4<f32>(rot * in_pos_alen + arrow_pos.xyz , 1.0);
    out.tex_coord = tex_coord;
    out.color = color;
    return out;
}

@group(1)@binding(0)
var texture: texture_2d<f32>;
@group(1)@binding(1)
var t_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = in.color * textureSample(texture, t_sampler, in.tex_coord);
    return tex;
}
