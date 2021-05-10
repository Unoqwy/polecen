use fancy_regex::Regex;

// TODO: convert fancy regex to native rust parsing
// benefits: performance, detecting invalid formatted input, and knowing where it is invalid
// foreseeable breaking change: split_args will return Result

lazy_static::lazy_static! {
    static ref SPLIT_ARGUMENTS_PATTERN: Regex =
        Regex::new(r#"(?<!\\)"(?:\\.|[^"\\])*?"|(?<!")(?:\\.|[^"\s])+(?!")"#).unwrap();
}

pub fn split_args<'a>(text: &'a str) -> Box<dyn Iterator<Item = &'a str> + Send + 'a> {
    Box::new(
        SPLIT_ARGUMENTS_PATTERN
            .captures_iter(text)
            .filter_map(|c| c.ok())
            .filter_map(|c| c.get(0))
            .map(|g| g.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                if s.starts_with("\"") && s.ends_with("\"") {
                    s.trim_matches(|c| c == '"')
                } else {
                    s
                }
            }),
    )
}
