use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::{Cursor, Read},
    iter, mem,
};

use anyhow::{Context, Error};

use rand::seq::SliceRandom;
use wasm_bindgen::prelude::*;

const BALLOTS_2021: &[u8] = include_bytes!("../ballots2021.zip");
const BALLOTS_2023: &[u8] = include_bytes!("../ballots2023.zip");

fn election_to_ballot_data(election: &str) -> Result<BallotData, String> {
    match election {
        "sc23" => parse_ballots(BALLOTS_2023, Race::School),
        "cc23" => parse_ballots(BALLOTS_2023, Race::Council),
        "sc21" => parse_ballots(BALLOTS_2021, Race::School),
        "cc21" => parse_ballots(BALLOTS_2021, Race::Council),
        _ => return Err("ERROR: Unknown election".to_owned()),
    }
    .map_err(|e| e.to_string())
}

#[wasm_bindgen]
pub fn candidates(election: &str) -> Vec<String> {
    election_to_ballot_data(election)
        .map(|ballot_data| ballot_data.candidates)
        .unwrap_or_default()
}

#[wasm_bindgen]
pub fn precints(election: &str) -> Vec<String> {
    election_to_ballot_data(election)
        .map(|ballot_data| ballot_data.precints)
        .unwrap_or_default()
}

#[wasm_bindgen]
pub fn simulate(
    election: &str,
    shuffle_within_precints: bool,
    shuffle_precint_order: bool,
    extra_votes: usize,
    extra_candidate: String,
    extra_precint: String,
) -> String {
    let mut ballot_data = match election_to_ballot_data(election) {
        Ok(ballot_data) => ballot_data,
        Err(e) => return e,
    };

    let reverse_candidate_mapping = iter::once("Exhausted")
        .chain(ballot_data.candidates.iter().map(|x| x.as_str()))
        .collect::<Vec<_>>();

    // let ballot_ids = ballot_data
    //     .ballot_ids
    //     .iter()
    //     .cloned()
    //     .flatten()
    //     .collect::<Vec<_>>();

    if extra_votes > 0 {
        let Some(candidate_index) = reverse_candidate_mapping
            .iter()
            .position(|&x| x == extra_candidate)
        else {
            return format!("ERROR: Unknown candidate '{extra_candidate}'");
        };

        let Some(precint_index) = ballot_data
            .precints
            .iter()
            .position(|x| x == &*extra_precint)
        else {
            return format!("ERROR: Unknown precint '{extra_precint}'");
        };

        let mut ballot_contents = [0; 16];
        ballot_contents[0] = candidate_index as u8;

        for _ in 0..extra_votes {
            ballot_data.ballots[precint_index].push(ballot_contents);
        }
    }

    let normal = run(
        ballot_data.seats,
        &ballot_data
            .ballots
            .iter()
            .cloned()
            .flatten()
            .collect::<Vec<_>>(),
        &[], // &reverse_candidate_mapping,
        &[], // &ballot_ids,
    );

    const ITERS: usize = 5000;
    let mut other_outcomes = HashMap::new();
    if shuffle_within_precints || shuffle_precint_order || extra_votes > 0 {
        for _ in 0..ITERS {
            let mut ballots = ballot_data.ballots.clone();

            if shuffle_precint_order {
                ballots.shuffle(&mut rand::thread_rng());
            }

            if shuffle_within_precints {
                for b in &mut ballots {
                    b.shuffle(&mut rand::thread_rng());
                }
            }

            let ballots = ballots.iter().cloned().flatten().collect::<Vec<_>>();
            let result = run(ballot_data.seats, &ballots, &[], &[]);
            if result != normal {
                *other_outcomes.entry(result).or_insert(0) += 1;
            }
        }
    }

    let mut output = String::new();
    output.push_str(&format!(
        "<p class=\"mb-1\"><strong>Seats:</strong> {}</p>",
        ballot_data.seats
    ));
    output.push_str(&format!(
        "<p class=\"mb-1\"><strong>Ballots:</strong> {}\n</p>",
        ballot_data.ballots.iter().flatten().count()
    ));

    if other_outcomes.is_empty() {
        output.push_str("<p class=\"mb-4\"><strong>Winners:</strong> ");
        output.push_str(
            &normal
                .iter()
                .map(|&x| reverse_candidate_mapping[x as usize])
                .collect::<Vec<_>>()
                .join(", "),
        );
        if shuffle_within_precints || shuffle_precint_order || extra_votes > 0 {
            output.push_str("<p>No other outcomes found in 5000 runs.</p>");
        }
        return output;
    }

    let undefeated: HashSet<_> = normal
        .iter()
        .copied()
        .filter(|&x| other_outcomes.keys().all(|y| y.contains(&x)))
        .collect();

    output.push_str("<p class=\"mb-4\"><strong>Undefeated:</strong> ");
    output.push_str(
        &undefeated
            .iter()
            .map(|&x| reverse_candidate_mapping[x as usize])
            .collect::<Vec<_>>()
            .join(", "),
    );

    let normal_count = ITERS - other_outcomes.values().sum::<usize>();
    let outcomes: BTreeMap<_, _> = other_outcomes
        .into_iter()
        .map(|(k, v)| (v, (false, k)))
        .chain(iter::once((normal_count, (true, normal.clone()))))
        .collect();

    for (count, (is_normal, candidates)) in outcomes.into_iter().rev() {
        let candidates = candidates
            .iter()
            .filter(|x| !undefeated.contains(x))
            .map(|&x| reverse_candidate_mapping[x as usize])
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!(
            "<p class=\"mb-1{}\">{:.2}%: {}</p>",
            if is_normal { " fw-bold" } else { "" },
            100.0 * (count as f32) / ITERS as f32,
            candidates
        ));
    }

    output
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Elected,
    Eliminated,
    Continuing,
}

type Ballot = [u8; 16];
pub fn run(seats: usize, ballots: &[Ballot], _names: &[&str], _ballot_ids: &[String]) -> Vec<u8> {
    let mut piles: [Vec<_>; 256] = (0..256)
        .map(|_| Vec::new())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let mut status = [Status::Continuing; 256];
    status[0] = Status::Eliminated;

    for (i, ballot) in ballots.iter().enumerate() {
        piles[ballot[0] as usize].push(i as u32);
    }

    // Vote totals at the end of each round. Used to break ties.
    let mut round_totals: Vec<Vec<usize>> = Vec::new();
    round_totals.push(piles.iter().map(Vec::len).collect());

    let quota = (ballots.len() + seats) / (seats + 1);
    // println!("Quota: {}", quota);

    // Round 1 winners
    for i in 1..256 {
        if piles[i].len() >= quota {
            // println!("{} elected", names[i]);
            status[i] = Status::Elected;
        }
    }

    // println!("Quota: {}", quota);

    // Remove over
    let mut order = (1..256)
        .filter(|&i| piles[i].len() > quota)
        .collect::<Vec<_>>();
    order.sort_by_key(|&i| piles[i].len());
    for i in order.into_iter().rev() {
        let step = (piles[i].len() as f64 / (piles[i].len() - quota) as f64).round() as usize;
        // println!(
        //     "{} over quota by {} votes",
        //     names[i],
        //     piles[i].len() - quota
        // );
        // println!("Step: {}", step);

        let mut num_removed = 0;
        let mut removed = vec![false; piles[i].len()];

        let mut start_index = step - 1;
        let mut j = start_index;
        while piles[i].len() - num_removed > quota {
            for &recipient in &ballots[piles[i][j] as usize][1..] {
                if status[recipient as usize] == Status::Continuing {
                    let ballot_index = piles[i][j];
                    piles[recipient as usize].push(ballot_index);
                    if piles[recipient as usize].len() >= quota {
                        status[recipient as usize] = Status::Elected;
                    }
                    removed[j] = true;
                    num_removed += 1;
                    // println!(
                    //     "   {}:  {} --> {}",
                    //     ballot_ids[ballot_index as usize], names[i], names[recipient as usize]
                    // );
                    break;
                }
            }
            j += step;
            if j >= piles[i].len() {
                start_index += 1;
                j = start_index;
                //j -= piles[i].len() / step * step - 1;
            }
        }
        piles[i] = piles[i]
            .iter()
            .enumerate()
            .filter(|(j, _)| !removed[*j])
            .map(|(_, &x)| x)
            .collect();

        // for i in 1..18 {
        //     if status[i] != Status::Eliminated {
        //         println!("{:4} votes: {}", piles[i].len(), names[i]);
        //     }
        // }
        // println!();
    }

    // Remove zero votes
    for i in 1..256 {
        if piles[i].is_empty() {
            status[i] = Status::Eliminated;
        }
    }

    loop {
        let candidates_remaining = status.iter().filter(|&&s| s != Status::Eliminated).count();
        if candidates_remaining <= seats {
            break;
        }

        round_totals.push(piles.iter().map(Vec::len).collect());

        // Eliminate last place candidate
        let mut last_candidate = 0;
        let mut min_votes = usize::MAX;
        for (candidate, ballots) in piles
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(i, _)| status[*i] == Status::Continuing)
        {
            if ballots.len() < min_votes {
                min_votes = ballots.len();
                last_candidate = candidate;
            } else if ballots.len() == min_votes {
                for round in round_totals.iter().rev() {
                    if round[candidate] < round[last_candidate] {
                        last_candidate = candidate;
                        break;
                    } else if round[candidate] > round[last_candidate] {
                        break;
                    }
                }
            }
        }

        // Transfer votes
        status[last_candidate] = Status::Eliminated;
        // println!(
        //     "{:4} votes: {} eliminated",
        //     min_votes, names[last_candidate]
        // );

        // if candidates_remaining == 11 {
        //     let mut success_lengths = [0; 16];
        //     let mut fail_lengths = [0; 16];

        //     for ballot in &piles[0] {
        //         let length = ballots[*ballot as usize]
        //             .iter()
        //             .filter(|&&x| x != 0)
        //             .count();
        //         fail_lengths[length] += 1;
        //         if length >= 7 {
        //             println!(
        //                 "{}: {}",
        //                 length,
        //                 ballots[*ballot as usize]
        //                     .iter()
        //                     .map(|x| names[*x as usize])
        //                     .collect::<Vec<_>>()
        //                     .join(", ")
        //             );
        //         }
        //     }

        //     for pile in &piles[1..] {
        //         for ballot in pile {
        //             let length = ballots[*ballot as usize]
        //                 .iter()
        //                 .filter(|&&x| x != 0)
        //                 .count();
        //             success_lengths[length] += 1;
        //         }
        //     }
        //     for i in 1..16 {
        //         println!(
        //             "{}: {:.1}% ({}/{})",
        //             i,
        //             100.0 * success_lengths[i] as f32
        //                 / (success_lengths[i] + fail_lengths[i]) as f32,
        //             fail_lengths[i],
        //             (success_lengths[i] + fail_lengths[i])
        //         );
        //     }
        //     println!("total failed = {}", fail_lengths.iter().sum::<usize>());
        // }

        for ballot_index in mem::take(&mut piles[last_candidate]) {
            let ballot = &ballots[ballot_index as usize];
            let mut i = 0;
            while ballot[i] != 0 && status[ballot[i] as usize] != Status::Continuing {
                i += 1;
            }

            piles[ballot[i] as usize].push(ballot_index);
            if ballot[i] != 0 && piles[ballot[i] as usize].len() >= quota {
                status[ballot[i] as usize] = Status::Elected;
            }
        }
    }

    let mut elected = Vec::new();
    for (candidate, &s) in status.iter().enumerate() {
        if s != Status::Eliminated {
            elected.push(candidate as u8);
        }
    }

    elected.sort();
    elected
}

#[allow(unused)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Race {
    School,
    Council,
}
impl Race {
    fn to_str(&self) -> &'static str {
        match *self {
            Race::School => "School",
            Race::Council => "Council",
        }
    }
}

#[derive(Default)]
pub struct BallotData {
    pub ballots: Vec<Vec<Ballot>>,
    pub ballot_ids: Vec<Vec<String>>,
    pub precints: Vec<String>,
    pub candidates: Vec<String>,
    pub seats: usize,
}

pub fn parse_ballots(zipfile: &[u8], race: Race) -> Result<BallotData, Error> {
    let mut archive = zip::ZipArchive::new(Cursor::new(zipfile))?;

    let mut chp = None;
    let mut prms = HashMap::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        if !entry.is_file() || !entry.name().contains(race.to_str()) {
            continue;
        }

        let name = entry.name().split('/').last().unwrap().to_lowercase();
        if name.ends_with(".prm") {
            let mut data = String::new();
            entry.take(1 << 20).read_to_string(&mut data)?;
            prms.insert(name, data);
        } else if name.ends_with(".chp") {
            let mut data = Vec::new();
            entry.take(1 << 20).read_to_end(&mut data)?;

            // Parse the data as utf-8, or Latin-1 if that fails.
            chp = Some(String::from_utf8(data).unwrap_or_else(|e| {
                e.into_bytes()
                    .into_iter()
                    .map(|x| x as char)
                    .collect::<String>()
            }));
        }
    }

    let mut ballot_data = BallotData::default();

    let mut seperators = b",)".to_vec();
    let mut candidate_ids = Vec::new();
    let mut includes = Vec::new();

    let chp = chp.context("missing .chp file")?;
    for line in chp.lines() {
        let Some((command, record)) = line.split_once(char::is_whitespace) else {
            continue;
        };

        match command {
            ".ELECT" => {
                ballot_data.seats = record.trim().parse()?;
            }
            ".BALLOT-FORMAT-SEPS" => {
                seperators = record.as_bytes().to_vec();
            }
            ".CANDIDATE" => {
                let record = record
                    .split_once(", ")
                    .context("Failed to parse candidate def")?;
                candidate_ids.push(record.0.to_string());

                let mut candidate_name = record.1.trim().trim_matches('"').to_string();
                if let Some((last, first)) = candidate_name.split_once(", ") {
                    candidate_name = format!("{first} {last}");
                }
                ballot_data.candidates.push(candidate_name);
            }
            ".INCLUDE" => {
                includes.push(record.trim().to_lowercase());
            }
            _ => {}
        }
    }

    let candidate_id_to_index = candidate_ids
        .into_iter()
        .enumerate()
        .map(|(i, x)| (x, 1 + i as u8))
        .collect::<HashMap<_, _>>();

    for include in includes {
        let prm = prms
            .remove(include.trim())
            .context(format!("missing .PRM file for {include}"))?;

        ballot_data.precints.push(
            include
                .strip_suffix(".prm")
                .unwrap_or(include.as_str())
                .to_string(),
        );

        let mut ballot_ids = Vec::new();
        let mut ballots = Vec::new();
        for line in prm.lines() {
            if line.is_empty() {
                continue;
            }
            let (ballot_id, rankings) = line
                .split_once(char::from(seperators[1]))
                .context("Failed to parse ballot")?;

            let ballot_id = ballot_id
                .split_once(char::from(seperators[0]))
                .context("Failed to parse ballot id")?
                .0
                .to_string();

            let mut ballot = Vec::new();
            for candidate in rankings.trim().split(char::from(seperators[0])) {
                if candidate.is_empty() || candidate.contains("=") {
                    continue;
                }
                let candidate = candidate
                    .split_once('[')
                    .context("Failed to parse candidate")?
                    .0;
                ballot.push(
                    *candidate_id_to_index
                        .get(candidate)
                        .with_context(|| format!("Unknown candidate '{candidate}'"))?,
                );
            }
            if !ballot.is_empty() {
                ballot.resize(16, 0);
                ballots.push(ballot.try_into().unwrap());
                ballot_ids.push(ballot_id);
            }
        }
        ballot_data.ballot_ids.push(ballot_ids);
        ballot_data.ballots.push(ballots);
    }

    Ok(ballot_data)
}
