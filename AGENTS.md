# Repository Development Rules for pulldown-cmark-to-cmark

These rules apply to all contributors, including AI agents.

## AI Agent Communication

- AI agents communicating through a person's account must identify themselves
  and speak only for themselves, including in issue or PR descriptions and comments.
- Agents must not generate statements on the account holder's behalf, including
  claims that the account holder reviewed, tested, or approved the work. The
  account holder must write any such statement themselves.
- Agents creating a PR must explain why the work was pursued, such as the project
  that will benefit or where the bug was first discovered. Any motivation is
  valid; explain it rather than merely restating what the PR does. "Want to fix a
  bug" is insufficient because that is already evident from the PR itself.
- AI assistance that does not replace the person as the speaker, such as
  proofreading or wording polish, does not require identification.
- Attributing AI assistance in commit metadata, for example with an `Assisted-by:`
  or `Co-authored-by:` trailer, is welcome but not required.

## Commit Messages

Follow "purposeful conventional commits" style:

- Write commit messages in Markdown, assuming readers view them with syntax
  highlighting. Enclose code, crate names, and shell commands in backticks, and
  use markup when it helps readers understand or navigate the prose.
- Use the body to explain what motivated the change, not merely what changed.
- Use conventional commit prefixes only when the message should appear in the
  changelog. Use `feat:` or `fix:` for features or fixes visible to users.
- This is a single-crate repository, so use unscoped titles.
- Refactors and chores that do not affect users get no prefix. If a refactor
  changes the public API, use `feat`, `change`, `rename`, or `remove` with the
  breaking-change marker instead.
- Breaking changes must appear in the changelog. Put `!` after the prefix and
  before the colon, for example `change!:`, `remove!:`, `rename!:`, or `feat!:`.
- Do not edit `CHANGELOG.md`. It is generated from conventional commits; express
  changelog-worthy changes through commit messages instead.
- Each commit must be self-contained and pass CI independently. Keep a breaking
  change and all adaptations needed to build and test the crate in the same commit.

Examples:

```text
fix: validate GFM footnote references alongside emphasis
feat: add incremental serialization to `State`
change!: require parser options when serializing events
Explain the delimiter validation fast path
```
