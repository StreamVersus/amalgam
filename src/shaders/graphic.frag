#version 450
layout(location = 0) in vec2 inTexCoord;
layout(location = 0) out vec4 outColor;

layout(set = 1, binding = 0) uniform sampler2D textures[];

void main() {
    vec4 textureColor = texture(textures[0], inTexCoord);
    outColor = textureColor;
}