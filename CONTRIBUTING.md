### How to contribute

* [fork this project][fork] on github
* For setting up the environment to run the self tests, look at `.github/workflows/rust.yml`.
* **Write a test that fails unless your patch is present.**
  * There are fixture-based tests run by [`cat.sh`][sh-tests].
  * There are [unit-level tests][unit-tests] run by `cargo test`.
* **Write the patch to fix the test**.
* Add yourself to the `authors` line in the [`Cargo.toml`][cargo-authors] file.
* Initiate a pull request

[fork]: https://github.com/Byron/pulldown-cmark-to-cmark/fork
[cargo-authors]: https://github.com/Byron/pulldown-cmark-to-cmark/blob/master/Cargo.toml#L4 
[unit-tests]: https://github.com/Byron/pulldown-cmark-to-cmark/blob/76667725b61be24890fbdfed5e7ecdb4c1ad1dc8/tests/fmt.rs#L146 
[sh-tests]: https://github.com/Byron/pulldown-cmark-to-cmark/blob/76667725b61be24890fbdfed5e7ecdb4c1ad1dc8/tests/cat.sh#L16
