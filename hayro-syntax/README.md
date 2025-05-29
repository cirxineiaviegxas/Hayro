# hayro-syntax

<!-- cargo-rdme start -->

A low-level library for reading PDF files.

This crate implements most of the points in the `Syntax` chapter of the PDF reference, and therefore
serves as a very good basis for building various abstractions on top of it, without having to reimplement
the PDF parsing logic. The supported features include:
- Parsing the xref tables in all of its possible formats, including xref streams.
- Parsing of objects (also in object streams).
- Parsing and evaluating of PDF functions.
- Parsing and decoding PDF streams.
- Iterating over pages in a PDF as well as their content streams.

This crate does not provide more high-level functionality, such as parsing fonts or color spaces.
Such functionality is out-of-scope for `hayro-syntax`, since this crate is supposed to be
as light-weight and application-agnostic as possible. Functionality-wise, this crate is therefore
pretty much feature-complete, though more low-level APIs might be added in the future.

## Features
This crate has one feature, `jpeg2000`. PDF allows for the insertion of JPEG2000 images. However,
unfortunately, JPEG2000 is a very complicated format. There exists a Rust crate that allows decoding
such images (which is also used by `hayro-syntax`), but it is a very heavy dependency, has a lot of
unsafe code (due to having been ported with `c2rust`), and also has a dependency on libc, meaning that you
might be restricted in the targets you can build to. Because of this, I recommend not enabling this
feature, unless you absolutely need to be able to support such images. The main reason why this

<!-- cargo-rdme end -->
