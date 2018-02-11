[![Build Status](https://travis-ci.org/Byron/termbook.svg?branch=master)](https://travis-ci.org/Byron/termbook)

A utility library which translates [`Event`][pdcm-event] back to markdown.
It's the prerequisite for writing markdown filters which can work as
[mdbook-preprocessors][mdbook-prep].

[pdcm-event]: https://docs.rs/pulldown-cmark/0.1.0/pulldown_cmark/enum.Event.html
[mdbook-prep]: https://rust-lang-nursery.github.io/mdBook/for_developers/preprocessors.html

### How to use

Please have a look at the [`stupicat`-example][sc-example] for a complete tour
of the API, or have a look at the [api-docs][api].

It's easiest to get this library into your `Cargo.toml` using `cargo-add`:
```
cargo add pulldown-cmark-to-cmark
```

[sc-example]: https://github.com/Byron/termbook/blob/8af7230ec7b9d5e72f43214dfa7540f90a2e6da9/lib/pulldown-cmark-to-cmark/examples/stupicat.rs#L21
[api]: https://docs.rs/crate/pulldown-cmark-to-cmark

### Maintenance Guide

#### Making a new release

 * **Assure all documentation is up-to-date and tests are green**
 * update the `version` in `Cargo.toml` and `git commit`
 * run `cargo publish`
 * run `git tag -s pulldown-cmark-to-cmark_<version>`
 * run `git push --tags origin master`
