use crate::content::{TypedIter, UntypedIter};
use crate::object::Object;
use crate::object::array::Array;
use crate::object::dict::Dict;
use crate::object::dict::keys::{CONTENTS, CROP_BOX, KIDS, MEDIA_BOX, RESOURCES, TYPE};
use crate::object::name::Name;
use crate::object::stream::Stream;
use kurbo::Rect;
use log::warn;
use std::cell::OnceCell;

pub struct Pages<'a> {
    pub pages: Vec<Page<'a>>,
}

impl<'a> Pages<'a> {
    pub fn new(pages_dict: Dict<'a>) -> Option<Pages<'a>> {
        let mut pages = vec![];
        resolve_pages(pages_dict, &mut pages)?;

        Some(Self { pages })
    }

    pub fn len(&self) -> usize {
        self.pages.len()
    }
}

fn resolve_pages<'a>(pages_dict: Dict<'a>, entries: &mut Vec<Page<'a>>) -> Option<()> {
    let kids = pages_dict.get::<Array<'a>>(KIDS)?;

    // TODO: Add inheritance of page attributes

    for dict in kids.iter::<Dict>() {
        match dict.get::<Name>(TYPE)?.get().as_ref() {
            b"Pages" => resolve_pages(dict, entries)?,
            b"Page" => entries.push(Page::new(dict)),
            _ => return None,
        }
    }

    Some(())
}

pub struct Page<'a> {
    inner: Dict<'a>,
    page_streams: OnceCell<Option<Vec<u8>>>,
}

impl<'a> Page<'a> {
    fn new(dict: Dict<'a>) -> Page<'a> {
        Self {
            inner: dict,
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

    pub fn media_box(&self) -> Rect {
        let arr: Vec<f32> = self
            .inner
            .get::<Array>(MEDIA_BOX)
            .unwrap()
            .iter::<f32>()
            .collect();

        Rect::new(arr[0] as f64, arr[1] as f64, arr[2] as f64, arr[3] as f64)
    }

    pub fn crop_box(&self) -> Rect {
        let media_box = self.media_box();

        let crop_box = if let Some(crop_box) = self.inner.get::<Array>(CROP_BOX) {
            let arr: Vec<f32> = crop_box.iter::<f32>().collect();

            Rect::new(arr[0] as f64, arr[1] as f64, arr[2] as f64, arr[3] as f64)
        } else {
            media_box
        };

        media_box.intersect(crop_box)
    }

    pub fn operations(&self) -> UntypedIter {
        self.operations_impl().unwrap_or(UntypedIter::empty())
    }

    pub fn typed_operations(&self) -> TypedIter {
        TypedIter::new(self.operations().into_iter())
    }
}
