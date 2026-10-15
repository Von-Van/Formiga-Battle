//! Invite codes: six letters and digits a friend can read aloud or type, from an alphabet with
//! nothing that looks like anything else in it. A code opens one room and nothing more; it is
//! never proof of who anyone is.

/// No 0 or O, 1 or I or L, 2 or Z, 5 or S, 8 or B.
const ALPHABET: &[u8] = b"34679ACDEFGHJKMNPQRTUVWXY";
pub const LENGTH: usize = 6;

pub fn generate(entropy: &mut dyn FnMut() -> u64) -> String {
    let mut bits = entropy();
    (0..LENGTH)
        .map(|_| {
            let c = ALPHABET[(bits % ALPHABET.len() as u64) as usize] as char;
            bits /= ALPHABET.len() as u64;
            c
        })
        .collect()
}

/// A code as typed, read generously: any case, spaces and dashes ignored. `None` if it could not
/// be a code at all.
pub fn normalize(typed: &str) -> Option<String> {
    let code: String = typed
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    (code.len() == LENGTH && code.bytes().all(|b| ALPHABET.contains(&b))).then_some(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_is_read_back_however_it_is_typed() {
        let mut n = 12345u64;
        let code = generate(&mut || {
            n = n.wrapping_mul(6364136223846793005).wrapping_add(1);
            n
        });
        assert_eq!(code.len(), LENGTH);
        let typed = format!(" {}-{} ", code[..3].to_lowercase(), &code[3..]);
        assert_eq!(normalize(&typed), Some(code));
        assert_eq!(normalize("ABC"), None);
        assert_eq!(normalize("O0O0O0"), None);
    }
}
