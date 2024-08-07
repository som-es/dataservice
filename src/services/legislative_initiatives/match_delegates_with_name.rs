use sqlx::PgPool;

const MAX_STEPS: usize = 10000000;

pub fn levenshtein_distance(lhs: &str, rhs: &str) -> usize {
    let mut steps = 0;
    fn levenshtein_distance_inner_inner(lhs: &[char], rhs: &[char], m: usize, n: usize, steps: &mut usize) -> usize {
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
            levenshtein_distance_inner(lhs, rhs, m - 1, n - 1, steps)
        } else {
            1 + [
                levenshtein_distance_inner(lhs, rhs, m, n - 1, steps),
                levenshtein_distance_inner(lhs, rhs, m - 1, n, steps),
                levenshtein_distance_inner(lhs, rhs, m - 1, n - 1, steps),
            ]
            .iter()
            .min()
            .unwrap()
        }
    }
    fn levenshtein_distance_inner(lhs: &[char], rhs: &[char], m: usize, n: usize, steps: &mut usize) -> usize {
        if *steps >= MAX_STEPS {
            return usize::MAX;
        }
        let dist = levenshtein_distance_inner_inner(lhs, rhs, m, n, steps);
        *steps+=1;
        dist
    }
    let lhs = lhs.chars().collect::<Vec<_>>();
    let rhs = rhs.chars().collect::<Vec<_>>();
    levenshtein_distance_inner(&lhs, &rhs, lhs.len(), rhs.len(), &mut steps)
}

pub fn match_delegate_name_with_delegate_id(
    delegates: &[(String, String, usize)],
    delegate_name: &str,
) -> Vec<(usize, usize, String, String)> {
    let delegate_split = delegate_name.split(' ').collect::<Vec<_>>();
    let mut levenshtein_scores = delegates
        .iter()
        .cloned()
        .map(|(firstname, surname, delegate_id)| {
            if delegate_name == surname {
                return (0, delegate_id, firstname, surname);
            }

            let fullname = format!("{surname} {firstname}");
            if delegate_split.len() > 1 {
                (
                    levenshtein_distance(&fullname, delegate_name),
                    delegate_id,
                    firstname,
                    surname,
                )
            } else {
                (
                    levenshtein_distance(&surname, delegate_name),
                    delegate_id,
                    firstname,
                    surname,
                )
            }
        })
        .collect::<Vec<_>>();

    levenshtein_scores.sort_by(|a, b| a.0.cmp(&b.0));
    levenshtein_scores
}

pub fn match_delegate_names_with_delegate_ids(pg: &PgPool, delegate_names: &[String]) {}

#[cfg(test)]
mod tests {
    use super::{levenshtein_distance, match_delegate_name_with_delegate_id};

    #[test]
    fn test_distance_delegate_match() {
        let delegates = vec![
            ("Amon Werner", 1),
            ("Belakowitsch Dagmar", 2),
            ("Berlakovich Nikolaus", 3),
            ("Bacher Walter", 4),
            ("Bacher Stefan", 4),
            ("Becher Ruth", 5),
            ("Duzdar Muna", 6),
        ]
        .iter()
        .map(|(name, id)| {
            let name = name.split(' ').collect::<Vec<_>>();
            let firstname = name[1].to_string();
            let surname = name[0].to_string();
            (firstname, surname, *id)
        })
        .collect::<Vec<_>>();
        let delegate_name = "Bacher Stefono";
        let scores = match_delegate_name_with_delegate_id(&delegates, delegate_name);
        print!("scores: {scores:?}");
    }

    #[test]
    fn test_calc_levensthein_distance_with_2_byte_thing() {
        let lhs = "abgegene";
        let rhs = "aöbgegebene";

        assert_eq!(levenshtein_distance(&lhs, &rhs), 3);
    }
    #[test]
    fn test_calc_levensthein_distance() {
        let lhs = "abgegene";
        let rhs = "abgegebene";

        assert_eq!(levenshtein_distance(lhs, rhs,), 2);
    }
}
