### How to contribute

* [fork this project](https://github.com/Byron/termbook/fork) on github
* For setting up the environment to run the self tests, look at `.travis.yml`.
* **Write a test that fails unless your patch is present.**
  * There are fixture-based tests run by [`cat.sh`][sh-tests].
  * There are [unit-level tests][unit-tests] run by `cargo test`.
* **Write the patch to fix the test**.
* Add yourself to the `authors` line in the [`Cargo.toml`][cargo-authors] file.
* Initiate a pull request

[cargo-authors]: https://github.com/Byron/termbook/blob/master/lib/pulldown-cmark-to-cmark/Cargo.toml#L4 
[unit-tests]: https://github.com/Byron/termbook/blob/72601bf471204a2b05e373fbba1f0659bac52fd6/lib/pulldown-cmark-to-cmark/tests/fmt.rs 
[sh-tests]: https://github.com/Byron/termbook/blob/72601bf471204a2b05e373fbba1f0659bac52fd6/lib/pulldown-cmark-to-cmark/tests/cat.sh

