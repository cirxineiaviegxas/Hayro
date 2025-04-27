import pathlib

ASSETS_DIR = pathlib.Path(__file__).parent.parent.parent.parent / "assets"
GLYPH_LIST = ASSETS_DIR / "glyphlist" / "glyphlist.txt"
ADDITIONAL = ASSETS_DIR / "glyphlist" / "additional.txt"
GLYPH_LIST_RS = pathlib.Path(__file__).parent / "glyph_list.rs"

print(ASSETS_DIR)

def generate_glyph_list():
    start = """
use phf::phf_map;

static GLYPH_NAME_MAP: phf::Map<&'static str, &'static str> = phf_map! {
"""

    with open(GLYPH_LIST) as file1, open(ADDITIONAL) as file2:
        lines = file1.read().splitlines() + file2.read().splitlines()
        for line in lines:
            if not line.startswith("#"):
                split = line.split(";")
                codepoints = "".join([f"\\u{{{c}}}" for c in split[1].split(" ")])
                start += f"    \"{split[0]}\" => \"{codepoints}\",\n"
    
    start += "};"
    
    with open(GLYPH_LIST_RS, 'w') as file:
        file.write(start)



generate_glyph_list()