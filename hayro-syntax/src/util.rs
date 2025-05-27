use log::{error, warn};

pub(crate) trait OptionLog {
    fn error_none(self, f: &str) -> Self;
    fn warn_none(self, f: &str) -> Self;
}

impl<T> OptionLog for Option<T> {
    fn error_none(self, f: &str) -> Self {
        self.or_else(|| {
            error!("{}", f);

            None
        })
    }

    fn warn_none(self, f: &str) -> Self {
        self.or_else(|| {
            warn!("{}", f);

            None
        })
    }
}
