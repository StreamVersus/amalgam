#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormals;
layout(location = 2) in vec2 inTexCoord;

layout(location = 0) out vec2 fragTexCoord;

layout(set = 0, binding = 0) uniform UBO {
    mat4 view;
    mat4 projection;
} vp;

void main() {
    gl_Position = vp.projection * vp.view * vec4(inPosition, 1.0);
    fragTexCoord = inTexCoord;
}