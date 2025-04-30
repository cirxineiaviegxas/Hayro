use std::collections::HashMap;
use std::sync::Arc;
use log::warn;
use hayro_syntax::object::array::Array;
use hayro_syntax::object::dict::Dict;
use hayro_syntax::object::dict::keys::{BASE_ENCODING, BASE_FONT, DIFFERENCES, ENCODING, FIRST_CHAR, FONT_DESCRIPTOR, FONT_FILE2, MISSING_WIDTH, WIDTHS};
use hayro_syntax::object::name::Name;
use hayro_syntax::object::Object;
use hayro_syntax::object::stream::Stream;
use crate::font::blob::FontBlob;
use crate::font::Encoding;
use crate::font::standard::{select_standard_font, StandardFont};
use crate::font::type1::{Type1Font};

#[derive(Debug)]
enum InnerFont {
    Standard(StandardFont),
    Custom(FontBlob)
}

#[derive(Debug)]
pub(crate) struct TrueTypeFont {
    base_font: InnerFont,
    widths: HashMap<u8, f32>,
    encoding: Encoding,
}

impl TrueTypeFont {
    pub fn new(dict: &Dict) -> TrueTypeFont {
        let descriptor = dict
            .get::<Dict>(FONT_DESCRIPTOR).unwrap();
        
        let widths = read_widths(dict, &descriptor);
        let (encoding, _) = read_encoding(dict);
        let base_font = select_standard_font(dict)
            .map(|d| InnerFont::Standard(d))
            .or_else(|| 
                descriptor.get::<Stream>(FONT_FILE2)
                    .and_then(|s| s.decoded().ok())
                    .map(|d| InnerFont::Custom(FontBlob::new(Arc::new(d.to_vec()), 0)))
            )
            .unwrap_or_else(|| {
                warn!("failed to extract base font. falling back to Times New Roman.");
                
                InnerFont::Standard(StandardFont::TimesRoman)
            });
        
        
        Self {
            base_font,
            widths,
            encoding,
        }
    }
}

fn read_widths(dict: &Dict, descriptor: &Dict) -> HashMap<u8, f32> {
    let mut widths = HashMap::new();
    
    let first_char = dict.get::<u8>(FIRST_CHAR);
    let last_char = dict.get::<u8>(FIRST_CHAR);
    let widths_arr = dict.get::<Array>(WIDTHS);
    let missing_width = descriptor.get::<f32>(MISSING_WIDTH)
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