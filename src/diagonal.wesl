// Diagonal split: P2's view below the top-left to bottom-right line, a divider on the line.
import bevy_ui_render::ui_vertex_output::UiVertexOutput;

@group(1) @binding(0)
var p2: texture_2d<f32>;
@group(1) @binding(1)
var p2_sampler: sampler;

/// Divider half-width in pixels at 800 px of window height, and its colour (cream).
const HALF_WIDTH: f32 = 2.0;
const LINE: vec4<f32> = vec4<f32>(0.98, 0.93, 0.82, 1.0);

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let p = in.uv * in.size;
    // Signed pixel distance to the diagonal, positive below it (P2's side)
    let d = (p.y * in.size.x - p.x * in.size.y) / length(in.size);
    let half_width = HALF_WIDTH * max(in.size.y / 800.0, 0.75);
    // Anti-aliased edge of the line
    let line = 1.0 - smoothstep(half_width - 0.75, half_width + 0.75, abs(d));
    let view = textureSample(p2, p2_sampler, in.uv);
    let below = step(0.0, d);
    return mix(vec4<f32>(view.rgb, below), LINE, line);
}
