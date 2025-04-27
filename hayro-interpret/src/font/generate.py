import pathlib

ASSETS_DIR = pathlib.Path(__file__).parent.parent.parent.parent / "assets"
GLYPH_LIST = ASSETS_DIR / "glyphlist" / "glyphlist.txt"
ZAPF_DINGS_BATS = ASSETS_DIR / "glyphlist" / "zapfdingbats.txt"
ADDITIONAL = ASSETS_DIR / "glyphlist" / "additional.txt"
GLYPH_LIST_RS = pathlib.Path(__file__).parent / "glyph_list.rs"

print(ASSETS_DIR)

def generate_glyph_list():
    start = """
use phf::phf_map;

pub(crate) static GLYPH_NAMES: phf::Map<&'static str, &'static str> = phf_map! {
"""

    def process_lines(lines, current):
        for line in lines:
            if not line.startswith("#"):
                split = line.split(";")
                codepoints = "".join([f"\\u{{{c}}}" for c in split[1].split(" ")])
                current += f"    \"{split[0]}\" => \"{codepoints}\",\n"
        return current

    with open(GLYPH_LIST) as file1, open(ADDITIONAL) as file2:
        lines = file1.read().splitlines() + file2.read().splitlines()
        start = process_lines(lines, start)
    
    start += "};"
        
    start += """\n\n
pub(crate) static ZAPF_DINGS: phf::Map<&'static str, &'static str> = phf_map! {
"""

    with open(ZAPF_DINGS_BATS) as file:
        lines = file.read().splitlines()
        start = process_lines(lines, start)

    start += "};"

    with open(GLYPH_LIST_RS, 'w') as file:
        file.write(start)


generate_glyph_list()