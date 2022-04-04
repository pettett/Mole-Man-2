#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec3 color;


layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec2 uv;


layout(binding = 0) uniform Transforms{
	mat4 world_to_screen;
};

struct Tile{
	uint sheet_pos;
	uint grid_pos;
};

layout(binding = 1 ) buffer TilemapData {
    vec2 tile_size; 
	uint grid_width;
	uint sheet_width;
    Tile tiles[]; 
};


void main() {
	fragColor = color;

	Tile tile = tiles[gl_InstanceIndex];
	uint sheet_x = tile.sheet_pos % sheet_width;
	uint sheet_y = tile.sheet_pos / sheet_width;

	
	uv = tile_size * vec2(sheet_x, sheet_y) + vec2(position.x, 1 - position.y) * tile_size;

	uint grid_x = tile.grid_pos % grid_width;
	uint grid_y = tile.grid_pos / grid_width;

    gl_Position = vec4(position + vec2(grid_x, grid_y) , 0.0, 1.0) * world_to_screen;
}