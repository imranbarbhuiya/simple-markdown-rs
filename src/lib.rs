mod default_rules;
mod parse;

pub use default_rules::{create_default_rules, ASTNode, Rule, RuleMap, State};
pub use parse::parser_for;

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_text(input: &str) -> Vec<ASTNode> {
        let rules = create_default_rules();
        let parse = parser_for(rules);
        parse(input, None)
    }

    #[test]
    fn test_heading() {
        let result = parse_text("# Heading 1\n\n");
        assert_eq!(result[0]["type"], "heading");
        assert_eq!(result[0]["level"], "1");
        assert_eq!(result[0]["content"], "Heading 1");
    }

    #[test]
    fn test_lheading() {
        let result = parse_text("Heading 1\n=======\n\n");
        assert_eq!(result[0]["type"], "heading");
        assert_eq!(result[0]["level"], "1");
    }

    #[test]
    fn test_hr() {
        let result = parse_text("---\n\n");
        assert_eq!(result[0]["type"], "hr");
    }

    #[test]
    fn test_codeblock() {
        let result = parse_text("```js\ncode block\n```\n\n");
        assert_eq!(result[0]["type"], "codeBlock");
        assert_eq!(result[0]["content"], "code block");
        assert_eq!(result[0]["lang"], "js");
    }

    #[test]
    fn test_fence() {
        let result = parse_text("~~~js\ncode block\n~~~\n\n");
        assert_eq!(result[0]["type"], "codeBlock");
        assert_eq!(result[0]["content"], "code block");
        assert_eq!(result[0]["lang"], "js");
    }

    #[test]
    fn test_blockquote() {
        let result = parse_text("> quote\n\n");
        assert_eq!(result[0]["type"], "blockQuote");
    }

    #[test]
    fn test_list() {
        let result = parse_text("* item 1\n* item 2\n\n");
        assert_eq!(result[0]["type"], "list");
        assert_eq!(result[0]["ordered"], "false");
    }

    #[test]
    fn test_ordered_list() {
        let result = parse_text("1. item 1\n2. item 2\n\n");
        assert_eq!(result[0]["type"], "list");
        assert_eq!(result[0]["ordered"], "true");
    }

    #[test]
    fn test_link() {
        let result = parse_text("[text](https://example.com)");
        assert_eq!(result[0]["type"], "link");
        assert_eq!(result[0]["content"], "text");
        assert_eq!(result[0]["target"], "https://example.com");
    }

    #[test]
    fn test_image() {
        let result = parse_text("![alt](image.jpg)");
        assert_eq!(result[0]["type"], "image");
        assert_eq!(result[0]["alt"], "alt");
        assert_eq!(result[0]["target"], "image.jpg");
    }

    #[test]
    fn test_strong() {
        let result = parse_text("**bold**");
        assert_eq!(result[0]["type"], "strong");
        assert_eq!(result[0]["content"], "bold");
    }

    #[test]
    fn test_em() {
        let result = parse_text("*italic*");
        assert_eq!(result[0]["type"], "em");
        assert_eq!(result[0]["content"], "italic");
    }

    #[test]
    fn test_inline_code() {
        let result = parse_text("`code`");
        assert_eq!(result[0]["type"], "inlineCode");
        assert_eq!(result[0]["content"], "code");
    }

    #[test]
    fn test_url() {
        let result = parse_text("https://example.com");
        assert_eq!(result[0]["type"], "link");
        assert_eq!(result[0]["target"], "https://example.com");
    }

    #[test]
    fn test_autolink() {
        let result = parse_text("<https://example.com>");
        assert_eq!(result[0]["type"], "link");
        assert_eq!(result[0]["target"], "https://example.com");
    }

    #[test]
    fn test_mailto() {
        let result = parse_text("<user@example.com>");
        assert_eq!(result[0]["type"], "link");
        assert_eq!(result[0]["target"], "mailto:user@example.com");
    }

    #[test]
    fn test_paragraph() {
        let result = parse_text("This is a paragraph\n\n");
        assert_eq!(result[0]["type"], "paragraph");
        assert!(result[0]["content"].contains("This is a paragraph"));
    }

    #[test]
    fn test_escape() {
        let result = parse_text(r#"\*not italic\*"#);
        assert_eq!(result[0]["type"], "text");
        assert_eq!(result[0]["content"], "*");
    }

    #[test]
    fn test_del() {
        let result = parse_text("~~strikethrough~~");
        assert_eq!(result[0]["type"], "del");
        assert_eq!(result[0]["content"], "strikethrough");
    }

    #[test]
    fn test_multiple_elements() {
        let result = parse_text("# Heading\n\n**strong**\n_italic_\n\n");
        println!("{:?}", result);
        assert_eq!(result[0]["type"], "heading");
        assert_eq!(result[0]["content"], "Heading");
        assert_eq!(result[1]["type"], "newline");
        assert_eq!(result[2]["type"], "newline");
        assert_eq!(result[3]["type"], "strong");
        assert_eq!(result[3]["content"], "strong");
        assert_eq!(result[4]["type"], "newline");
        assert_eq!(result[5]["type"], "em");
        assert_eq!(result[5]["content"], "italic");
        assert_eq!(result[6]["type"], "newline");
        assert_eq!(result[7]["type"], "newline");
        assert_eq!(result.len(), 8);
    }
}
