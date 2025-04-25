Below you can find the feature of the PDF specification that are supported by `hayro-syntax`. For this crate, only
parts of the chapter `Syntax` are relevant.

# Objects
- We support reading and storing all types of primitive PDF objects. 🟢

# Filters
We do not support the `FFilter` and `FDecodeParams` attributes. 🔴

- ASCIIHexDecode is supported. 🟢
- ASCII85Decode is supported. 🟢
- LZWDecode/FlateDecode
  - We support those filters as well as PNG predictors.
  - We do not support the TIFF `Predictor`. 🔴
  - We do not support predictors with bits per component != 8. 🔴
- RunLengthDecode is supported. 🟢
- DCTDecode
  - We support it in principle.  
  - However, we do not support the `ColorTransform` parameter. 🔴
- CCITTFaxDecode is not supported. 🔴
- JBIG2Decode is not supported. 🔴
- JPXDecode is not supported. 🔴
- Crypt is not supported. 🔴

# File structure
In general, we support most of the requirements mentioned there. 🟢

- We do not preserve incremental updates, though, and instead only care about the latest version. 🔴
- We currently do not read the version of the PDF document. 🔴

# Encryption
We do not support encryption.

# Document structure
We do not support most of the document structure.