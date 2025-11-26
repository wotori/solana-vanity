#[derive(Clone)]
pub struct PrefixRule {
    pub raw: String,
    pub bytes: Vec<u8>,
}

pub fn candidate_matches(candidate: &[u8], prefix: &[u8], ignore_case: bool) -> bool {
    if candidate.len() < prefix.len() {
        return false;
    }
    if !ignore_case {
        candidate[..prefix.len()] == prefix[..]
    } else {
        candidate[..prefix.len()]
            .iter()
            .zip(prefix.iter())
            .all(|(a, b)| eq_ignore_ascii_case(*a, *b))
    }
}

fn eq_ignore_ascii_case(a: u8, b: u8) -> bool {
    a == b || a.eq_ignore_ascii_case(&b)
}
