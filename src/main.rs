use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::{Cursor, Read},
    iter, mem,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Error};
use colored::ColoredString;
use rand::seq::SliceRandom;

#[allow(unused)]
fn print_count(piles: &BTreeMap<String, Vec<usize>>, outcome: ColoredString, output_at_top: bool) {
    let mut counts = Vec::new();
    for (candidate, pile) in piles {
        if !pile.is_empty() {
            counts.push((pile.len(), candidate));
        }
    }
    counts.sort();

    //println!("Count {}: {}", count, outcome);
    if output_at_top {
        println!("{outcome}");
    }
    for (count, candidate) in counts.iter().rev() {
        println!("{:4}: {}", count, candidate);
    }
    if !output_at_top {
        println!("{outcome}");
    }
    println!();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Elected,
    Eliminated,
    Continuing,
}

type Ballot = [u8; 16];
fn run(seats: usize, ballots: &[Ballot], names: &[&str], ballot_ids: &[String]) -> Vec<u8> {
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
        if s == Status::Elected {
            elected.push(candidate as u8);
        }
    }
    elected.sort();
    elected
}

#[allow(unused)]
#[derive(Copy, Clone, PartialEq, Eq)]
enum Race {
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
struct BallotData {
    ballots: Vec<Vec<[u8; 16]>>,
    ballot_ids: Vec<Vec<String>>,
    precints: Vec<String>,
    candidates: Vec<String>,
    seats: usize,
}

fn parse_ballots(zipfile: &[u8], race: Race) -> Result<BallotData, Error> {
    let mut archive = zip::ZipArchive::new(Cursor::new(zipfile))?;

    let mut chp = None;
    let mut prms = HashMap::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        if !entry.is_file() || !entry.name().contains(race.to_str()) {
            continue;
        }

        let name = entry.name().split('/').last().unwrap().to_string();
        if name.ends_with(".PRM") {
            let mut data = String::new();
            entry.take(1 << 20).read_to_string(&mut data)?;
            prms.insert(name, data);
        } else if name.ends_with(".chp") {
            let mut data = String::new();
            entry.take(1 << 20).read_to_string(&mut data)?;
            chp = Some(data);
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
                ballot_data.seats = record.parse()?;
            }
            ".BALLOT-FORMAT-SEPS" => {
                seperators = record.as_bytes().to_vec();
            }
            ".CANDIDATE" => {
                let record = record
                    .split_once(", ")
                    .context("Failed to parse candidate def")?;
                candidate_ids.push(record.0.to_string());
                ballot_data
                    .candidates
                    .push(record.1.trim_matches('"').to_string());
            }
            ".INCLUDE" => {
                includes.push(record.to_string());
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
                .strip_suffix(".PRM")
                .unwrap_or("unknown")
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

fn main() -> Result<(), Error> {
    //let data = include_str!("../council.csv");

    colored::control::set_override(true);

    // let mut _empty_ballots = 0;
    // let mut ballots = Vec::new();
    // for line in data.lines().skip(1) {
    //     let columns = line.split(',').collect::<Vec<_>>();
    //     let mut ballot = columns[3..].to_vec();
    //     ballot.retain(|&x| !x.is_empty() /*&& x != "Sobrinho-Wheeler" */);
    //     // if ballot[0] == "Pierre" {
    //     //     ballot = vec!["Carlone"];
    //     //     n+=1;
    //     // }
    //     // if ballot.len() >= 15 {
    //     //     println!("{}: {:?}", ballot.len(), ballot);
    //     // }
    //     if !ballot.is_empty() {
    //         ballots.push(ballot);
    //     } else {
    //         _empty_ballots += 1;
    //     }
    // }

    let ballot_data = parse_ballots(include_bytes!("../ballots2023.zip"), Race::School)?;

    // let reverse_candidate_mapping = iter::once("Exhausted")
    //     .chain(
    //         ballots
    //             .iter()
    //             .flatten()
    //             .map(|&x| x)
    //             .collect::<HashSet<_>>()
    //             .into_iter(),
    //     )
    //     .collect::<Vec<_>>();
    // let candidate_mapping = reverse_candidate_mapping
    //     .iter()
    //     .enumerate()
    //     .map(|(i, &x)| (x, i))
    //     .collect::<HashMap<_, _>>();

    // let ballots: Vec<[u8; 16]> = ballots
    //     .iter()
    //     .map(|ballot| {
    //         let mut ballot = ballot
    //             .iter()
    //             .map(|&x| candidate_mapping[&x] as u8)
    //             .collect::<Vec<_>>();
    //         ballot.resize(16, 0);
    //         ballot.try_into().unwrap()
    //     })
    //     .collect();

    let reverse_candidate_mapping = iter::once("Exhausted")
        .chain(ballot_data.candidates.iter().map(|x| x.as_str()))
        .collect::<Vec<_>>();
    let ballots = ballot_data
        .ballots
        .iter()
        .cloned()
        .flatten()
        .collect::<Vec<_>>();

    let ballot_ids = ballot_data
        .ballot_ids
        .iter()
        .cloned()
        .flatten()
        .collect::<Vec<_>>();

    const THREADS: usize = 20;
    const ITERS: usize = 5000;

    let normal = run(
        ballot_data.seats,
        &ballots,
        &reverse_candidate_mapping,
        &ballot_ids,
    );

    let councils = Arc::new(Mutex::new(HashMap::new()));
    let mut joins = Vec::new();
    for _ in 0..THREADS {
        let normal = normal.clone();
        let mut ballots = ballot_data.ballots.clone();
        let councils = councils.clone();
        joins.push(std::thread::spawn(move || {
            for _ in 0..ITERS {
                for b in &mut ballots {
                    b.shuffle(&mut rand::thread_rng());

                }
                let mut ballots = ballots.iter().cloned().flatten().collect::<Vec<_>>();
                let result = run(ballot_data.seats, &ballots, &[], &[]);
                if result != normal {
                    *councils.lock().unwrap().entry(result).or_insert(0) += 1;
                }
            }
        }));
    }
    for j in joins {
        j.join().unwrap();
    }

    println!();
    let mut sum = 0;
    for (council, n) in councils.lock().unwrap().iter() {
        println!(
            "{:.4}%: {}",
            100.0 * (*n as f32) / (ITERS * THREADS) as f32,
            council
                .iter()
                .map(|&x| reverse_candidate_mapping[x as usize])
                .collect::<Vec<_>>()
                .join(", ")
        );
        sum += n;
    }
    println!(
        "{:.4}%: {}",
        100.0 * ((ITERS * THREADS - sum) as f32) / (ITERS * THREADS) as f32,
        normal
            .iter()
            .map(|&x| reverse_candidate_mapping[x as usize])
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(())
}
