use log::error;

pub(crate) trait OptionLog {
    fn error_none(self, f: &str) -> Self;
}

impl<T> OptionLog for Option<T> {
    fn error_none(self, f: &str) -> Self {
        self.or_else(|| {
            error!("{}", f);

            None
        })
    }
}
