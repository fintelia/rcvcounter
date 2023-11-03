use std::{
    collections::{BTreeMap, HashMap, HashSet},
    iter, mem,
    sync::{Arc, Mutex},
};

use anyhow::Error;
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

// fn transfer_votes<'a, 'b>(
//     ballot: &'b mut Vec<&'a str>,
//     index: usize,
//     piles: &'b mut BTreeMap<String, Vec<usize>>,
//     exhausted: &'b mut Vec<usize>,
// ) -> bool {
//     while !ballot.is_empty() && !piles.contains_key(ballot[0]) {
//         ballot.remove(0);
//     }
//     if !ballot.is_empty() {
//         piles
//             .entry(ballot[0].to_string())
//             .or_insert_with(Vec::new)
//             .push(index);
//         true
//     } else {
//         exhausted.push(index);
//         false
//     }
// }

// fn simulate(mut ballots: Vec<Vec<&str>>) -> Vec<String> {
//     let quota = (ballots.len() + 9) / 10;
//     // println!("{:?} ballots. Quota is {}", ballots.len(), quota);
//     // println!("{} empty ballots", empty_ballots);

//     let mut piles = BTreeMap::new();
//     for (i, ballot) in ballots.iter().enumerate() {
//         piles
//             .entry(ballot[0].to_string())
//             .or_insert_with(Vec::new)
//             .push(i);
//     }

//     // print_count(&piles, "".into(), true);

//     let mut exhausted = Vec::new();

//     // Eliminate candidates with fewer than 50 first place votes
//     let mut eliminated = Vec::new();
//     for (candidate, pile) in &piles {
//         if pile.len() < 50 {
//             eliminated.push((candidate.to_string(), pile.len()));
//         }
//     }
//     let eliminated_piles = eliminated
//         .iter()
//         .map(|(candidate, _)| piles.remove(&*candidate).unwrap())
//         .collect::<Vec<_>>();
//     for pile in &eliminated_piles {
//         for &ballot in pile {
//             transfer_votes(&mut ballots[ballot], ballot, &mut piles, &mut exhausted);
//         }
//     }

//     // println!();
//     // print_count(&piles, "Eliminated candidates with fewer than 50 first place votes".underline(), true);

//     let mut rng = rand::thread_rng();

//     let mut elected = Vec::new();

//     // Transfer votes from candidates that made quota
//     while elected.len() < 9 {
//         let max_votes = piles.iter().map(|p| p.1.len()).max().unwrap();
//         if max_votes >= quota || piles.len() + elected.len() == 9 {
//             let mut first_place: Vec<_> = piles
//                 .iter()
//                 .filter(|p| p.1.len() == max_votes)
//                 .map(|p| p.0.to_string())
//                 .collect();
//             first_place.shuffle(&mut rng);

//             let elected_candidate = first_place.pop().unwrap();
//             elected.push(elected_candidate.clone());

//             let mut pile = piles.remove(&*elected_candidate).unwrap();
//             pile.shuffle(&mut rng);

//             let mut left_to_transfer = pile.len() - quota;
//             while !pile.is_empty() && left_to_transfer > 0 {
//                 let ballot = pile.pop().unwrap();
//                 if transfer_votes(&mut ballots[ballot], ballot, &mut piles, &mut exhausted) {
//                     left_to_transfer -= 1;
//                 }
//             }

//             // print_count(&piles, format!("{max_votes}: {elected_candidate}").bold(), true);

//             // let mut just_elected = Vec::new();
//             // for (candidate, pile) in &piles {
//             //     if pile.len() >= quota {
//             //         just_elected.push(candidate.to_owned());
//             //     }
//             // }
//             // let elected_piles = just_elected
//             //     .iter()
//             //     .map(|candidate| piles.remove(candidate).unwrap())
//             //     .collect::<Vec<_>>();
//             // elected.extend(just_elected);
//             // println!("Elected: {:?}", elected);

//             // for mut pile in elected_piles {
//             //     pile.shuffle(&mut rng);
//             //     for ballot in pile {
//             //         transfer_votes(&mut ballots[ballot], ballot, &mut piles, &mut exhausted)
//             //     }
//             // }
//         } else {
//             // Eliminate last place candidate
//             let min_votes = piles.iter().map(|p| p.1.len()).min().unwrap();
//             let mut last_place: Vec<_> = piles
//                 .iter()
//                 .filter(|p| p.1.len() == min_votes)
//                 .map(|p| p.0.to_string())
//                 .collect();
//             last_place.shuffle(&mut rng);

//             let eliminated_candidate = last_place.pop().unwrap();
//             let pile = piles.remove(&*eliminated_candidate).unwrap();
//             let votes = pile.len();
//             for ballot in pile {
//                 transfer_votes(&mut ballots[ballot], ballot, &mut piles, &mut exhausted);
//             }
//             // print_count(&piles, format!("{votes}: {eliminated_candidate}").strikethrough(), false);
//             eliminated.push((eliminated_candidate.to_owned(), votes));
//         }
//     }

//     // for (candidate, _) in elected {
//     //     println!("{}", format!("{:4}: {}", quota, candidate).bold());
//     // }
//     // for (candidate, votes) in eliminated.iter().rev() {
//     //     println!("{}", format!("{:4}: {}", votes, candidate).color(colored::Color::Red));
//     // }

//     // match eliminated.iter().find(|(candidate, _)| candidate == "Sobrinho-Wheeler") {
//     //     Some((_, votes)) => *votes,
//     //     None => panic!("Jivan Wins!"),
//     // }
//     elected.sort();
//     elected
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Elected,
    Eliminated,
    Continuing,
}

type Ballot = [u8; 16];
fn run(ballots: &[Ballot]) -> Vec<u8> {
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

    let quota = (ballots.len() + 9) / 10;

    // Round 1 winners
    for i in 1..256 {
        if piles[i].len() >= quota {
            // println!("{} elected", names[i]);
            status[i] = Status::Elected;
        }
    }

    // Remove over
    for i in 1..256 {
        if piles[i].len() > quota {
            status[i] = Status::Elected;

            let step = (piles[i].len() as f64 / quota as f64).round() as usize;

            let mut j = step - 1;
            while piles[i].len() > quota {
                for &recipient in &ballots[piles[i][j] as usize][1..] {
                    if status[recipient as usize] == Status::Continuing {
                        let ballot_index = piles[i].remove(j);
                        piles[recipient as usize].push(ballot_index);
                        if piles[recipient as usize].len() >= quota {
                            status[recipient as usize] = Status::Elected;
                        }
                        break;
                    }
                }

                j += step;
                if j >= piles[i].len() {
                    j -= piles[i].len() - 1;
                }
            }
        }
    }

    // Remove zero votes
    for i in 1..256 {
        if piles[i].is_empty() {
            status[i] = Status::Eliminated;
        }
    }

    loop {
        let candidates_remaining = status.iter().filter(|&&s| s != Status::Eliminated).count();
        if candidates_remaining <= 9 {
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

fn main() -> Result<(), Error> {
    let data = include_str!("../council.csv");

    colored::control::set_override(true);

    let mut _empty_ballots = 0;
    let mut ballots = Vec::new();
    for line in data.lines().skip(1) {
        let columns = line.split(',').collect::<Vec<_>>();
        let mut ballot = columns[3..].to_vec();
        ballot.retain(|&x| !x.is_empty() /*&& x != "Sobrinho-Wheeler" */);
        // if ballot[0] == "Pierre" {
        //     ballot = vec!["Carlone"];
        //     n+=1;
        // }
        // if ballot.len() >= 15 {
        //     println!("{}: {:?}", ballot.len(), ballot);
        // }
        if !ballot.is_empty() {
            ballots.push(ballot);
        } else {
            _empty_ballots += 1;
        }
    }

    // for _ in 0..250 {
    //     ballots.push(vec!["Sobrinho-Wheeler"]);
    // }
    // for _ in 0..250 {
    //     ballots.push(vec!["Carlone"]);
    // }
    // for _ in 0..250 {
    //     ballots.push(vec!["Williams"]);
    // }

    let reverse_candidate_mapping = iter::once("Exhausted")
        .chain(
            ballots
                .iter()
                .flatten()
                .map(|&x| x)
                .collect::<HashSet<_>>()
                .into_iter(),
        )
        .collect::<Vec<_>>();
    let candidate_mapping = reverse_candidate_mapping
        .iter()
        .enumerate()
        .map(|(i, &x)| (x, i))
        .collect::<HashMap<_, _>>();

    let ballots: Vec<[u8; 16]> = ballots
        .iter()
        .map(|ballot| {
            let mut ballot = ballot
                .iter()
                .map(|&x| candidate_mapping[&x] as u8)
                .collect::<Vec<_>>();
            ballot.resize(16, 0);
            ballot.try_into().unwrap()
        })
        .collect();

    const THREADS: usize = 20;
    const ITERS: usize = 5000;

    let normal = run(&ballots);

    let councils = Arc::new(Mutex::new(HashMap::new()));
    let mut joins = Vec::new();
    for _ in 0..THREADS {
        let normal = normal.clone();
        let mut ballots = ballots.clone();
        let councils = councils.clone();
        joins.push(std::thread::spawn(move || {
            for _ in 0..ITERS {
                ballots.shuffle(&mut rand::thread_rng());
                let result = run(&ballots);
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
