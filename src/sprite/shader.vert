#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec3 color;


layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec2 uv;


layout(binding = 0) uniform Transforms{
	mat4 world_to_screen;
};

 

layout(push_constant) uniform constants {
    vec2 world_pos;

	uint tile_x;
	uint tile_y;

	vec2 tile_uv_size; 
	vec2 tile_scale;
};


void main() {
	fragColor = color;
  

	uv = tile_uv_size * vec2(tile_x, tile_y) + vec2(position.x, 1 - position.y) * tile_uv_size;

    gl_Position = vec4( world_pos + position * tile_scale + vec2(tile_x, tile_y) , 0.0, 1.0) * world_to_screen;
}