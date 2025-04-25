#[derive(Debug)]
pub struct BeginCompatibility;
op0!(BeginCompatibility, "BX");

#[derive(Debug)]
pub struct EndCompatibility;
op0!(EndCompatibility, "EX");

#[derive(Debug)]
pub struct SaveState;
op0!(SaveState, "q");

#[derive(Debug)]
pub struct RestoreState;
op0!(RestoreState, "Q");

#[derive(Debug)]
pub struct Transform(
    pub Number,
    pub Number,
    pub Number,
    pub Number,
    pub Number,
    pub Number,
);
op6!(Transform, "cm");

#[derive(Debug)]
pub struct LineWidth(pub Number);
op1!(LineWidth, "w");

#[derive(Debug)]
pub struct LineCap(pub Number);
op1!(LineCap, "J");

#[derive(Debug)]
pub struct LineJoin(pub Number);
op1!(LineJoin, "j");

#[derive(Debug)]
pub struct MiterLimit(pub Number);
op1!(MiterLimit, "M");

#[derive(Debug)]
pub struct DashPattern<'a>(
    pub Array<'a>,
    pub Number,
);
op2!(DashPattern<'a>, "d");

#[derive(Debug)]
pub struct RenderingIntent<'a>(pub Name<'a>);
op1!(RenderingIntent<'a>, "ri");

#[derive(Debug)]
pub struct FlatnessTolerance(pub Number);
op1!(FlatnessTolerance, "i");

#[derive(Debug)]
pub struct SetGraphicsState<'a>(pub Name<'a>);
op1!(SetGraphicsState<'a>, "gs");

#[derive(Debug)]
pub struct MoveTo(
    pub Number,
    pub Number,
);
op2!(MoveTo, "m");

#[derive(Debug)]
pub struct LineTo(
    pub Number,
    pub Number,
);
op2!(LineTo, "l");

#[derive(Debug)]
pub struct CubicTo(
    pub Number,
    pub Number,
    pub Number,
    pub Number,
    pub Number,
    pub Number,
);
op6!(CubicTo, "c");

#[derive(Debug)]
pub struct CubicStartTo(
    pub Number,
    pub Number,
    pub Number,
    pub Number,
);
op4!(CubicStartTo, "v");

#[derive(Debug)]
pub struct CubicEndTo(
    pub Number,
    pub Number,
    pub Number,
    pub Number,
);
op4!(CubicEndTo, "y");

#[derive(Debug)]
pub struct ClosePath;
op0!(ClosePath, "h");

#[derive(Debug)]
pub struct RectPath(
    pub Number,
    pub Number,
    pub Number,
    pub Number,
);
op4!(RectPath, "re");

#[derive(Debug)]
pub struct StrokePath;
op0!(StrokePath, "S");

#[derive(Debug)]
pub struct CloseAndStrokePath;
op0!(CloseAndStrokePath, "s");

#[derive(Debug)]
pub struct FillPathNonZero;
op0!(FillPathNonZero, "f");

#[derive(Debug)]
pub struct FillPathNonZeroCompatibility;
op0!(FillPathNonZeroCompatibility, "F");

#[derive(Debug)]
pub struct FillPathEvenOdd;
op0!(FillPathEvenOdd, "f*");

#[derive(Debug)]
pub struct FillAndStrokeNonZero;
op0!(FillAndStrokeNonZero, "B");

#[derive(Debug)]
pub struct FillAndStrokeEvenOdd;
op0!(FillAndStrokeEvenOdd, "B*");

#[derive(Debug)]
pub struct CloseFillAndStrokeNonZero;
op0!(CloseFillAndStrokeNonZero, "b");

#[derive(Debug)]
pub struct CloseFillAndStrokeEvenOdd;
op0!(CloseFillAndStrokeEvenOdd, "b*");

#[derive(Debug)]
pub struct EndPath;
op0!(EndPath, "n");

#[derive(Debug)]
pub struct ShowText<'a>(pub string::String<'a>);
op1!(ShowText<'a>, "Tj");

#[derive(Debug)]
pub struct NextLineAndShowText<'a>(pub string::String<'a>);
op1!(NextLineAndShowText<'a>, "'");

#[derive(Debug)]
pub struct ShowTextWithParameters<'a>(
    pub Number,
    pub Number,
    pub string::String<'a>,
);
op3!(ShowTextWithParameters<'a>, "\"");

#[derive(Debug)]
pub struct ShowTexts<'a>(pub Array<'a>);
op1!(ShowTexts<'a>, "TJ");

#[derive(Debug)]
pub enum TypedOperation<'a> {
    BeginCompatibility(BeginCompatibility),
    EndCompatibility(EndCompatibility),
    SaveState(SaveState),
    RestoreState(RestoreState),
    Transform(Transform),
    LineWidth(LineWidth),
    LineCap(LineCap),
    LineJoin(LineJoin),
    MiterLimit(MiterLimit),
    DashPattern(DashPattern<'a>),
    RenderingIntent(RenderingIntent<'a>),
    FlatnessTolerance(FlatnessTolerance),
    SetGraphicsState(SetGraphicsState<'a>),
    MoveTo(MoveTo),
    LineTo(LineTo),
    CubicTo(CubicTo),
    CubicStartTo(CubicStartTo),
    CubicEndTo(CubicEndTo),
    ClosePath(ClosePath),
    RectPath(RectPath),
    StrokePath(StrokePath),
    CloseAndStrokePath(CloseAndStrokePath),
    FillPathNonZero(FillPathNonZero),
    FillPathNonZeroCompatibility(FillPathNonZeroCompatibility),
    FillPathEvenOdd(FillPathEvenOdd),
    FillAndStrokeNonZero(FillAndStrokeNonZero),
    FillAndStrokeEvenOdd(FillAndStrokeEvenOdd),
    CloseFillAndStrokeNonZero(CloseFillAndStrokeNonZero),
    CloseFillAndStrokeEvenOdd(CloseFillAndStrokeEvenOdd),
    EndPath(EndPath),
    ShowText(ShowText<'a>),
    NextLineAndShowText(NextLineAndShowText<'a>),
    ShowTextWithParameters(ShowTextWithParameters<'a>),
    ShowTexts(ShowTexts<'a>),
    Fallback,
}

impl<'a> TypedOperation<'a> {
    pub(crate) fn dispatch(operation: &Operation<'a>) -> Option<TypedOperation<'a>> {
        let op_name = operation.operator.get();
        Some(match op_name.as_ref() {
            b"BX" => BeginCompatibility::from_stack(&operation.operands)?.into(),
            b"EX" => EndCompatibility::from_stack(&operation.operands)?.into(),
            b"q" => SaveState::from_stack(&operation.operands)?.into(),
            b"Q" => RestoreState::from_stack(&operation.operands)?.into(),
            b"cm" => Transform::from_stack(&operation.operands)?.into(),
            b"w" => LineWidth::from_stack(&operation.operands)?.into(),
            b"J" => LineCap::from_stack(&operation.operands)?.into(),
            b"j" => LineJoin::from_stack(&operation.operands)?.into(),
            b"M" => MiterLimit::from_stack(&operation.operands)?.into(),
            b"d" => DashPattern::from_stack(&operation.operands)?.into(),
            b"ri" => RenderingIntent::from_stack(&operation.operands)?.into(),
            b"i" => FlatnessTolerance::from_stack(&operation.operands)?.into(),
            b"gs" => SetGraphicsState::from_stack(&operation.operands)?.into(),
            b"m" => MoveTo::from_stack(&operation.operands)?.into(),
            b"l" => LineTo::from_stack(&operation.operands)?.into(),
            b"c" => CubicTo::from_stack(&operation.operands)?.into(),
            b"v" => CubicStartTo::from_stack(&operation.operands)?.into(),
            b"y" => CubicEndTo::from_stack(&operation.operands)?.into(),
            b"h" => ClosePath::from_stack(&operation.operands)?.into(),
            b"re" => RectPath::from_stack(&operation.operands)?.into(),
            b"S" => StrokePath::from_stack(&operation.operands)?.into(),
            b"s" => CloseAndStrokePath::from_stack(&operation.operands)?.into(),
            b"f" => FillPathNonZero::from_stack(&operation.operands)?.into(),
            b"F" => FillPathNonZeroCompatibility::from_stack(&operation.operands)?.into(),
            b"f*" => FillPathEvenOdd::from_stack(&operation.operands)?.into(),
            b"B" => FillAndStrokeNonZero::from_stack(&operation.operands)?.into(),
            b"B*" => FillAndStrokeEvenOdd::from_stack(&operation.operands)?.into(),
            b"b" => CloseFillAndStrokeNonZero::from_stack(&operation.operands)?.into(),
            b"b*" => CloseFillAndStrokeEvenOdd::from_stack(&operation.operands)?.into(),
            b"n" => EndPath::from_stack(&operation.operands)?.into(),
            b"Tj" => ShowText::from_stack(&operation.operands)?.into(),
            b"'" => NextLineAndShowText::from_stack(&operation.operands)?.into(),
            b"\"" => ShowTextWithParameters::from_stack(&operation.operands)?.into(),
            b"TJ" => ShowTexts::from_stack(&operation.operands)?.into(),
            _ => return Self::Fallback.into(),
        })
    }
}