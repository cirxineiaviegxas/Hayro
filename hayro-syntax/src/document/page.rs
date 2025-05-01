use crate::content::{TypedIter, UntypedIter};
use crate::object::Object;
use crate::object::array::Array;
use crate::object::dict::Dict;
use crate::object::dict::keys::{CONTENTS, CROP_BOX, KIDS, MEDIA_BOX, RESOURCES, ROTATE, TYPE};
use crate::object::name::Name;
use crate::object::name::names::*;
use crate::object::rect::Rect;
use crate::object::stream::Stream;
use log::warn;
use std::cell::OnceCell;

pub struct Pages<'a> {
    pub pages: Vec<Page<'a>>,
}

#[derive(Debug, Clone)]
struct PagesContext {
    media_box: Option<Rect>,
    crop_box: Option<Rect>,
    rotate: Option<u32>
}

impl PagesContext {
    pub fn new() -> Self {
        Self {
            media_box: None,
            crop_box: None,
            rotate: None
        }
    }
}

impl<'a> Pages<'a> {
    pub fn new(pages_dict: Dict<'a>) -> Option<Pages<'a>> {
        let mut pages = vec![];
        let ctx = PagesContext::new();
        resolve_pages(pages_dict, &mut pages, ctx)?;

        Some(Self { pages })
    }

    pub fn len(&self) -> usize {
        self.pages.len()
    }
}

fn resolve_pages<'a>(
    pages_dict: Dict<'a>,
    entries: &mut Vec<Page<'a>>,
    mut ctx: PagesContext,
) -> Option<()> {
    if let Some(media_box) = pages_dict.get::<Rect>(MEDIA_BOX) {
        ctx.media_box = Some(media_box);
    }

    if let Some(crop_box) = pages_dict.get::<Rect>(CROP_BOX) {
        ctx.crop_box = Some(crop_box);
    }

    if let Some(rotate) = pages_dict.get::<u32>(ROTATE) {
        ctx.rotate = Some(rotate);
    }

    let kids = pages_dict.get::<Array<'a>>(KIDS)?;

    // TODO: Add inheritance of page attributes

    for dict in kids.iter::<Dict>() {
        match dict.get::<Name>(TYPE)?.as_ref() {
            PAGES => resolve_pages(dict, entries, ctx.clone())?,
            PAGE => entries.push(Page::new(dict, &ctx)),
            _ => return None,
        }
    }

    Some(())
}

#[derive(Debug, Copy, Clone)]
pub enum Rotation {
    None,
    Horizontal,
    Flipped,
    FlippedHorizontal
}

pub struct Page<'a> {
    inner: Dict<'a>,
    media_box: kurbo::Rect,
    crop_box: kurbo::Rect,
    rotation: Rotation,
    page_streams: OnceCell<Option<Vec<u8>>>,
}

impl<'a> Page<'a> {
    fn new(dict: Dict<'a>, ctx: &PagesContext) -> Page<'a> {
        let media_box = dict
            .get::<Rect>(MEDIA_BOX)
            .or_else(|| ctx.media_box)
            // TODO: A default media box
            .unwrap();

        let crop_box = dict
            .get::<Rect>(CROP_BOX)
            .or_else(|| ctx.crop_box)
            .unwrap_or(media_box);
        
        let rotation = match dict.get::<u32>(ROTATE).or_else(|| ctx.rotate).unwrap_or(0) % 360 {
            0 => Rotation::None,
            90 => Rotation::Horizontal,
            180 => Rotation::Flipped,
            270 => Rotation::FlippedHorizontal,
            _ => Rotation::None,
        };

        let crop_box = crop_box.get().intersect(media_box.get());

        Self {
            inner: dict,
            media_box: media_box.get(),
            crop_box,
            rotation,
            page_streams: OnceCell::new(),
        }
    }

    pub fn resources(&self) -> Dict<'a> {
        self.inner.get::<Dict>(RESOURCES).unwrap_or_default()
    }

    fn operations_impl(&self) -> Option<UntypedIter> {
        let convert_single = |s: Stream| {
            let data = s.decoded().ok()?;
            Some(data.to_vec())
        };

        let stream = self
            .page_streams
            .get_or_init(|| {
                match self.inner.get::<Object>(CONTENTS)? {
                    Object::Stream(stream) => convert_single(stream),
                    Object::Array(array) => {
                        let streams = array.iter::<Stream>().flat_map(convert_single);

                        let mut collected = vec![];

                        for stream in streams {
                            collected.extend(stream);
                            // Streams must have at least one whitespace in-between.
                            collected.push(b' ')
                        }

                        Some(collected)
                    }
                    _ => {
                        warn!("contents entry of page was neither stream nor array of streams");

                        return None;
                    }
                }
            })
            .as_ref()?;

        let iter = UntypedIter::new(&stream);

        Some(iter)
    }

    pub fn media_box(&self) -> kurbo::Rect {
        self.media_box
    }
    
    pub fn rotation(&self) -> Rotation {
        self.rotation
    }

    pub fn crop_box(&self) -> kurbo::Rect {
        self.crop_box
    }

    pub fn operations(&self) -> UntypedIter {
        self.operations_impl().unwrap_or(UntypedIter::empty())
    }

    pub fn typed_operations(&self) -> TypedIter {
        TypedIter::new(self.operations().into_iter())
    }
}
