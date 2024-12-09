use simple_markdown_rs::{create_default_rules, parser_for};

fn main() {
    let rules = create_default_rules();
    println!("Passing {} rules", rules.len());

    let parse = parser_for(rules);
    let result = parse(
        "example input **bold** https://anurl.com and some text",
        None,
    );
    println!("Parsing complete!");
    println!("Result: {:#?}", result);
}
