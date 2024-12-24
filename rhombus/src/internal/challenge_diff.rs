use std::collections::HashMap;
use std::hash::Hash;

use crate::grpc::proto::{
    AuthorPatch, CategoryPatch, ChallengeData, ChallengeDataPatch, ChallengePatch,
};

fn zip_maps<'a, K: Eq + Hash, V>(
    m1: &'a HashMap<K, V>,
    m2: &'a HashMap<K, V>,
) -> impl Iterator<Item = (&'a K, Option<&'a V>, Option<&'a V>)> {
    m1.iter()
        .map(move |(k, v1)| (k, Some(v1), m2.get(k)))
        .chain(
            m2.iter()
                .filter_map(|(k, v2)| (!m1.contains_key(k)).then_some((k, None, Some(v2)))),
        )
}

pub fn diff_challenge_data(old: &ChallengeData, new: &ChallengeData) -> ChallengeDataPatch {
    ChallengeDataPatch {
        challenges: zip_maps(&old.challenges, &new.challenges)
            .filter(|(_k, a, b)| a != b)
            .map(|(k, old, new)| {
                (
                    k.clone(),
                    ChallengePatch {
                        old: old.cloned(),
                        new: new.cloned(),
                    },
                )
            })
            .collect(),
        categories: zip_maps(&old.categories, &new.categories)
            .filter(|(_k, a, b)| a != b)
            .map(|(k, old, new)| {
                (
                    k.clone(),
                    CategoryPatch {
                        old: old.cloned(),
                        new: new.cloned(),
                    },
                )
            })
            .collect(),
        authors: zip_maps(&old.authors, &new.authors)
            .filter(|(_k, a, b)| a != b)
            .map(|(k, old, new)| {
                (
                    k.clone(),
                    AuthorPatch {
                        old: old.cloned(),
                        new: new.cloned(),
                    },
                )
            })
            .collect(),
    }
}
