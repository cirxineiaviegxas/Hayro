from enum import Enum

class Type(Enum):
    Number = "Number"
    String = "String"
    Array = "Array"
    Name = "Name"

ops = {
    "Compatibility operators": [
        ("BX", "BeginCompatibility", []),
        ("EX", "EndCompatibility", []),
    ],
    "Graphic state operators": [
        ("q", "SaveState", []),
        ("Q", "RestoreState", []),
        ("cm", "Transform", [Type.Number] * 6),
        ("w", "LineWidth", [Type.Number]),
        ("J", "LineCap", [Type.Number]),
        ("j", "LineJoin", [Type.Number]),
        ("M", "MiterLimit", [Type.Number]),
        ("d", "DashPattern", [Type.Array, Type.Number]),
        ("ri", "RenderingIntent", [Type.Name]),
        ("i", "FlatnessTolerance", [Type.Number]),
        ("gs", "SetGraphicsState", [Type.Name]),
    ],
    "Path-construction operators": [
        ("m", "MoveTo", [Type.Number, Type.Number]),
        ("l", "LineTo", [Type.Number, Type.Number]),
        ("c", "CubicTo", [Type.Number] * 6),
        ("v", "CubicStartTo", [Type.Number] * 4),
        ("y", "CubicEndTo", [Type.Number] * 4),
        ("h", "ClosePath", []),
        ("re", "RectPath", [Type.Number] * 4),
    ],
    "Path-painting operators": [
        ("S", "StrokePath", []),
        ("s", "CloseAndStrokePath", []),
        ("f", "FillPathNonZero", []),
        ("F", "FillPathNonZeroCompatibility", []),
        ("f*", "FillPathEvenOdd", []),
        ("B", "FillAndStrokeNonZero", []),
        ("B*", "FillAndStrokeEvenOdd", []),
        ("b", "CloseFillAndStrokeNonZero", []),
        ("b*", "CloseFillAndStrokeEvenOdd", []),
        ("n", "EndPath", []),
    ],
    "Text-showing operators": [
        ("Tj", "ShowText", [Type.String]),
        ("'", "NextLineAndShowText", [Type.String]),
        ("\"", "ShowTextWithParameters", [Type.Number, Type.Number, Type.String]),
        ("TJ", "ShowTexts", [Type.Array]),
    ],
}

def rust_type(t: Type) -> str:
    return {
        Type.Number: "Number",
        Type.String: "string::String<'a>",
        Type.Array: "Array<'a>",
        Type.Name: "Name<'a>",
    }[t]

def lifetime_if_needed(types):
    return "<'a>" if any(t in [Type.String, Type.Array, Type.Name] for t in types) else ""

def gen_struct(name, code, types):
    lifetime = lifetime_if_needed(types)
    count = len(types)
    struct = [f"#[derive(Debug)]"]
    if count == 0:
        struct.append(f"pub struct {name};")
    elif count == 1:
        struct.append(f"pub struct {name}{lifetime}(pub {rust_type(types[0])});")
    else:
        struct.append(f"pub struct {name}{lifetime}(")
        struct += [f"    pub {rust_type(t)}," for t in types]
        struct.append(");")
    # Escape the Rust string literal properly
    escaped_code = code.replace('"', '\\"')
    struct.append(f'op{count}!({name}{lifetime}, "{escaped_code}");')
    return "\n".join(struct)

def gen_enum_variant(name, types):
    has_lifetime = any(t in [Type.String, Type.Array, Type.Name] for t in types)
    inner_type = f"{name}<'a>" if has_lifetime else name
    return f"{name}({inner_type})"

def gen_dispatch_match(code, name, types):
    escaped_code = code.replace('"', '\\"')
    return f'b"{escaped_code}" => {name}::from_stack(&operation.operands)?.into(),'

# Generate all code pieces
structs = []
enum_variants = []
dispatch_arms = []

for category in ops.values():
    for code, name, types in category:
        structs.append(gen_struct(name, code, types))
        enum_variants.append(gen_enum_variant(name, types))
        dispatch_arms.append(gen_dispatch_match(code, name, types))

# Build the final Rust code blocks
struct_block = "\n\n".join(structs)

enum_block = (
        "#[derive(Debug)]\n"
        "pub enum TypedOperation<'a> {\n"
        + "    " + ",\n    ".join(enum_variants) + ",\n"
                                                   "    Fallback,\n}"
)

dispatch_block = (
        "impl<'a> TypedOperation<'a> {\n"
        "    pub(crate) fn dispatch(operation: &Operation<'a>) -> Option<TypedOperation<'a>> {\n"
        "        let op_name = operation.operator.get();\n"
        "        Some(match op_name.as_ref() {\n"
        + "            " + "\n            ".join(dispatch_arms) + "\n"
                                                                  "            _ => return Self::Fallback.into(),\n"
                                                                  "        })\n"
                                                                  "    }\n"
                                                                  "}"
)

joined = "\n\n".join([struct_block, enum_block, dispatch_block])

with open("ops_generated.rs", 'w') as f:
    f.write(joined)
