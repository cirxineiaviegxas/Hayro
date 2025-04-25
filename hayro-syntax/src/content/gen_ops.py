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

def gen_struct(name, code, types):
    if code == "\"":
        code = "\\\""
    lifetime = "'a" if any(t in [Type.String, Type.Array, Type.Name] for t in types) else ""
    count = len(types)
    struct = [f"#[derive(Debug)]"]
    if count == 0:
        struct.append(f"pub struct {name};")
    elif count == 1:
        struct.append(f"pub struct {name}{f'<{lifetime}>' if lifetime else ''}(pub {rust_type(types[0])});")
    else:
        struct.append(f"pub struct {name}{f'<{lifetime}>' if lifetime else ''}(")
        struct += [f"    pub {rust_type(t)}," for t in types]
        struct.append(");")
    struct.append(f"op{count}!({name}{f'<{lifetime}>' if lifetime else ''}, \"{code}\");")
    return "\n".join(struct)

rust_structs = "\n\n".join(
    gen_struct(name, code, types)
    for category in ops.values()
    for code, name, types in category
)

print(rust_structs)
