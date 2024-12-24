use std::collections::HashMap;
use std::hash::Hash;

use crate::grpc::proto::{
    AuthorPatch, CategoryPatch, Challenge, ChallengeAttachment, ChallengeData, ChallengeDataPatch,
    ChallengePatch,
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

#[derive(PartialEq)]
struct ChallengeWithoutPoints<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub category: &'a str,
    pub author: &'a str,
    pub ticket_template: Option<&'a str>,
    pub files: &'a Vec<ChallengeAttachment>,
    pub flag: &'a str,
    pub healthscript: Option<&'a str>,
    pub score_type: Option<&'a str>,
}

fn without_points<'a>(chal: &'a Challenge) -> ChallengeWithoutPoints<'a> {
    ChallengeWithoutPoints {
        name: &chal.name,
        description: &chal.description,
        category: &chal.category,
        author: &chal.author,
        ticket_template: chal.ticket_template.as_deref(),
        files: &chal.files,
        flag: &chal.flag,
        healthscript: chal.healthscript.as_deref(),
        score_type: chal.score_type.as_deref(),
    }
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
            .filter(|(_k, patch)| {
                patch.old.as_ref().map(without_points) != patch.new.as_ref().map(without_points)
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
