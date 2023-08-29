use std::collections::BTreeMap;

use anyhow::Error;
use rand::seq::SliceRandom;

fn print_count(count: usize, piles: &BTreeMap<&str, Vec<usize>>, outcome: &str) {
    let mut counts = Vec::new();
    for (candidate, pile) in piles {
        if !pile.is_empty() {
            counts.push((pile.len(), candidate));
        }
    }
    counts.sort();

    println!("Count {}: {}", count, outcome);
    for (count, candidate) in counts.iter().rev() {
        println!("{:4}: {}", count, candidate);
    }
    println!();
}

fn transfer_votes<'a, 'b>(
    ballot: &'b mut Vec<&'a str>,
    index: usize,
    piles: &'b mut BTreeMap<&'a str, Vec<usize>>,
    exhausted: &'b mut Vec<usize>,
) -> bool {
    while !ballot.is_empty() && !piles.contains_key(ballot[0]) {
        ballot.remove(0);
    }
    if !ballot.is_empty() {
        piles.entry(ballot[0]).or_insert_with(Vec::new).push(index);
        true
    } else {
        exhausted.push(index);
        false
    }
}

fn main() -> Result<(), Error> {
    let data = include_str!("../council.csv");

    let mut empty_ballots = 0;
    let mut ballots = Vec::new();
    for line in data.lines().skip(1) {
        let columns = line.split(',').collect::<Vec<_>>();
        let mut ballot = columns[3..].to_vec();
        ballot.retain(|&x| !x.is_empty() && x != "Mallon" && x != "Carlone" && x != "Zondervan");
        if !ballot.is_empty() {
            ballots.push(ballot);
        } else {
            empty_ballots += 1;
        }
    }

    let quota = (ballots.len() + 9) / 10;
    println!("{:?} ballots. Quota is {}", ballots.len(), quota);
    println!("{} empty ballots", empty_ballots);

    let mut piles = BTreeMap::new();
    for (i, ballot) in ballots.iter().enumerate() {
        piles.entry(ballot[0]).or_insert_with(Vec::new).push(i);
    }

    print_count(0, &piles, "");

    let mut exhausted = Vec::new();

    // Eliminate candidates with fewer than 50 first place votes
    let mut eliminated = Vec::new();
    for (candidate, pile) in &piles {
        if pile.len() < 50 {
            eliminated.push(candidate.to_owned());
        }
    }
    let eliminated_piles = eliminated
        .iter()
        .map(|candidate| piles.remove(candidate).unwrap())
        .collect::<Vec<_>>();
    for pile in &eliminated_piles {
        for &ballot in pile {
            transfer_votes(&mut ballots[ballot], ballot, &mut piles, &mut exhausted);
        }
    }

    print_count(1, &piles, "Eliminated candidates with fewer than 50 first place votes");

    let mut rng = rand::thread_rng();

    let mut elected = Vec::new();

    // Transfer votes from candidates that made quota
    let mut count = 2;
    while elected.len() < 9 {
        let max_votes = piles.iter().map(|p| p.1.len()).max().unwrap();
        if max_votes >= quota || piles.len() + elected.len() == 9 {
            let mut first_place: Vec<_> = piles
                .iter()
                .filter(|p| p.1.len() == max_votes)
                .map(|p| p.0.to_string())
                .collect();
            first_place.shuffle(&mut rng);

            let elected_candidate = first_place.pop().unwrap();
            elected.push(elected_candidate.clone());

            let mut pile = piles.remove(&*elected_candidate).unwrap();
            pile.shuffle(&mut rng);

            let mut left_to_transfer = pile.len() - quota;
            while !pile.is_empty() && left_to_transfer > 0 {
                let ballot = pile.pop().unwrap();
                if transfer_votes(&mut ballots[ballot], ballot, &mut piles, &mut exhausted) {
                    left_to_transfer -= 1;
                }
            }

            print_count(count, &piles, &format!("Elected {elected_candidate}"));

            // let mut just_elected = Vec::new();
            // for (candidate, pile) in &piles {
            //     if pile.len() >= quota {
            //         just_elected.push(candidate.to_owned());
            //     }
            // }
            // let elected_piles = just_elected
            //     .iter()
            //     .map(|candidate| piles.remove(candidate).unwrap())
            //     .collect::<Vec<_>>();
            // elected.extend(just_elected);
            // println!("Elected: {:?}", elected);

            // for mut pile in elected_piles {
            //     pile.shuffle(&mut rng);
            //     for ballot in pile {
            //         transfer_votes(&mut ballots[ballot], ballot, &mut piles, &mut exhausted)
            //     }
            // }
        } else {
            // Eliminate last place candidate
            let min_votes = piles.iter().map(|p| p.1.len()).min().unwrap();
            let mut last_place: Vec<_> = piles
                .iter()
                .filter(|p| p.1.len() == min_votes)
                .map(|p| p.0.to_string())
                .collect();
            last_place.shuffle(&mut rng);

            let eliminated_candidate = last_place.pop().unwrap();
            for ballot in piles.remove(&*eliminated_candidate).unwrap() {
                transfer_votes(&mut ballots[ballot], ballot, &mut piles, &mut exhausted);
            }
            print_count(count, &piles, &format!("Eliminated {eliminated_candidate}"));
        }

        count += 1;
    }

    println!("Elected: {}", elected.join(", "));

    Ok(())
}
