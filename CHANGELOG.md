### v6.0.1 (2021-06-23)

* handline [html in tables correctly](https://github.com/Byron/pulldown-cmark-to-cmark/pull/26).

### v6.0.1 (2021-06-04)

* also escape ']' characters

### v5.0.0 (2020-08-02)

* Allow configuring the [amount of backticks used in code blocks](https://github.com/Byron/pulldown-cmark-to-cmark/pull/18). 
  May **break** code relying on the amount of fields in the configuration struct.

### v4.0.2 (2020-04-22)

* Fixed table header handling ([see PR for details](https://github.com/Byron/pulldown-cmark-to-cmark/pull/15))

### v4.0.0 (2020-04-22)

* BREAKING: Move all types from `pulldown_cmark_to_cmark::fmt::*` into `pulldown_cmark_to_cmark::*` for simplicity. 
  For most common use-cases, this means that users of `pulldown_cmark_to_cmark::fmt::cmark` now use `pulldown_cmark_to_cmark::cmark` instead.

### v3.0.1 (2020-04-22)

* support for markdown embedded in HTML tags, like

  ```markdown
  <article>
    
    * list
    * list

    **bold**

  </article>
  ```
