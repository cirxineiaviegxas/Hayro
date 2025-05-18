import pathlib

ASSETS_DIR = pathlib.Path(__file__).parent.parent.parent.parent.parent / "assets"
GLYPH_LIST = ASSETS_DIR / "glyphlist" / "glyphlist.txt"
ZAPF_DINGS_BATS = ASSETS_DIR / "glyphlist" / "zapfdingbats.txt"
ADDITIONAL = ASSETS_DIR / "glyphlist" / "additional.txt"
GLYPH_LIST_RS = pathlib.Path(__file__).parent / "glyph_names.rs"
STANDARD_RS = pathlib.Path(__file__).parent / "standard.rs"
SYMBOL_RS = pathlib.Path(__file__).parent / "symbol.rs"
ZAPF_DINGS_RS = pathlib.Path(__file__).parent / "zapf_dings.rs"
METRICS_RS = pathlib.Path(__file__).parent / "metrics.rs"

def generate_glyph_list():
    start = """// THIS FILE WAS AUTO-GENERATED, DO NOT EDIT MANUALLY!
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

    with open(GLYPH_LIST_RS, 'w') as file:
        file.write(start)

def generate_encodings():    
    for (font, file, out) in [
        # Is the same for Helvetica and Courier as well.
        ("STANDARD", "Times-Roman", STANDARD_RS),
        ("SYMBOL", "Symbol", SYMBOL_RS),
        ("ZAPF_DING_BATS", "ZapfDingbats", ZAPF_DINGS_RS),
    ]:

        start = """// THIS FILE WAS AUTO-GENERATED, DO NOT EDIT MANUALLY!
use phf::phf_map;"""

        with open(ASSETS_DIR / "font_metrics" / f"{file}.afm") as file:

            start += f"""\n
pub(crate) static {font}: phf::Map<u8, &'static str> = phf_map! {{
"""
            lines = [l for l in file.read().splitlines() if l.startswith("C ")]
            
            for line in lines:
                split = line.split(";")
                temp = split[0].split(" ")
                code = int(temp[1])
                temp = split[2].split(" ")
                name = temp[2]
                
                if code != -1:
                    start += f"    {code}u8 => \"{name}\",\n"

            start += "};"
        
        with open(out, 'w') as file:
            file.write(start)

def generate_metrics():
    fonts = [
        ("COURIER", "Courier"),
        ("COURIER_BOLD", "Courier-Bold"),
        ("COURIER_BOLD_OBLIQUE", "Courier-BoldOblique"),
        ("COURIER_OBLIQUE", "Courier-Oblique"),
        ("HELVETICA", "Helvetica"),
        ("HELVETICA_BOLD", "Helvetica-Bold"),
        ("HELVETICA_BOLD_OBLIQUE", "Helvetica-BoldOblique"),
        ("HELVETICA_OBLIQUE", "Helvetica-Oblique"),
        ("SYMBOL", "Symbol"),
        ("TIMES_BOLD", "Times-Bold"),
        ("TIMES_BOLD_ITALIC", "Times-BoldItalic"),
        ("TIMES_ITALIC", "Times-Italic"),
        ("TIMES_ROMAN", "Times-Roman"),
        ("ZAPF_DING_BATS", "ZapfDingbats"),
    ]

    start = """// THIS FILE WAS AUTO-GENERATED, DO NOT EDIT MANUALLY!
use phf::phf_map;"""

    for (font, file) in fonts:
        with open(ASSETS_DIR / "font_metrics" / f"{file}.afm") as file:
            start += f"""\n
pub(crate) static {font}: phf::Map<&'static str, f32> = phf_map! {{
"""

            lines = [l for l in file.read().splitlines() if l.startswith("C ")]
    
            for line in lines:
                split = line.split(";")
                temp = split[1].split(" ")
                width = temp[2]
                temp = split[2].split(" ")
                name = temp[2]
    
                start += f"    \"{name}\" => {width}f32,\n"
    
            start += "};"

    with open(METRICS_RS, 'w') as file:
        file.write(start)

generate_glyph_list()
generate_encodings()
generate_metrics()