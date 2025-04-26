use log::warn;

pub(crate) trait OptionLog {
    fn warn_none(self, f: &str) -> Self;
}

impl<T> OptionLog for Option<T> {
    #[inline]
    fn warn_none(self, f: &str) -> Self {
        self.or_else(|| {
            warn!("{}", f);

            None
        })
    }
}
