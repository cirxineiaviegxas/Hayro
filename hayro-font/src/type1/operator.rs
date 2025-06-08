pub(crate) mod sb_operator {
    pub const HORIZONTAL_STEM: u8 = 1;
    pub const VERTICAL_STEM: u8 = 3;
    pub const VERTICAL_MOVE_TO: u8 = 4;
    pub const LINE_TO: u8 = 5;
    pub const HORIZONTAL_LINE_TO: u8 = 6;
    pub const VERTICAL_LINE_TO: u8 = 7;
    pub const CURVE_TO: u8 = 8;
    pub const CLOSE_PATH: u8 = 9;
    pub const CALL_SUBR: u8 = 10;
    pub const RETURN: u8 = 11;
    pub const ESCAPE: u8 = 12;
    pub const HSBW: u8 = 13;
    pub const ENDCHAR: u8 = 14;
    pub const MOVE_TO: u8 = 21;
    pub const HORIZONTAL_MOVE_TO: u8 = 22;
    pub const VH_CURVE_TO: u8 = 30;
    pub const HV_CURVE_TO: u8 = 31;
}

pub(crate) mod tb_operator {
    pub const DOTSECTION: u8 = 0;
    pub const VSTEM3: u8 = 1;
    pub const HSTEM3: u8 = 2;
    pub const SEAC: u8 = 6;
    pub const SBW: u8 = 7;
    pub const DIV: u8 = 12;
    pub const CALL_OTHER_SUBR: u8 = 16;
    pub const POP: u8 = 17;
    pub const SET_CURRENT_POINT: u8 = 33;
}
