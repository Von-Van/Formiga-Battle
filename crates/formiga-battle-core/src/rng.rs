//! Every chance a match takes is a hash of the match seed and where in the match it is asked,
//! never a running generator. The same question always has the same answer, so a replay or a
//! server re-running a turn sees exactly what the players saw, and asking one more question
//! somewhere never shifts every answer after it.

/// What a roll is for, so two questions asked at the same moment never share an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum Purpose {
    TieBreak = 1,
    Impulse = 2,
    Opportunity = 3,
    Pairing = 4,
    Coach = 5,
    Roster = 6,
    /// Which found thing turns up, and on which pair of spots.
    Find = 7,
}

/// SplitMix64's finaliser: a good mix of 64 bits.
const fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A number from the seed, the purpose and whatever says where in the match it is asked.
pub fn roll(seed: u64, purpose: Purpose, parts: &[u64]) -> u64 {
    let mut value = mix(seed ^ (purpose as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    for part in parts {
        value = mix(value ^ mix(part.wrapping_add(0x9e37_79b9_7f4a_7c15)));
    }
    value
}

/// Whether something with a chance of `permille` in a thousand happens.
pub fn chance(seed: u64, purpose: Purpose, parts: &[u64], permille: u32) -> bool {
    (roll(seed, purpose, parts) % 1000) < u64::from(permille.min(1000))
}

/// FNV-1a over bytes: how a board's state is named for replay and desync checks.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_question_always_has_the_same_answer() {
        assert_eq!(
            roll(7, Purpose::TieBreak, &[1, 2]),
            roll(7, Purpose::TieBreak, &[1, 2])
        );
        assert_ne!(
            roll(7, Purpose::TieBreak, &[1, 2]),
            roll(7, Purpose::Impulse, &[1, 2])
        );
        assert_ne!(
            roll(7, Purpose::TieBreak, &[1, 2]),
            roll(7, Purpose::TieBreak, &[2, 1])
        );
    }

    #[test]
    fn a_chance_happens_about_as_often_as_it_says() {
        let hits = (0..10_000)
            .filter(|n| chance(42, Purpose::Impulse, &[*n], 250))
            .count();
        assert!((2_300..2_700).contains(&hits), "{hits}");
    }
}
