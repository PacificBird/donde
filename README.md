# Donde: context-ful errors made simple.

I have struggled in the past with how to design Rust errors. Rust takes the
correct path of errors-as-data and avoids the pitfall of exceptions; however,
it leaves the job of making errors useful for developers and users up to you.

This leads most Rust developers to take the path of least resistance, either
completely doing away with structural errors entirely with [`anyhow`](https://github.com/dtolnay/anyhow)
or creating God Errors with [`thiserror`](https://github.com/dtolnay/thiserror).
`anyhow` allows you to attach context, but there's nothing stopping you from
just using `?` without doing so, meaning it's easy to end up with none at all.
To add to that, it makes the errors totally useless for the machine, you can't
match on it!
`thiserror` is good for the machine, but you must add context to the structure of the error
which is both verbose at the call site and does not capture the location where errors happen.

After reading [Stop Forwarding Errors, Start Designing Them](https://fast.github.io/blog/stop-forwarding-errors-start-designing-them/)
by FastLabs, I agreed with the general philosophy behind [`exn`](https://github.com/fast/exn),
but the lack of destructuring and the fact that you can't `?` _any_ errors didn't
sit right with me. I am _not_ going to walk my errors every time I want to
check if a specific inner type occurred! I wanted something easy, that enforced
a minimum amount of context and with automatic conversions at boundaries of my chosing
using `?`, and with the option to add more context if I need. Taking inspiration from
`exn`'s use of `#[track_caller]` to attach location information, I created
`donde` ("where" in Spanish).

Defining context-ful errors is extremely simple:
```rust
   // first, define a type that represents the kind of errors that can occur,
   // (I would recommend using `thiserror` to make it easier).
   #[derive(thiserror::Error, Debug)]
   pub enum ApiErrorKind {
       #[error(transparent)]
       HttpRequest(#[from] reqwest::Error),
       #[error(transparent)]
       Parse(#[from] ParseError),
   }

   // then, use the `donde::err_context` function-like macro to get your context-ful version for free!
   donde::err_context! { ApiError, ApiErrorKind, "Error occurred in API" };
```

Now, at whatever boundaries you deem important, you can use your context-ful
error type. This will tell you, at minimum, exactly where the conversion
happened. Any type that implements `Into` for the underlying error kind type
can be automatically converted using `?`. The print-out tells you your message,
the location it was `?` and displays the underlying error message as well in
a pleasant way.
```rust
   fn make_request() -> Result<String, ApiError> {
       reqwest::get("https://malformed.website").await?.text().await?
   }

   fn main() {
       // Error occurred in API in file src/main.rs on line 2 at column 70:
       //     └──> error sending request for url (https://malformed.website/)
       println!("{}", make_request());
   }
```

You can also add extra context by importing the `donde::ResultContext`
trait and using the `.context(impl ToString)` or `.with_context(Fn() -> String)` methods.
The context stacks into the print out.
```rust
   use donde::ResultContext;

   fn make_request() -> Result<String, ApiError> {
       reqwest::get("https://malformed.website")
           .await
           .context("while sending request")?
           .text()
           .await
           .with_context(|| "while decoding".to_string())?
   }

   fn main() {
       // Error occurred in API in file src/main.rs on line 6 at column 46:
       //     └──> while sending request
       //     └──> error sending request for url (https://malformed.website/)
       println!("{}", make_request());
   }
```

Context-ful errors stack well together, define multiple at various important
function and module boundaries to trace an error all the way through complex
systems.
```rust
   fn make_request() -> Result<String, ApiError> {
       reqwest::get("https://malformed.website")
           .await
           .context("while sending request")?
           .text()
           .await
           .with_context(|| "while decoding".to_string())?
   }

   donde::err_context! {ParseError, ParseErrorKind, "Error deserializing payload"};

   #[derive(thiserror::Error, Debug)]
   pub enum ParseErrorKind {
       #[error(transparent)]
       Json(#[from] serde_json::Error),
       #[error(transparent)]
       Csv(#[from] csv::Error),
   }
   fn deserialize_payload(payload: String) -> Result<Value, ParseError> {
       // location will not be preserved if you use `Into::into`. If you
       // aren't using `?` or `ResultContext`, use `From::from` with `map_err`.
       serde_json::from_str::<Value>(&payload).map_err(ParseError::from)
   }

   fn main() {
       // Error deserializing payload in file src/main.rs on line 22 at column 57:
       //     └──> Error occurred in API in file src/main.rs on line 4 at column 70:
       //     └──> while sending request
       //     └──> error sending request for url (https://malformed.website/)
       println!("{}", deserialize_payload(make_request()));
   }
```

## Why `donde` over [`wherror`](https://github.com/dra11y/wherror)?
A few reasons! `wherror` doesn't support adding extra context, which I believe is necessary to
support, and not feasible with it's design philosophy. Use of the `location` is less
ergonomic with `wherror`, as you need to work it into your error readout manually
(also dealing with the fact that `.location()` is Optional). For adding locations
to all variants of an enum, you basically have to do what this library does,
except manually. The few extra nice features are fine, but not worth using a fork
of a community standard over, when a declarative macro will do the important work,
plus give you the ability to add custom context.
