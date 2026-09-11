#![doc = include_str!("../README.md")]

/// Newline + indent with arrow diagram. Use this to indent your own errors the same way as `whererror`!
pub const INDENT: &'static str = "\n    └──> ";

/// This trait allows for conversions from Results without contextful errors into ones
/// with context.
pub trait ResultContext<T, E, Ctx> {
    fn context(self, context: impl std::string::ToString) -> std::result::Result<T, Ctx>;

    fn with_context(self, f: impl Fn() -> String) -> std::result::Result<T, Ctx>;
}

#[macro_export]
macro_rules! err_context {
    ($ctx:ident, $kind:ty, $msg:literal) => {
        #[derive(Debug)]
        pub struct $ctx {
            /// The error that happened.
            pub kind: $kind,
            /// File name, line, and column where `kind` was converted into this error.
            location: (&'static str, u32, u32),
            /// Optional context for when `kind` was converted into this error.
            context: String,
        }

        impl std::fmt::Display for $ctx {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "{msg} in file {file} on line {line} at column {column}:{indent}{context}{kind}",
                    msg = $msg,
                    indent = $crate::INDENT,
                    context = if !self.context.is_empty() {
                            [&self.context, $crate::INDENT].concat()
                        } else {
                            "".to_string()
                        },
                    file = self.location.0,
                    line = self.location.1,
                    column = self.location.2,
                    kind = self.kind
                )
            }
        }

        impl std::error::Error for $ctx {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.kind)
            }
        }

        impl $ctx {
            /// Gets a reference to kind of error that occurred.
            #[allow(unused)]
            pub fn kind(&self) -> &$kind {
                &self.kind
            }

            /// Gets the kind of error that occurred by consuming self.
            #[allow(unused)]
            pub fn into_kind(self) -> $kind {
                self.kind
            }

            /// Optional context for when `kind` was converted into this error.
            #[allow(unused)]
            pub fn context(&self) -> &str {
                &self.context
            }

            /// Optional context for when `kind` was converted into this error, consumes self.
            #[allow(unused)]
            pub fn into_context(self) -> String {
                self.context
            }

            /// Add or change context for this error.
            pub fn set_context(self, context: impl std::string::ToString) -> $ctx {
                $ctx {
                    context: context.to_string(),
                    ..self
                }
            }

            /// File name, line, and column where `kind` was converted into this error.
            pub fn location(&self) -> (&'static str, u32, u32) {
                self.location
            }

        }

        impl<T> $crate::ResultContext<T, $ctx, $ctx> for std::result::Result<T, $ctx> {
            fn context(self, context: impl std::string::ToString) -> std::result::Result<T, $ctx> {
                match self {
                    Ok(val) => Ok(val),
                    Err(err) => Err(err.set_context(context.to_string())),
                }
            }

            fn with_context(self, f: impl Fn() -> String) -> std::result::Result<T, $ctx> {
                match self {
                    Ok(val) => Ok(val),
                    Err(err) => Err(err.set_context(f())),
                }
            }
        }

        impl<T: Into<$kind>> From<T> for $ctx {
            /// Turns errors which can themselves be turned into `kind` into this error,
            /// tracking where this conversion happened. Works with `?` but does not work with [Into].
            #[track_caller]
            fn from(value: T) -> Self {
                let loc = std::panic::Location::caller();
                $ctx {
                    kind: value.into(),
                    location: (loc.file(), loc.line(), loc.column()),
                    context: String::new(),
                }
            }
        }

        impl<T, E> $crate::ResultContext<T, E, $ctx> for std::result::Result<T, E>
        where
            E: Into<$kind>,
        {

            #[track_caller]
            fn context(self, context: impl std::string::ToString) -> std::result::Result<T, $ctx> {
                let loc = std::panic::Location::caller();
                match self {
                    Ok(val) => Ok(val),
                    Err(err) => Err($ctx {
                        kind: err.into(),
                        location: (loc.file(), loc.line(), loc.column()),
                        context: context.to_string(),
                    }),
                }
            }

            #[track_caller]
            fn with_context(self, f: impl Fn() -> String) -> std::result::Result<T, $ctx> {
                let loc = std::panic::Location::caller();
                match self {
                    Ok(val) => Ok(val),
                    Err(err) => Err($ctx {
                        kind: err.into(),
                        location: (loc.file(), loc.line(), loc.column()),
                        context: f(),
                    }),
                }
            }
        }
    };
}

#[cfg(test)]
mod test {
    use super::*;

    err_context!(Error, ErrorKind, "An error occurred");

    #[derive(Debug)]
    pub enum ErrorKind {
        HttpRequest,
        Parse,
    }
    impl std::fmt::Display for ErrorKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ErrorKind::HttpRequest => write!(f, "An error occurred during HTTP request!"),
                ErrorKind::Parse => write!(f, "An error occurred parsing the payload!"),
            }
        }
    }
    impl std::error::Error for ErrorKind {}

    #[test]
    fn test_capture_location_on_try_result() {
        fn f() -> Result<std::convert::Infallible, Error> {
            Err(ErrorKind::HttpRequest)?
        }

        assert!(matches!(
            f(),
            Err(Error {
                kind: ErrorKind::HttpRequest,
                location: ("src/lib.rs", _, _),
                context: _,
            })
        ));
    }

    #[test]
    fn test_context() {
        let result: Result<(), Error> = Err(ErrorKind::Parse).context("inside test");
        assert!(matches!(
            result,
            Err(Error {
                kind: ErrorKind::Parse,
                context,
                location: _
            }) if context == "inside test"
        ))
    }

    #[test]
    fn test_with_context() {
        let result: Result<(), Error> =
            Err(ErrorKind::Parse).with_context(|| "inside test".to_string());
        assert!(matches!(
            result,
            Err(Error {
                kind: ErrorKind::Parse,
                context,
                location: _
            }) if context == "inside test"
        ))
    }
}
