use std::collections::HashMap;
use log::warn;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BASE_ENCODING, BASE_FONT, DIFFERENCES, ENCODING, FIRST_CHAR, FONT_DESCRIPTOR, MISSING_WIDTH, WIDTHS};
use hayro_syntax::object::name::Name;
use hayro_syntax::object::Object;
use crate::font::blob::FontBlob;
use crate::font::Encoding;
use crate::font::standard::{select, StandardFont};
use crate::font::type1::{Type1Font};

#[derive(Debug)]
pub(crate) struct TrueTypeFont {
    base_font: Option<StandardFont>,
    widths: HashMap<u8, f32>,
    blob: FontBlob,
    encoding: Encoding,
}


impl TrueTypeFont {
    pub fn new(dict: &Dict) -> TrueTypeFont {
        let widths = read_widths(dict);
        // let encoding = read_encoding()
        todo!();
        // match dict.get::<Name>(BASE_FONT)
        //     .and_then(|b| select(b)) {
        //     Some(f)
        // }
        // 
        // let base_font = dict.get::<Name>(BASE_FONT)
        //     .and_then(|b| select(b)).unwrap();
        // let blob = base_font.get_blob();
        // 
        // let mut encoding_map = HashMap::new();
        // let encoding = read_encoding(dict, &mut encoding_map);
        // 
        // Self {
        //     base_font: Some(base_font),
        //     encodings: encoding_map,
        //     encoding,
        //     blob,
        // }
    }
}

fn read_widths(dict: &Dict) -> HashMap<u8, f32> {
    let mut widths = HashMap::new();
    
    let first_char = dict.get::<u8>(FIRST_CHAR);
    let last_char = dict.get::<u8>(FIRST_CHAR);
    let widths_arr = dict.get::<Array>(WIDTHS);
    let missing_width = dict
        .get::<Dict>(FONT_DESCRIPTOR)
        .and_then(|d| d.get::<f32>(MISSING_WIDTH))
        .unwrap_or(0.0);
    
    match (first_char, last_char, widths_arr) {
        (Some(fc), Some(_), Some(w)) => {
            let mut iter = w.iter::<f32>();
            let mut idx = 0;
            
            while idx < fc {
                widths.insert(idx, missing_width);
                idx += 1;
            }
            
            while let Some(w) = iter.next() {
                widths.insert(idx, w);
                idx += 1;
            }
            
            while idx <= u8::MAX {
                widths.insert(idx, missing_width);
                idx += 1;
            }
        }
        _ => {}
    }
    
    widths
}

pub(crate) fn read_encoding(dict: &Dict) -> (Encoding, HashMap<u8, String>) {
    fn get_encoding_base(dict: &Dict, name: Name) -> Encoding {
        match dict.get::<Name>(name) {
            Some(n) => match n.get().as_ref() {
                b"WinAnsiEncoding" => Encoding::WinAnsi,
                b"MacRomanEncoding" => Encoding::MacRoman,
                b"MacExpertEncoding" => Encoding::MacExpert,
                _ => {
                    warn!("Unknown font encoding {}", name.as_str());
                    
                    Encoding::Standard
                },
            },
            None => Encoding::BuiltIn,
        }
    }
    
    let mut map = HashMap::new();

    if let Some(encoding_dict) = dict.get::<Dict>(ENCODING) {
        // Note that those only exist for Type1 fonts, not for TrueType fonts.
        if let Some(differences) = encoding_dict.get::<Array>(DIFFERENCES) {
            let mut entries = differences.iter::<Object>();

            let mut code = 0;

            while let Some(obj) = entries.next() {
                if let Ok(num) = obj.clone().cast::<i32>() {
                    code = num;
                } else if let Ok(name) = obj.cast::<Name>() {
                    map.insert(code as u8, name.as_str());
                    code += 1;
                }
            }
        }

        (get_encoding_base(&encoding_dict, BASE_ENCODING), map)
    } else {
        (get_encoding_base(&dict, ENCODING), HashMap::new())
    }

}