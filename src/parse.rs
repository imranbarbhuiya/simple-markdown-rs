use fancy_regex::Regex;

use crate::default_rules::{ASTNode, ParserClosure, RuleMap, State};

lazy_static::lazy_static! {
    static ref CR_NEWLINE_R: Regex = Regex::new(r"\r\n?").unwrap();
    static ref TAB_R: Regex = Regex::new(r"\t").unwrap();
    static ref FORMFEED_R: Regex = Regex::new(r"\f").unwrap();
}

pub(crate) fn preprocess(source: &str) -> String {
    let result = CR_NEWLINE_R.replace_all(source, "\n");
    let result = FORMFEED_R.replace_all(&result, "");
    TAB_R.replace_all(&result, "    ").to_string()
}

pub fn parser_for(rules: RuleMap) -> ParserClosure {
    let mut rule_list: Vec<_> = rules
        .into_iter() // Transfer ownership of the rules into the iteration
        .filter(|(_, rule)| rule.match_fn.is_some())
        .collect();

    // Sort rules by order
    rule_list.sort_by(|(_, rule_a), (_, rule_b)| {
        let order_a = rule_a.order.unwrap_or(f64::MAX);
        let order_b = rule_b.order.unwrap_or(f64::MAX);
        order_a.partial_cmp(&order_b).unwrap()
    });

    // Return a boxed closure
    Box::new(move |source: &str, state: Option<State>| -> Vec<ASTNode> {
        let mut state = state.unwrap_or_default();
        let mut result = Vec::new();
        let mut remaining_source = preprocess(source);

        while !remaining_source.is_empty() {
            let mut matched = false;

            for (_rule_type, rule) in &rule_list {
                if let Some(match_fn) = rule.match_fn {
                    if let Some(capture) = match_fn(&remaining_source, &mut state) {
                        let matched_len = if let Some(capture_len) = rule.capture_len {
                            capture_len(&capture)
                        } else if !capture.is_empty() {
                            capture[0].len()
                        } else {
                            1
                        };

                        remaining_source = remaining_source[matched_len..].to_string();

                        if let Some(parse_fn) = rule.parse {
                            let parsed = parse_fn(capture, &mut state);
                            if !parsed.is_empty() {
                                result.push(parsed);
                            }
                        }
                        matched = true;
                        break;
                    }
                }
            }

            if !matched {
                // Take at least one character if nothing matches to avoid infinite loop
                remaining_source = remaining_source[1..].to_string();
            }
        }

        result
    })
}
