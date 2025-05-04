use crate::run_test;

#[test]
fn clip_path_evenodd() {
    run_test("clip_path_evenodd", None);
}
#[test]
fn clip_path_nested() {
    run_test("clip_path_nested", None);
}
#[test]
fn color_space_icc_gray() {
    run_test("color_space_icc_gray", None);
}
#[test]
fn color_space_icc_srgb() {
    run_test("color_space_icc_srgb", None);
}
#[test]
fn font_cid_1() {
    run_test("font_cid_1", None);
}
#[test]
fn font_cid_2() {
    run_test("font_cid_2", Some(0..=0));
}
#[test]
fn font_cid_3() {
    run_test("font_cid_3", None);
}
#[test]
fn font_cid_4() {
    run_test("font_cid_4", None);
}
#[test]
fn font_cid_5() {
    run_test("font_cid_5", None);
}
#[test]
fn font_cid_6() {
    run_test("font_cid_6", None);
}
#[test]
fn font_cid_7() {
    run_test("font_cid_7", None);
}
#[test]
fn font_standard_1() {
    run_test("font_standard_1", None);
}
#[test]
fn font_standard_2() {
    run_test("font_standard_2", Some(0..=0));
}
#[test]
fn font_truetype_1() {
    run_test("font_truetype_1", None);
}
#[test]
fn font_truetype_2() {
    run_test("font_truetype_2", None);
}
#[test]
fn font_truetype_3() {
    run_test("font_truetype_3", None);
}
#[test]
fn font_truetype_4() {
    run_test("font_truetype_4", None);
}
#[test]
fn font_truetype_5() {
    run_test("font_truetype_5", None);
}
#[test]
fn font_truetype_6() {
    run_test("font_truetype_6", None);
}
#[test]
fn font_truetype_7() {
    run_test("font_truetype_7", Some(1..=1));
}
#[test]
fn font_type1_1() {
    run_test("font_type1_1", None);
}
#[test]
fn font_type1_10() {
    run_test("font_type1_10", Some(0..=1));
}
#[test]
fn font_type1_11() {
    run_test("font_type1_11", None);
}
#[test]
fn font_type1_12() {
    run_test("font_type1_12", None);
}
#[test]
fn font_type1_2() {
    run_test("font_type1_2", None);
}
#[test]
fn font_type1_3() {
    run_test("font_type1_3", None);
}
#[test]
fn font_type1_4() {
    run_test("font_type1_4", None);
}
#[test]
fn font_type1_5() {
    run_test("font_type1_5", None);
}
#[test]
fn font_type1_6() {
    run_test("font_type1_6", None);
}
#[test]
fn font_type1_7() {
    run_test("font_type1_7", None);
}
#[test]
fn font_type1_8() {
    run_test("font_type1_8", None);
}
#[test]
fn font_type1_9() {
    run_test("font_type1_9", None);
}
#[test]
fn font_type1_cff_1() {
    run_test("font_type1_cff_1", None);
}
#[test]
fn font_type1_cff_2() {
    run_test("font_type1_cff_2", None);
}
#[test]
fn font_type1_cff_3() {
    run_test("font_type1_cff_3", None);
}
#[test]
fn font_type1_cff_4() {
    run_test("font_type1_cff_4", None);
}
#[test]
fn font_type1_cff_5() {
    run_test("font_type1_cff_5", None);
}
#[test]
fn font_type1_cff_6() {
    run_test("font_type1_cff_6", None);
}
#[test]
fn fonts_type1_latex() {
    run_test("fonts_type1_latex", None);
}
#[test]
fn integration_coat_of_arms() {
    run_test("integration_coat_of_arms", None);
}
#[test]
fn integration_diagram() {
    run_test("integration_diagram", None);
}
#[test]
fn integration_matplotlib() {
    run_test("integration_matplotlib", None);
}
#[test]
fn issue_clipping_panic() {
    run_test("issue_clipping_panic", None);
}
#[test]
fn issue_cubic_start_end() {
    run_test("issue_cubic_start_end", None);
}
#[test]
fn issue_scaled_glyph() {
    run_test("issue_scaled_glyph", None);
}
#[test]
fn page_media_box_bottom_left() {
    run_test("page_media_box_bottom_left", None);
}
#[test]
fn page_media_box_bottom_right() {
    run_test("page_media_box_bottom_right", None);
}
#[test]
fn page_media_box_top_left() {
    run_test("page_media_box_top_left", None);
}
#[test]
fn page_media_box_top_right() {
    run_test("page_media_box_top_right", None);
}
#[test]
fn page_media_box_zoomed_out() {
    run_test("page_media_box_zoomed_out", None);
}
#[test]
fn page_rotation_180() {
    run_test("page_rotation_180", None);
}
#[test]
fn page_rotation_270() {
    run_test("page_rotation_270", None);
}
#[test]
fn page_rotation_90() {
    run_test("page_rotation_90", None);
}
#[test]
fn page_rotation_none() {
    run_test("page_rotation_none", None);
}
#[test]
fn path_rendering_1() {
    run_test("path_rendering_1", None);
}
#[test]
fn path_rendering_10() {
    run_test("path_rendering_10", None);
}
#[test]
fn path_rendering_11() {
    run_test("path_rendering_11", None);
}
#[test]
fn path_rendering_12() {
    run_test("path_rendering_12", None);
}
#[test]
fn path_rendering_13() {
    run_test("path_rendering_13", None);
}
#[test]
fn path_rendering_14() {
    run_test("path_rendering_14", None);
}
#[test]
fn path_rendering_15() {
    run_test("path_rendering_15", None);
}
#[test]
fn path_rendering_16() {
    run_test("path_rendering_16", None);
}
#[test]
fn path_rendering_17() {
    run_test("path_rendering_17", None);
}
#[test]
fn path_rendering_2() {
    run_test("path_rendering_2", None);
}
#[test]
fn path_rendering_3() {
    run_test("path_rendering_3", None);
}
#[test]
fn path_rendering_4() {
    run_test("path_rendering_4", None);
}
#[test]
fn path_rendering_5() {
    run_test("path_rendering_5", None);
}
#[test]
fn path_rendering_6() {
    run_test("path_rendering_6", None);
}
#[test]
fn path_rendering_7() {
    run_test("path_rendering_7", None);
}
#[test]
fn path_rendering_8() {
    run_test("path_rendering_8", None);
}
#[test]
fn path_rendering_9() {
    run_test("path_rendering_9", None);
}
#[test]
fn text_rendering_1() {
    run_test("text_rendering_1", None);
}
#[test]
fn text_rendering_2() {
    run_test("text_rendering_2", None);
}
#[test]
fn text_rendering_3() {
    run_test("text_rendering_3", None);
}
#[test]
fn text_rendering_4() {
    run_test("text_rendering_4", None);
}
#[test]
fn text_rendering_5() {
    run_test("text_rendering_5", None);
}
#[test]
fn text_rendering_clipping() {
    run_test("text_rendering_clipping", None);
}
#[test]
fn text_rendering_stroking_clipping() {
    run_test("text_rendering_stroking_clipping", None);
}
