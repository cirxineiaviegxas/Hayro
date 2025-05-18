// Copyright 2025 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Basic render operations.

use crate::coarse::Wide;
use crate::encode::{EncodeExt, EncodedPaint};
use crate::fine::Fine;
use crate::flatten::Line;
use crate::mask::Mask;
use crate::paint::{Paint, PaintType};
use crate::pixmap::Pixmap;
use crate::strip::Strip;
use crate::tile::Tiles;
use crate::{flatten, strip};
use kurbo::{Affine, BezPath, Cap, Join, Rect, Shape, Stroke};
use peniko::color::palette::css::BLACK;
use peniko::{BlendMode, Compose, Fill, Mix};
use std::vec;
use std::vec::Vec;

pub(crate) const DEFAULT_TOLERANCE: f64 = 0.1;
/// A render context.
#[derive(Debug)]
pub struct RenderContext {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) wide: Wide,
    pub(crate) alphas: Vec<u8>,
    pub(crate) line_buf: Vec<Line>,
    pub(crate) tiles: Tiles,
    pub(crate) strip_buf: Vec<Strip>,
    pub(crate) paint: PaintType,
    pub(crate) paint_transform: Affine,
    pub(crate) stroke: Stroke,
    pub(crate) transform: Affine,
    pub(crate) fill_rule: Fill,
    pub(crate) encoded_paints: Vec<EncodedPaint>,
}

impl RenderContext {
    /// Create a new render context with the given width and height in pixels.
    pub fn new(width: u16, height: u16) -> Self {
        let wide = Wide::new(width, height);

        let alphas = vec![];
        let line_buf = vec![];
        let tiles = Tiles::new();
        let strip_buf = vec![];

        let transform = Affine::IDENTITY;
        let fill_rule = Fill::NonZero;
        let paint = BLACK.into();
        let paint_transform = Affine::IDENTITY;
        let stroke = Stroke {
            width: 1.0,
            join: Join::Bevel,
            start_cap: Cap::Butt,
            end_cap: Cap::Butt,
            ..Default::default()
        };
        let encoded_paints = vec![];

        Self {
            width,
            height,
            wide,
            alphas,
            line_buf,
            tiles,
            strip_buf,
            transform,
            paint,
            paint_transform,
            fill_rule,
            stroke,
            encoded_paints,
        }
    }

    fn encode_current_paint(&mut self) -> Paint {
        match self.paint.clone() {
            PaintType::Solid(s) => s.into(),
            PaintType::Image(i) => i.encode_into(&mut self.encoded_paints, self.transform),
            PaintType::ShadingPattern(s) => {
                s.encode_into(&mut self.encoded_paints, self.paint_transform)
            }
        }
    }

    /// Fill a path.
    pub fn fill_path(&mut self, path: &BezPath) {
        flatten::fill(path, self.transform, &mut self.line_buf);
        let paint = self.encode_current_paint();
        self.render_path(self.fill_rule, paint);
    }

    /// Stroke a path.
    pub fn stroke_path(&mut self, path: &BezPath) {
        flatten::stroke(path, &self.stroke, self.transform, &mut self.line_buf);
        let paint = self.encode_current_paint();
        self.render_path(Fill::NonZero, paint);
    }

    /// Fill a rectangle.
    pub fn fill_rect(&mut self, rect: &Rect) {
        self.fill_path(&rect.to_path(DEFAULT_TOLERANCE));
    }

    /// Push a new layer with the given properties.
    ///
    /// Note that the mask, if provided, needs to have the same size as the render context. Otherwise,
    /// it will be ignored. In addition to that, the mask will not be affected by the current
    /// transformation matrix in place.
    pub fn push_layer(
        &mut self,
        clip_path: Option<&BezPath>,
        blend_mode: Option<BlendMode>,
        opacity: Option<f32>,
        mask: Option<Mask>,
    ) {
        let clip = if let Some(c) = clip_path {
            flatten::fill(c, self.transform, &mut self.line_buf);
            self.make_strips(self.fill_rule);
            Some((self.strip_buf.as_slice(), self.fill_rule))
        } else {
            None
        };

        let mask = mask.and_then(|m| {
            if m.width() != self.width || m.height() != self.height {
                None
            } else {
                Some(m)
            }
        });

        self.wide.push_layer(
            clip,
            blend_mode.unwrap_or(BlendMode::new(Mix::Normal, Compose::SrcOver)),
            mask,
            opacity.unwrap_or(1.0),
        );
    }

    /// Pop the last-pushed layer.
    pub fn pop_layer(&mut self) {
        self.wide.pop_layer();
    }

    /// Set the current stroke.
    pub fn set_stroke(&mut self, stroke: Stroke) {
        self.stroke = stroke;
    }

    /// Set the current paint.
    pub fn set_paint(&mut self, paint: impl Into<PaintType>) {
        self.paint = paint.into();
    }

    /// Set the current fill rule.
    pub fn set_fill_rule(&mut self, fill_rule: Fill) {
        self.fill_rule = fill_rule;
    }

    /// Set the current transform.
    pub fn set_transform(&mut self, transform: Affine) {
        self.transform = transform;
    }

    /// Reset the render context.
    pub fn reset(&mut self) {
        self.line_buf.clear();
        self.tiles.reset();
        self.alphas.clear();
        self.strip_buf.clear();
        self.wide.reset();
    }

    /// Render the current context into a buffer.
    /// The buffer is expected to be in premultiplied RGBA8 format with length `width * height * 4`
    pub fn render_to_buffer(&self, buffer: &mut [u8], width: u16, height: u16) {
        assert!(
            !self.wide.has_layers(),
            "some layers haven't been popped yet"
        );
        assert_eq!(
            buffer.len(),
            (width as usize) * (height as usize) * 4,
            "provided width ({}) and height ({}) do not match buffer size ({})",
            width,
            height,
            buffer.len(),
        );

        let mut fine = Fine::new(width, height);
        let width_tiles = self.wide.width_tiles();
        let height_tiles = self.wide.height_tiles();
        for y in 0..height_tiles {
            for x in 0..width_tiles {
                let wtile = self.wide.get(x, y);
                fine.set_coords(x, y);

                fine.clear(wtile.bg.0);
                for cmd in &wtile.cmds {
                    fine.run_cmd(cmd, &self.alphas, &self.encoded_paints);
                }
                fine.pack(buffer);
            }
        }
    }

    /// Render the current context into a pixmap.
    pub fn render_to_pixmap(&self, pixmap: &mut Pixmap) {
        let width = pixmap.width();
        let height = pixmap.height();
        self.render_to_buffer(pixmap.data_as_u8_slice_mut(), width, height);
    }

    /// Return the width of the pixmap.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Return the height of the pixmap.
    pub fn height(&self) -> u16 {
        self.height
    }

    // Assumes that `line_buf` contains the flattened path.
    fn render_path(&mut self, fill_rule: Fill, paint: Paint) {
        self.make_strips(fill_rule);
        self.wide.generate(&self.strip_buf, fill_rule, paint);
    }

    fn make_strips(&mut self, fill_rule: Fill) {
        self.tiles
            .make_tiles(&self.line_buf, self.width, self.height);
        self.tiles.sort_tiles();

        strip::render(
            &self.tiles,
            &mut self.strip_buf,
            &mut self.alphas,
            fill_rule,
            &self.line_buf,
        );
    }
}
