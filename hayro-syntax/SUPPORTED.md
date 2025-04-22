Below you can find the feature of the PDF specification that are supported by `hayro-syntax`. For this crate, only
parts of the chapter `Syntax` are relevant.

# Filters
We do not support the `FFilter` and `FDecodeParams` attributes.

- ASCIIHexDecode is supported. 🟢
- ASCII85Decode is supported. 🟢
- LZWDecode/FlateDecode
  - We support the baseline of those filters.
  - We do not support `Predictor` and associated parameters.
- RunLengthDecode is supported. 🟢
- DCTDecode
  - We support it in principle.
  - However, we do not support the `ColorTransform` parameter.
- CCITTFaxDecode is not supported. 🔴
- JBIG2Decode is not supported. 🔴
- JPXDecode is not supported. 🔴
- Crypt is not supported. 🔴