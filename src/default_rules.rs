use fancy_regex::Regex;
use std::collections::HashMap;

fn unescape_url(raw_url: &str) -> String {
    let re = Regex::new(r"\\([^\d\sA-Za-z])").unwrap();
    re.replace_all(raw_url, "$1").to_string()
}

lazy_static::lazy_static! {
    static ref LIST_BULLET: &'static str = r"(?:[*+-]|\d+\.)";
    static ref LIST_ITEM_PREFIX: &'static str = r"( *)(?:[*+-]|\d+\.) +";
    static ref LIST_ITEM_PREFIX_R: Regex = Regex::new(&format!(r"^{}", *LIST_ITEM_PREFIX)).unwrap();
    static ref LIST_ITEM_R: Regex = Regex::new(&format!(
        r"^{}[^\n]*(?:\n(?!{} )[^\n]*)*(\n|$)",
        *LIST_ITEM_PREFIX,
        *LIST_BULLET
    )).unwrap();
    static ref BLOCK_END_R: Regex = Regex::new(r"\n{2,}$").unwrap();
    static ref LIST_BLOCK_END_R: Regex = Regex::new(r"\n{2,}$").unwrap();
    static ref LIST_ITEM_END_R: Regex = Regex::new(r" *\n+$").unwrap();
    static ref LIST_R: Regex = Regex::new(&format!(
        r"^( *)({})[^\S\n][\s\S]+?(?:\n{{2,}}(?! )(?!\1{} )\n*|\s*\n*$)",
        *LIST_BULLET,
        *LIST_BULLET
    )).unwrap();
    static ref LIST_LOOKBEHIND_R: Regex = Regex::new(r"(?:^|\n)( *)$").unwrap();
    static ref INLINE_CODE_ESCAPE_BACKTICKS_R: Regex = Regex::new(r"^ (?= *`)|(` *) $").unwrap();
    static ref LINK_INSIDE: &'static str = r#"(?:\[[^\]]*\]|[^\[\]]|\](?=[^\[]*\]))*"#;
    static ref LINK_HREF_AND_TITLE: &'static str = r#"(?:\s*<(?:[^<>]|\\.)*>|\s*(?:[^\s\\\)]|\\.)*?)(?:\s+["']([^"']*)["'])?\s*"#;
    static ref AUTOLINK_MAILTO_CHECK_R: Regex = Regex::new("(?i)mailto:").unwrap();
}

pub type RuleMap = HashMap<String, Rule>;
pub type ASTNode = HashMap<String, String>;
pub type ParserClosure = Box<dyn Fn(&str, Option<State>) -> Vec<ASTNode>>;
pub type MatchFunction = fn(&str, &mut State) -> Option<Vec<String>>;
pub type ParseFunction = fn(Vec<String>, &mut State) -> ASTNode;
pub type CaptureLengthFunction = fn(&Vec<String>) -> usize;

#[derive(Debug)]
pub struct Rule {
    pub order: Option<f64>,
    pub match_fn: Option<MatchFunction>,
    pub parse: Option<ParseFunction>,
    pub capture_len: Option<CaptureLengthFunction>,
}

// Update State type to include boolean flags and refs
#[derive(Default, Clone)]
pub struct State {
    pub inline: bool,
    pub _list: bool,
    pub prev_capture: Option<String>,
    pub _refs: HashMap<String, Vec<ASTNode>>,
    pub _defs: HashMap<String, ASTNode>,
    pub data: HashMap<String, String>,
}

pub fn create_default_rules() -> HashMap<String, Rule> {
    let mut rules = HashMap::new();
    let mut curr_order = 0.0;

    rules.insert(
        "heading".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r"^ *(#{1,6})([^\n]+?)#* *(?:\n *)+\n").unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[1].to_string(), caps[2].trim().to_string()])
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "heading".to_string()),
                    ("level".to_string(), capture[0].len().to_string()),
                    ("content".to_string(), capture[1].trim().to_string()),
                ])
            }),
            capture_len: Some(|capture| 2 + capture[1].len()),
        },
    );

    rules.insert(
        "lheading".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r"^([^\n]+)\n *(=|-){3,} *(?:\n *)+\n").unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[1].to_string(), caps[2].to_string()])
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "heading".to_string()),
                    (
                        "level".to_string(),
                        if capture[1] == "=" { "1" } else { "2" }.to_string(),
                    ),
                    ("content".to_string(), capture[0].clone()),
                ])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "hr".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r"^( *[*_-]){3,} *(?:\n *)+\n").unwrap();
                regex.captures(source).ok().flatten().map(|_| vec![])
            }),
            parse: Some(|_capture, _state| HashMap::from([("type".to_string(), "hr".to_string())])),
            capture_len: Some(|_| 1),
        },
    );

    rules.insert(
        "codeBlock".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r"^(?: {4}[^\n]+\n*)+(?:\n *)+\n").unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[0].to_string()])
            }),
            parse: Some(|capture, _state| {
                let content = capture[0]
                    .replace(r"^ {4}", "")
                    .trim_end_matches('\n')
                    .to_string();
                HashMap::from([
                    ("type".to_string(), "codeBlock".to_string()),
                    ("content".to_string(), content),
                ])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "fence".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex =
                    Regex::new(r"^ *(`{3,}|~{3,}) *(?:(\S+) *)?\n([\S\s]+?)\n?\1 *(?:\n *)+\n")
                        .unwrap();
                regex.captures(source).ok().flatten().map(|caps| {
                    vec![
                        caps[1].to_string(),
                        caps[2].to_string(),
                        caps[3].to_string(),
                    ]
                })
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "codeBlock".to_string()),
                    ("lang".to_string(), capture[1].clone()),
                    ("content".to_string(), capture[2].clone()),
                ])
            }),
            capture_len: Some(|capture| 3 + capture[0].len() + capture[1].len() + capture[2].len()),
        },
    );

    rules.insert(
        "blockQuote".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r"^( *>[^\n]+(\n[^\n]+)*\n*)+\n{2,}").unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[0].to_string()])
            }),
            parse: Some(|capture, _state| {
                let content = capture[0].replace(r"^ *> ?", "");
                HashMap::from([
                    ("type".to_string(), "blockQuote".to_string()),
                    ("content".to_string(), content),
                ])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "list".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, state| {
                let prev_capture = state.prev_capture.as_deref().unwrap_or("").to_string();

                if let Ok(Some(lookbehind)) = LIST_LOOKBEHIND_R.captures(&prev_capture) {
                    if state._list || !state.inline {
                        let updated_source =
                            format!("{}{}", lookbehind.get(1).map_or("", |m| m.as_str()), source);
                        return LIST_R.captures(&updated_source).ok().flatten().map(|caps| {
                            vec![
                                caps[0].to_string(),
                                caps[1].to_string(),
                                caps[2].to_string(),
                            ]
                        });
                    }
                }
                None
            }),
            parse: Some(|capture, _state| {
                let bullet = &capture[2];
                let ordered = bullet.len() > 1;
                let start: Option<String> = if ordered {
                    bullet.trim_end_matches('.').parse().ok()
                } else {
                    None
                };

                let items_str = LIST_BLOCK_END_R.replace(&capture[0], "\n");
                let mut items = Vec::new();

                // Fix the list item matching to handle fancy_regex::Matches correctly
                let regex_matches = LIST_ITEM_R.find_iter(&items_str);
                for m in regex_matches.flatten() {
                    items.push(m.as_str().to_string());
                }

                HashMap::from([
                    ("type".to_string(), "list".to_string()),
                    ("ordered".to_string(), ordered.to_string()),
                    (
                        "start".to_string(),
                        start.map_or("".to_string(), |n| n.to_string()),
                    ),
                    ("items".to_string(), serde_json::to_string(&items).unwrap()),
                ])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "def".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(
                    r#"^ *\[([^\]]+)]: *<?([^\s>]*)>?(?: +["(]([^\n]+)[")])?(?:\n *)+$"#,
                )
                .unwrap();
                regex.captures(source).ok().flatten().map(|caps| {
                    vec![
                        caps[1].to_string(),
                        caps[2].to_string(),
                        caps.get(3)
                            .map_or("".to_string(), |m| m.as_str().to_string()),
                    ]
                })
            }),
            parse: Some(|capture, _state| {
                let def = capture[0].replace(r"\s+", " ").to_lowercase();
                let target = capture[1].clone();
                let title = capture[2].clone();

                HashMap::from([
                    ("type".to_string(), "def".to_string()),
                    ("def".to_string(), def),
                    ("target".to_string(), target),
                    ("title".to_string(), title),
                ])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "newline".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r"^(?:\n *)*\n").unwrap();
                regex.captures(source).ok().flatten().map(|_| vec![])
            }),
            parse: Some(|_capture, _state| {
                HashMap::from([("type".to_string(), "newline".to_string())])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "escape".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r"^\\([^\d\sA-Za-z])").unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[1].to_string()])
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "text".to_string()),
                    ("content".to_string(), capture[0].clone()),
                ])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "autolink".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r"^<([^ :>]+:\/[^ >]+)>").unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[1].to_string()])
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "link".to_string()),
                    ("content".to_string(), capture[0].clone()),
                    ("target".to_string(), capture[0].clone()),
                ])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "mailto".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r"^<([^ >]+@[^ >]+)>").unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[1].to_string()])
            }),
            parse: Some(|capture, _state| {
                let address = capture[0].clone();
                let target = if !AUTOLINK_MAILTO_CHECK_R.is_match(&address).unwrap_or(false) {
                    format!("mailto:{}", address)
                } else {
                    address.clone()
                };
                HashMap::from([
                    ("type".to_string(), "link".to_string()),
                    ("content".to_string(), address),
                    ("target".to_string(), target),
                ])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "url".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r#"^(https?:\/\/[^\s<]+[^:\s<"'\)\]}])"#).unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[0].to_string()])
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "link".to_string()),
                    ("content".to_string(), capture[0].clone()),
                    ("target".to_string(), capture[0].clone()),
                    ("inline".to_string(), "true".to_string()),
                ])
            }),
            capture_len: None,
        },
    );

    // Replace the link rule implementation
    rules.insert(
        "link".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(&format!(
                    r#"^\[({})]\(({})\)"#,
                    *LINK_INSIDE, *LINK_HREF_AND_TITLE
                ))
                .unwrap();
                regex.captures(source).ok().flatten().map(|caps| {
                    vec![
                        caps[0].to_string(),
                        caps[1].to_string(),
                        caps[2].to_string(),
                        caps.get(3)
                            .map_or("".to_string(), |m| m.as_str().to_string()),
                    ]
                })
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "link".to_string()),
                    ("content".to_string(), capture[1].clone()),
                    ("target".to_string(), unescape_url(&capture[2])),
                    ("title".to_string(), capture[3].clone()),
                ])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "image".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(&format!(
                    r#"^!\[({})]\(({})\)"#,
                    *LINK_INSIDE, *LINK_HREF_AND_TITLE
                ))
                .unwrap();
                regex.captures(source).ok().flatten().map(|caps| {
                    vec![
                        caps[0].to_string(),
                        caps[1].to_string(),
                        caps[2].to_string(),
                        caps.get(3)
                            .map_or("".to_string(), |m| m.as_str().to_string()),
                    ]
                })
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "image".to_string()),
                    ("alt".to_string(), capture[1].clone()),
                    ("target".to_string(), unescape_url(&capture[2])),
                    ("title".to_string(), capture[3].clone()),
                ])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "reflink".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(&format!(r"^\[({})]\s*\[([^\]]*)]", *LINK_INSIDE)).unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[1].to_string(), caps[2].to_string()])
            }),
            parse: Some(|capture, state| {
                if let Some(def) = state._defs.get(&capture[1].to_lowercase()) {
                    HashMap::from([
                        ("type".to_string(), "link".to_string()),
                        ("content".to_string(), capture[0].clone()),
                        ("target".to_string(), def["target"].clone()),
                        (
                            "title".to_string(),
                            def.get("title").cloned().unwrap_or_default(),
                        ),
                    ])
                } else {
                    HashMap::new() // Return empty if reference not found
                }
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "refimage".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(&format!(r"^!\[({})]\s*\[([^\]]*)]", *LINK_INSIDE)).unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[1].to_string(), caps[2].to_string()])
            }),
            parse: Some(|capture, state| {
                if let Some(def) = state._defs.get(&capture[1].to_lowercase()) {
                    HashMap::from([
                        ("type".to_string(), "image".to_string()),
                        ("alt".to_string(), capture[0].clone()),
                        ("target".to_string(), def["target"].clone()),
                        (
                            "title".to_string(),
                            def.get("title").cloned().unwrap_or_default(),
                        ),
                    ])
                } else {
                    HashMap::new() // Return empty if reference not found
                }
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "em".to_string(),
        Rule {
            order: Some({curr_order += 1.0; curr_order}),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r"^\b_((?:__|\\[\s\S]|[^\\_])+?)_\b|^\*(?=\S)((?:\*\*|\\[\s\S]|\s+(?:\\[\s\S]|[^\s\*\\]|\*\*)|[^\s\*\\])+?)\*(?!\*)").unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps.get(2).or_else(|| caps.get(1)).unwrap().as_str().to_string(), caps.get(0).unwrap().as_str().to_string()])
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "em".to_string()),
                    ("content".to_string(), capture[0].clone()),
                    ("inline".to_string(), "true".to_string()),
                ])
            }),
            capture_len: Some(|capture| capture[1].len()),
        },
    );

    rules.insert(
        "strong".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r"^(\*\*|__)((?:\\[\s\S]|[^\\]|(?!\1)[*_])+?)\1").unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[0].to_string(), caps[2].to_string()])
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "strong".to_string()),
                    ("content".to_string(), capture[1].clone()),
                ])
            }),
            capture_len: None,
        },
    );

    // Replace the u rule implementation
    rules.insert(
        "u".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r"^__((?:\\[\S\s]|[^\\])+?)__(?!_)").unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[0].to_string(), caps[1].to_string()])
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "u".to_string()),
                    ("content".to_string(), capture[1].clone()),
                    ("inline".to_string(), "true".to_string()),
                ])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "del".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex =
                    Regex::new(r"^~~(?=\S)((?:\\[\S\s]|~(?!~)|[^\s\\~]|\s(?!~~))+?)~~").unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[1].to_string()])
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "del".to_string()),
                    ("content".to_string(), capture[0].clone()),
                    ("inline".to_string(), "true".to_string()),
                ])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "inlineCode".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r"^(`+)([\S\s]*?[^`])\1(?!`)").unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[0].to_string(), caps[2].to_string()])
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "inlineCode".to_string()),
                    ("content".to_string(), capture[1].clone()),
                ])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "br".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex = Regex::new(r"^ {2,}\n").unwrap();
                regex.captures(source).ok().flatten().map(|_| vec![])
            }),
            parse: Some(|_capture, _state| HashMap::from([("type".to_string(), "br".to_string())])),
            capture_len: None,
        },
    );

    rules.insert(
        "paragraph".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, state| {
                if state.inline {
                    return None;
                }

                let regex = Regex::new(r"^((?:[^\n]|\n(?! *\n))+)(?:\n *)+\n").unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[1].to_string()])
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "paragraph".to_string()),
                    ("content".to_string(), capture[0].clone()),
                ])
            }),
            capture_len: None,
        },
    );

    rules.insert(
        "text".to_string(),
        Rule {
            order: Some({
                curr_order += 1.0;
                curr_order
            }),
            match_fn: Some(|source, _state| {
                let regex =
                    Regex::new(r"^[\S\s]+?(?=[^\d\sA-Za-z\u00C0-\uFFFF]|\n\n| {2,}\n|\w+:\S|$)")
                        .unwrap();
                regex
                    .captures(source)
                    .ok()
                    .flatten()
                    .map(|caps| vec![caps[0].to_string()])
            }),
            parse: Some(|capture, _state| {
                HashMap::from([
                    ("type".to_string(), "text".to_string()),
                    ("content".to_string(), capture[0].clone()),
                    ("inline".to_string(), "true".to_string()),
                ])
            }),
            capture_len: None,
        },
    );

    rules
}
