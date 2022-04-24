#version 450


layout(location = 0) in vec3 color;
layout(location = 1) in vec2 uv;

layout(location = 0) out vec4 f_color;


layout(binding = 3) uniform sampler2D texSampler;


void main() {
    f_color =   texture(texSampler, uv);
}