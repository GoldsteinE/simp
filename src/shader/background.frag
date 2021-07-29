#version 140

out vec4 color;
uniform vec2 size;

const float top_offset = 25.0;

void main() {
	vec4 color1 = vec4(0.03, 0.03, 0.03, 0.5);
	vec4 color2 = vec4(0.02, 0.02, 0.02, 0.5);

	float checkSize = 10.0;
	float x = floor(gl_FragCoord[0] / checkSize);
	float y = floor((gl_FragCoord[1] - size.y + top_offset) / checkSize);
	
	if(mod(x + y, 2) == 0) {
		color = color1;
	} else {
		color = color2;
	}
}