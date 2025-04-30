// See <https://github.com/apache/pdfbox/blob/4438b8fdc67a3a9ebfb194595d0e81f88b708a37/pdfbox/src/main/java/org/apache/pdfbox/pdmodel/font/encoding/MacOSRomanEncoding.java.java>
use phf::phf_map;

pub(crate) static MAC_OS_ROMAN: phf::Map<u8, &'static str> = phf_map! {
    0o0255u8 => "notequal",
    0o0260u8 => "infinity",
    0o0262u8 => "lessequal",
    0o0263u8 => "greaterequal",
    0o0266u8 => "partialdiff",
    0o0267u8 => "summation",
    0o0270u8 => "product",
    0o0271u8 => "pi",
    0o0272u8 => "integral",
    0o0275u8 => "Omega",
    0o0303u8 => "radical",
    0o0305u8 => "approxequal",
    0o0306u8 => "Delta",
    0o0327u8 => "lozenge",
    0o0333u8 => "Euro",
    0o0360u8 => "apple"
};
