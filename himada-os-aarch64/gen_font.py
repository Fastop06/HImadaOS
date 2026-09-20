import re

with open('font8x8_basic.h', 'r') as f:
    data = f.read()

match = re.search(r'char font8x8_basic\[128\]\[8\] = \{(.*?)\};', data, re.DOTALL)
if match:
    content = match.group(1)
    with open('src/font.rs', 'w') as f:
        f.write("pub const FONT8X8: [[u8; 8]; 128] = [\n")
        lines = content.split('\n')
        for line in lines:
            line = line.replace('{', '[').replace('}', ']')
            f.write(line + "\n")
        f.write("];\n")
    print("font.rs generated successfully.")
else:
    print("Failed to parse font.")
