<!-- Modified from <https://github.com/pulldown-cmark/pulldown-cmark/blob/8713a415b04cdb0b7980a9a17c0ed0df0b36395e/pulldown-cmark/specs/math.txt> -->

# `$`-delimited LaTeX Math in pulldown-cmark

Mathematical expressions extension. Syntax based on
<https://github.com/jgm/commonmark-hs/blob/master/commonmark-extensions/test/math.md>.

Inline mode mathematical expressions:

This sentence uses `$` delimiters to show math inline: $\sqrt{3x-1}+(1+x)^2$
$\sum_{k=1}^n a_k b_k$: Mathematical expression at head of line

Display mode mathematical expressions:

**The Cauchy-Schwarz Inequality**

$$\left( \sum_{k=1}^n a_k b_k \right)^2 \leq \left( \sum_{k=1}^n a_k^2 \right) \left( \sum_{k=1}^n b_k^2 \right)$$

Inline math expressions cannot be empty, but display mode expressions can.

Oops empty $$ expression.

$$$$

This is a greedy, left-to-right parser.

$x$$$$$$$y$$

$x$$$$$$y$$

$$x$$$$$$y$$

Math expressions pass their content through as-is, ignoring any other inline
Markdown constructs:

$a<b>c</b>$

$${a*b*c} _c_ d$$

$not `code`$

$![not an](/image)$

$<https://not.a.link/>$

$&alpha;$

Sole `$` characters without a matching pair in the same block element
are handled as normal text.

Hello $world.

Dollar at end of line$

Mathematical expressions can continue across multiple lines:

$5x + 2 =
17$

$$\left( \sum_{k=1}^n a_k b_k \right)^2 \leq \left( \sum_{k=1}^n a_k^2 \right)
\left( \sum_{k=1}^n b_k^2 \right)$$

Markdown hard breaks are also not recognized inside math expressions:

$not a\
hard break  
either$

`$` character can be escaped with backslash in mathematical expressions:

$\$$

$$y = \$ x$$

Inline mode math expressions cannot contain unescaped `$` characters.
Neither can display math.

$x $ x$

$$ $ $$

alpha$$beta$gamma$$delta

Inline math expressions cannot start or end with whitespace, including newlines:

these are not math texts: $ y=x$, $y=x $, $
y=x$ and $y=x
$

>The start of a line counts as whitespace $2 +
>$

While displays can start with whitespace, {${
they should not allow inlines to do that $$2 +
$*$

Inline math expressions do not need to be surrounded with whitespace:

these are math texts: foo$y=x$bar and $y=x$bar and foo$y=x$ bar

Inline math expressions can be surrounded by punctuation:

math texts: $x=y$! and $x=y$? and $x=y$: and $x=y$. and $x=y$"

also math texts: !$x=y$! and ?$x=y$? and :$x=y$: and .$x=y$. and "$x=y$"

braces: ($x=y$) [$x=y$] {$x=y$}

Math expression as only item on a line:

$x=y$

Math expressions can be immediately followed by other math expressions:

$a$$b$

$a$$$b$$

$$a$$$b$

$$a$$$$b$$

Both inline and display mode math expressions are inline elements with the same
precedence as code spans. The leftmost valid element takes priority:

$Inline `first$ then` code

`Code $first` then$ inline

$$ Display `first $$ then` code

`Code $$ first` then $$ display

Indicators of block structure take precedence over math expressions:

$x + y - z$

$x + y
- z$

$$ x + y
> z $$

This also means that math expressions cannot contain empty lines, since they
start a new paragraph:

$not

math$

$$
not

math
$$

It also implies that math notation has lower
parsing power than block elements.

- $not
    - *
  math$

Note that math can contain embedded math.  In scanning
for a closing delimiter, we skip material in balanced
curly braces:

This is display math:
$$
\text{Hello $x^2$}
$$
And this is inline math:
$\text{Hello $x$ there!}$

Math expressions must be nested within balanced curly braces.
Backslash-escaped braces do not count.

This is not valid math: $}{$

Neither is this: { $}{$ }

This is: $\}\{$

This is: $\}$

Math environment contains 2+2: $}$2+2$

Math environment contains y: $x {$ $ } $y$

Math expressions must contain properly nested braces.

This is not display math. It is inline math:

$$\text{first $$ second}$

$$$\text{first $$ second}$

This is display math:

$$\text{first $$ second}$$

$$$\text{first $$ second}$$

This is also display math, but (counterintuitively) it's allowed to be empty
and expected to be as short as possible:

$$$$\text{first $$ second}$$

Dollar signs must also be backslash-escaped if they occur within math:

$\text{\$}$

$$x$x$$

${$^$$

$}$$$$

$}$] $$

## Edge case tests comparison with GitHub

Test cases
https://raw.githubusercontent.com/nschloe/github-math-bugs/db938ff690ab7c534d8195fe4a1a5163c20b1134/README.md

Inline math wrapped in quotes

$x$ $`y`$

Inline and display math in the same list

- $a$

  ```math
  a
  ```

  $$
  a
  $$

- ```math
  b
  ```

  $$
  b
  $$

Images and math in the same list

- ![node logo](https://nodejs.org/static/images/logo.svg)
- $x$

Inline and display math in `<details>`

<details>

$A = 5$

$$
A = 5
$$

</details>

`<` without surrounding whitespace

$a<b$

$$a<b$$

Math in footnotes

[^a]

Math in links

[$a$](x)

Math preceded by an alphabetical character

a$x$

-$x$

1$x$

Inline math at the end of italic text

_$a$ equals $b$_

**$a$ equals $b$**

Dollar in `\text`

$$
a
$$

- $$
  \text{$b$}
  $$

Backslashes in `$`-math

$\{a\,b\}$

Math vs. HTML mix-up

$a <b > c$

$[(a+b)c](d+e)$

${a}_b c_{d}$

Dollar-math with spaces

When $a \ne 0$, there are two solutions to $(ax^2 + bx + c = 0)$ and they are
$$ x = {-b \pm \sqrt{b^2-4ac} \over 2a} $$

Spacing around dollar sign in math mode

$x = \$$

Math in italic text

_Equation $\Omega(69)$ in italic text_

Inline math can't be preceded by brackets, quotation marks etc.

$\pi$
'$\pi$
"$\pi$
($\pi$
[$\pi$
{$\pi$
/$\pi$

## Relationship with tables

As a block element, tables parsing is stronger than math.

| first $|$ second |
|--------|---------|
| a ${   | }$ b    |

As a special case, pipes in math environments in tables are escaped
with backslashes. Though backslash-escaped characters in math
environments are normally passed through verbatim to the LaTeX
engine, escaped pipes in tables are an exception like they
are in code spans.

The behavior of the table parser should be as-if it found the bounds
of the table cell in a separate pass that only looked for the
strings `|` and `\|`, treating pipes as boundaries and removing the
escaping backslash before passing the string to the inline parser.

| first $\|$ second |
|-------------------|
| a ${   \| }$ b    |

| Description | Test case |
|-------------|-----------|
| Single      | $\$       |
| Double      | $\\$      |
| Basic test  | $\|$      |
| Basic test 2| $\|\|\$   |
| Basic test 3| $x\|y\|z\$|
| Not pipe    | $\.$      |
| Combo       | $\.\|$    |
| Combo 2     | $\.\|\$   |
| Extra       | $\\\.$    |
| Wait, what? | $\\|$     |
| Wait, what? | $\\\|$    |
| Wait, what? | $\\\\|$   |
| Wait, what? | $\\\\\|$  |

## Implementation limits

Implementations may impose limits on brace nesting to avoid performance issues,
but at least three levels of nesting should be supported.

Pulldown-cmark imposes the following limits:

1. At 25 levels of nesting, it switches from tracking nested pairs to simply
   counting the number of braces. This means the below example will spurriously
   recognize a math environment with the correct number of braces, but not
   nested correctly.

This is not an inline math environment: $}{$
But, because it's nested too deeply, this is parsed as an inline math environment:
{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{
improperly $}{$ nested
}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}
But this still isn't, because the braces are still counted: $}{$

This is also deeply nested, but, unlike the first example,
they don't have an equal number of close braces and open braces,
so aren't detected as math.
{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{
improperly $}$ nested ${$ example
}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}
This, however, is detected ${}$

${{{{{{{{{{{{{{{{{{{{{{{{{{{{{{
another improperly nested example
}}}}}}}}}}}}}}}}}}}}}}}}}}}}}}$

2. At 255 distinct brace-delimited groups, the counter rolls over. This means
   the below example will spurriously recognize an incorrectly-nested
   inline math environment.

${}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}  20 brace pairs
{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}  40 brace pairs
{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}  60 brace pairs
{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}  80 brace pairs
{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{} 100 brace pairs
{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{} 120 brace pairs
{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{} 140 brace pairs
{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{} 160 brace pairs
{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{} 180 brace pairs
{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{} 200 brace pairs
{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{} 220 brace pairs
{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{} 240 brace pairs
{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{$ 255 brace pairs and one unclosed brace

3. Thanks to rule 1, though, deeply-nested structures won't chew through all of
   the ID space. This means that the below example, even though it nests 255
   levels deep, parses correctly anyway.

${{{{{{{{{{{{{{{{{{{{ 20 open braces
{{{{{{{{{{{{{{{{{{{{  40 open braces
{{{{{{{{{{{{{{{{{{{{  60 open braces
{{{{{{{{{{{{{{{{{{{{  80 open braces
{{{{{{{{{{{{{{{{{{{{ 100 open braces
{{{{{{{{{{{{{{{{{{{{ 110 open braces
{{{{{{{{{{{{{{{{{{{{ 120 open braces
{{{{{{{{{{{{{{{{{{{{ 140 open braces
{{{{{{{{{{{{{{{{{{{{ 160 open braces
{{{{{{{{{{{{{{{{{{{{ 180 open braces
{{{{{{{{{{{{{{{{{{{{ 200 open braces
{{{{{{{{{{{{{{{{{{{{ 220 open braces
{{{{{{{{{{{{{{{{{{{{ 240 open braces
{{{{{{{{{{{{{{{ 255 open braces
}}}}}}}}}}}}}}}}}}}}  20 close braces
}}}}}}}}}}}}}}}}}}}}  40 close braces
}}}}}}}}}}}}}}}}}}}}  60 close braces
}}}}}}}}}}}}}}}}}}}}  80 close braces
}}}}}}}}}}}}}}}}}}}} 100 close braces
}}}}}}}}}}}}}}}}}}}} 120 close braces
}}}}}}}}}}}}}}}}}}}} 140 close braces
}}}}}}}}}}}}}}}}}}}} 160 close braces
}}}}}}}}}}}}}}}}}}}} 180 close braces
}}}}}}}}}}}}}}}}}}}} 200 close braces
}}}}}}}}}}}}}}}}}}}} 220 close braces
}}}}}}}}}}}}}}}}}}}} 240 close braces
}}}}}}}}}}}}}}}{$ 255 close braces and one open brace

[^a]: Lorem $a$
