mod display;
mod fmt;
mod spec;

#[cfg(test)]
mod calculate_code_block_token_count {
    use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag, TagEnd};
    use pulldown_cmark_to_cmark::calculate_code_block_token_count;

    const CODE_BLOCK_START: Event<'_> = Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::Borrowed(""))));
    const CODE_BLOCK_END: Event<'_> = Event::End(TagEnd::CodeBlock);

    #[test]
    fn no_token() {
        let events = &[CODE_BLOCK_START, Event::Text("text".into()), CODE_BLOCK_END];
        assert_eq!(calculate_code_block_token_count(events.iter()), None);
    }

    #[test]
    fn backtick() {
        let events = &[CODE_BLOCK_START, Event::Text("```".into()), CODE_BLOCK_END];
        assert_eq!(calculate_code_block_token_count(events.iter()), Some(4));

        let events = &[CODE_BLOCK_START, Event::Text("````".into()), CODE_BLOCK_END];
        assert_eq!(calculate_code_block_token_count(events.iter()), Some(5));

        let events = &[CODE_BLOCK_START, Event::Text("``````````".into()), CODE_BLOCK_END];
        assert_eq!(calculate_code_block_token_count(events.iter()), Some(11));
    }

    #[test]
    fn tilde() {
        let events = &[CODE_BLOCK_START, Event::Text("~~~".into()), CODE_BLOCK_END];
        assert_eq!(calculate_code_block_token_count(events.iter()), Some(4));

        let events = &[CODE_BLOCK_START, Event::Text("~~~~".into()), CODE_BLOCK_END];
        assert_eq!(calculate_code_block_token_count(events.iter()), Some(5));

        let events = &[CODE_BLOCK_START, Event::Text("~~~~~~~~~~".into()), CODE_BLOCK_END];
        assert_eq!(calculate_code_block_token_count(events.iter()), Some(11));
    }

    #[test]
    fn mix() {
        let events = &[CODE_BLOCK_START, Event::Text("```~~~~".into()), CODE_BLOCK_END];
        assert_eq!(calculate_code_block_token_count(events.iter()), Some(5));

        let events = &[CODE_BLOCK_START, Event::Text("~~~~`````~~".into()), CODE_BLOCK_END];
        assert_eq!(calculate_code_block_token_count(events.iter()), Some(6));

        let events = &[
            CODE_BLOCK_START,
            Event::Text("~~~```````~~~```~~".into()),
            CODE_BLOCK_END,
        ];
        assert_eq!(calculate_code_block_token_count(events.iter()), Some(8));
    }

    #[test]
    fn splitted_text() {
        let events = &[
            CODE_BLOCK_START,
            Event::Text("~~~".into()),
            Event::Text("~~~".into()),
            CODE_BLOCK_END,
        ];
        assert_eq!(calculate_code_block_token_count(events.iter()), Some(7));

        let events = &[
            CODE_BLOCK_START,
            Event::Text("````".into()),
            Event::Text("````".into()),
            CODE_BLOCK_END,
        ];
        assert_eq!(calculate_code_block_token_count(events.iter()), Some(9));
    }
}
