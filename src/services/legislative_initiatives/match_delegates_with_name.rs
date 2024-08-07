pub fn levenshtein_distance(lhs: &[char], rhs: &[char], m: usize, n: usize) -> usize {
    if lhs == rhs {
        return 0;
    }
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }
    if lhs[m - 1] == rhs[n - 1] {
        levenshtein_distance(lhs, rhs, m - 1, n - 1)
    } else {
        1 + [
            levenshtein_distance(lhs, rhs, m, n - 1),
            levenshtein_distance(lhs, rhs, m - 1, n),
            levenshtein_distance(lhs, rhs, m - 1, n - 1),
        ]
        .iter()
        .min()
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::levenshtein_distance;

    #[test]
    fn test_calc_levensthein_distance_with_2_byte_thing() {
        let lhs = "abgegene";
        let rhs = "aöbgegebene";
        assert_eq!(
            levenshtein_distance(
                &lhs.chars().collect::<Vec<_>>(),
                &rhs.chars().collect::<Vec<_>>(),
                lhs.len(),
                rhs.len()
            ),
            3
        );
    }
    #[test]
    fn test_calc_levensthein_distance() {
        let lhs = "abgegene";
        let rhs = "abgegebene";

        assert_eq!(
            levenshtein_distance(
                &lhs.chars().collect::<Vec<_>>(),
                &rhs.chars().collect::<Vec<_>>(),
                lhs.len(),
                rhs.len()
            ),
            3
        );
    }
}
