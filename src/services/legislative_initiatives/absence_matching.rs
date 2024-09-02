pub fn reverse_name(name: &str) -> String {
    let name = name.split_inclusive(' ').collect::<Vec<_>>();
    if name.len() > 1 {
        format!(
            "{} {}",
            name.last().unwrap(),
            name[..name.len() - 1]
                .iter()
                .fold(String::new(), |acc, x| format!("{acc}{x}"))
        )
        .trim_matches(char::is_whitespace)
        .to_string()
    } else {
        name[0].to_string()
    }
}
